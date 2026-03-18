use crate::{
    account::AccountController, character::CharacterController, gear_finder::item_wrapper::item_cmp,
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use sdk::{
    CanProvideXp, Code, CollectionClient, FROZEN_AXE, FROZEN_FISHING_ROD, FROZEN_GLOVES,
    FROZEN_PICKAXE, ItemsClient, Level, MAX_LEVEL, check_lvl_diff,
    entities::{Character, Item, Monster, Resource},
    gear::{Gear, Slot},
    items::{ItemSource, Type},
    simulator::{FightParams, HasEffects, Participant, Simulator, time_to_rest},
    skill::Skill,
};

pub use artifact_set::ArtifactSet;
pub use filter::Filter;
pub use item_wrapper::ItemWrapper;
pub use ring_set::RingSet;
pub use utility_set::UtilitySet;

mod artifact_set;
mod filter;
mod item_wrapper;
mod ring_set;
mod utility_set;

#[derive(Clone, Copy)]
pub enum GearPurpose<'a> {
    Combat(&'a Monster),
    Crafting(&'a Item),
    Gathering(&'a Resource),
}

#[derive(Default, Clone)]
pub struct GearFinder {
    items: ItemsClient,
    account: AccountController,
}

impl GearFinder {
    pub const fn new(items: ItemsClient, account: AccountController) -> Self {
        Self { items, account }
    }

    pub fn best_for(
        &self,
        purpose: GearPurpose,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        match purpose {
            GearPurpose::Combat(monster) => self.best_to_kill(monster, char, filter),
            GearPurpose::Crafting(item) => self.best_to_craft(item, char, filter),
            GearPurpose::Gathering(resource) => self.best_to_gather(resource, char, filter),
        }
    }

    fn best_to_kill(
        &self,
        monster: &Monster,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        self.gen_combat_gears(char, monster, filter)
            .filter_map(|g| {
                let fight = Simulator::fight(
                    Participant::new(
                        char.name().to_string(),
                        char.level(),
                        g.clone(),
                        100,
                        100,
                        0,
                    ),
                    None,
                    monster,
                    &FightParams::default().averaged(),
                );
                fight.is_winning().then_some((fight, g))
            })
            .min_set_by_key(|(f, _)| f.cd + time_to_rest(f.hp_lost as u32))
            .into_iter()
            .min_set_by_key(|(f, _)| f.monster_hp)
            .into_iter()
            .max_set_by_key(|(f, _)| f.hp)
            .into_iter()
            .max_set_by_key(|(_, g)| g.prospecting())
            .into_iter()
            .max_by_key(|(_, g)| g.wisdom())
            .map(|(_, g)| g)
    }

    fn gen_combat_gears(
        &self,
        char: &CharacterController,
        monster: &Monster,
        filter: Filter,
    ) -> impl Iterator<Item = Gear> {
        self.best_weapons(char, monster, filter)
            .flat_map(move |w| self.gen_combat_gears_with_weapon(char, monster, filter, w))
    }

    pub fn best_weapons(
        &self,
        char: &CharacterController,
        monster: &Monster,
        filter: Filter,
    ) -> impl Iterator<Item = Item> {
        self.items
            .filtered(|i| !i.is_tool() && self.is_eligible(i, Type::Weapon, filter, char))
            .into_iter()
            .sorted_by_key(|i| OrderedFloat(i.average_dmg_against(monster)))
            .rev()
            .take(3)
    }

    fn gen_combat_gears_with_weapon(
        &self,
        char: &CharacterController,
        monster: &Monster,
        filter: Filter,
        weapon: Item,
    ) -> impl Iterator<Item = Gear> {
        let mut items = [
            Type::Helmet,
            Type::Shield,
            Type::BodyArmor,
            Type::LegArmor,
            Type::Boots,
            Type::Amulet,
        ]
        .iter()
        .filter_map(|&item_type| {
            let armors = self.best_combat_armors(char, monster, &weapon, item_type, filter, &[]);
            (!armors.is_empty()).then_some(
                armors
                    .iter()
                    .map(|i| ItemWrapper::Armor(i.clone()))
                    .collect_vec(),
            )
        })
        .collect_vec();

        let ring_sets = self.gen_combat_ring_sets(char, monster, &weapon, filter);
        if !ring_sets.is_empty() {
            items.push(ring_sets);
        }
        if filter.utilities {
            let utilities_sets = self.gen_combat_utility_sets(char, monster, &weapon, filter);
            if !utilities_sets.is_empty() {
                items.push(utilities_sets);
            }
        }
        let artifact_sets = self.gen_combat_artifact_sets(char, monster, &weapon, filter);
        if !artifact_sets.is_empty() {
            items.push(artifact_sets);
        }
        let runes = self.best_combat_runes(char, filter);
        if !runes.is_empty() {
            items.push(runes);
        }
        if let Some(bag) = self.best_bag(char, filter) {
            items.push(vec![ItemWrapper::from(bag)]);
        }
        Self::gen_all_gears(Some(weapon), items)
    }

    fn gen_combat_ring_sets(
        &self,
        char: &CharacterController,
        monster: &Monster,
        weapon: &Item,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let rings = self.best_combat_armors(char, monster, weapon, Type::Ring, filter, &[]);
        let unique_rings = rings
            .iter()
            .flatten()
            .filter(|i| filter.available_only && char.has_available(i.code()) == 1)
            .cloned()
            .collect_vec();
        let rings2 =
            self.best_combat_armors(char, monster, weapon, Type::Ring, filter, &unique_rings);
        gen_ring_sets(rings, rings2)
    }

    fn gen_combat_utility_sets(
        &self,
        char: &CharacterController,
        monster: &Monster,
        weapon: &Item,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let utilities = self.best_combat_utilities(char, monster, weapon, filter);
        gen_utility_sets(utilities)
    }

    fn gen_combat_artifact_sets(
        &self,
        char: &CharacterController,
        monster: &Monster,
        weapon: &Item,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let artifacts = self.best_combat_armors(char, monster, weapon, Type::Artifact, filter, &[]);
        gen_artifacts_sets(artifacts)
    }

    fn best_combat_armors(
        &self,
        char: &CharacterController,
        monster: &Monster,
        weapon: &Item,
        r#type: Type,
        filter: Filter,
        unique_items: &[Item],
    ) -> Vec<Option<Item>> {
        let mut bests: Vec<Item> = vec![];
        let armors = self
            .items
            .filtered(|i| !unique_items.contains(i) && self.is_eligible(i, r#type, filter, char));
        if let Some(best) = best_armor_by(
            ArmorCriteria::DamageBoost { weapon, monster },
            &armors,
            char,
        ) {
            bests.push(best.clone());
        }
        if let Some(best) = best_armor_by(ArmorCriteria::DamageReduction { monster }, &armors, char)
        {
            bests.push(best.clone());
        }
        if r#type.is_artifact() {
            if let Some(best) = best_armor_by(ArmorCriteria::Prospecting, &armors, char)
                && bests.iter().all(|u| u.prospecting() < best.prospecting())
            {
                bests.push(best.clone());
            }
            if monster.provides_xp_at(char.level())
                && let Some(best) = best_armor_by(ArmorCriteria::Wisdom, &armors, char)
                && bests.iter().all(|u| u.wisdom() < best.wisdom())
            {
                bests.push(best.clone());
            }
        }
        if let Some(best) = best_armor_by(ArmorCriteria::Health, &armors, char)
            && bests.iter().all(|u| u.health() < best.health())
        {
            bests.push(best.clone());
        }
        bests
            .into_iter()
            .map(Some)
            .sorted_by(|a, b| item_cmp(a.as_ref(), b.as_ref()))
            .dedup()
            .collect_vec()
    }

    fn best_combat_utilities(
        &self,
        char: &CharacterController,
        monster: &Monster,
        weapon: &Item,
        filter: Filter,
    ) -> Vec<Option<Item>> {
        let mut bests: Vec<Item> = vec![];
        let utilities = self
            .items
            .filtered(|i| self.is_eligible(i, Type::Utility, filter, char));
        if let Some(best) = best_armor_by(
            ArmorCriteria::DamageBoost { weapon, monster },
            &utilities,
            char,
        ) {
            bests.push(best.clone());
        }
        if let Some(best) =
            best_armor_by(ArmorCriteria::DamageReduction { monster }, &utilities, char)
        {
            bests.push(best.clone());
        }
        if let Some(best) = best_armor_by(ArmorCriteria::Health, &utilities, char) {
            bests.push(best.clone());
        }
        if let Some(best) = best_armor_by(ArmorCriteria::Restore, &utilities, char) {
            bests.push(best.clone());
        }
        bests
            .into_iter()
            .map(Some)
            .sorted_by(|a, b| item_cmp(a.as_ref(), b.as_ref()))
            .dedup()
            .collect_vec()
    }

    fn best_combat_runes(&self, char: &CharacterController, filter: Filter) -> Vec<ItemWrapper> {
        self.items
            .filtered(|i| self.is_eligible(i, Type::Rune, filter, char))
            .iter()
            .max_set_by_key(HasEffects::burn)
            .into_iter()
            .map(Into::into)
            .collect_vec()
    }

    fn best_to_craft(
        &self,
        item: &Item,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        let skill = item.skill_to_craft()?;
        if !check_lvl_diff(char.skill_level(skill), item.level()) {
            return None;
        }
        self.gen_skill_gears(char, skill, item.level(), filter, false)
            .max_set_by_key(HasEffects::wisdom)
            .into_iter()
            .max_by_key(HasEffects::prospecting)
    }

    fn best_to_gather(
        &self,
        resource: &Resource,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        let skill = resource.skill();
        let level = resource.level();
        self.gen_skill_gears(char, skill, level, filter, true)
            .max_set_by_key(HasEffects::prospecting)
            .into_iter()
            .max_by_key(HasEffects::wisdom)
    }

    fn gen_skill_gears(
        &self,
        char: &CharacterController,
        skill: Skill,
        skill_level: u32,
        filter: Filter,
        with_tool: bool,
    ) -> impl Iterator<Item = Gear> {
        let armor_types = [
            Type::Helmet,
            Type::Shield,
            Type::BodyArmor,
            Type::LegArmor,
            Type::Boots,
            Type::Amulet,
        ];
        let mut items = armor_types
            .iter()
            .filter_map(|&item_type| {
                let armors =
                    self.best_skill_armors(char, skill, skill_level, item_type, filter, &[]);
                (!armors.is_empty()).then_some(
                    armors
                        .iter()
                        .map(|i| ItemWrapper::Armor(i.clone()))
                        .collect_vec(),
                )
            })
            .collect_vec();
        let ring_sets = self.gen_skill_rings_sets(char, skill, skill_level, filter);
        if !ring_sets.is_empty() {
            items.push(ring_sets);
        }
        let artifact_sets = self.gen_skill_artifacts_set(char, skill, skill_level, filter);
        if !artifact_sets.is_empty() {
            items.push(artifact_sets);
        }
        let tool = with_tool
            .then_some(self.best_tool(char, skill, filter))
            .flatten();
        if let Some(bag) = self.best_bag(char, filter) {
            items.push(vec![ItemWrapper::from(&bag)]);
        }
        Self::gen_all_gears(tool, items)
    }

    fn best_tool(&self, char: &CharacterController, skill: Skill, filter: Filter) -> Option<Item> {
        self.items
            .filtered(|i| i.is_tool() && self.is_eligible(i, Type::Weapon, filter, char))
            .into_iter()
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn best_skill_armors(
        &self,
        char: &CharacterController,
        skill: Skill,
        skill_level: u32,
        r#type: Type,
        filter: Filter,
        unique_items: &[Item],
    ) -> Vec<Option<Item>> {
        let mut bests: Vec<Item> = vec![];
        let armors = self.items.filtered(|i| {
            !unique_items.contains(i)
                && self.is_eligible(i, r#type, filter, char)
                && ((i.prospecting() > 0 && skill.is_gathering())
                    || (i.wisdom() > 0
                        && char.skill_level(skill) < MAX_LEVEL
                        && check_lvl_diff(char.skill_level(skill), skill_level)))
        });
        if let Some(best) = best_armor_by(ArmorCriteria::Prospecting, &armors, char)
            && bests.iter().all(|u| u.prospecting() < best.prospecting())
        {
            bests.push(best.clone());
        }
        if let Some(best) = best_armor_by(ArmorCriteria::Wisdom, &armors, char)
            && bests.iter().all(|u| u.wisdom() < best.wisdom())
        {
            bests.push(best.clone());
        }
        bests
            .into_iter()
            .map(Some)
            .sorted_by(|a, b| item_cmp(a.as_ref(), b.as_ref()))
            .dedup()
            .collect_vec()
    }

    fn gen_skill_rings_sets(
        &self,
        char: &CharacterController,
        skill: Skill,
        skill_level: u32,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let rings = self.best_skill_armors(char, skill, skill_level, Type::Ring, filter, &[]);
        let single_rings = rings
            .iter()
            .flatten()
            .filter(|i| filter.available_only && char.has_available(i.code()) <= 1)
            .cloned()
            .collect_vec();
        let rings2 =
            self.best_skill_armors(char, skill, skill_level, Type::Ring, filter, &single_rings);
        gen_ring_sets(rings, rings2)
    }

    fn gen_skill_artifacts_set(
        &self,
        char: &CharacterController,
        skill: Skill,
        skill_level: u32,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let artifacts =
            self.best_skill_armors(char, skill, skill_level, Type::Artifact, filter, &[]);
        gen_artifacts_sets(artifacts)
    }

    fn gen_all_gears(
        weapon: Option<Item>,
        items: Vec<Vec<ItemWrapper>>,
    ) -> impl Iterator<Item = Gear> {
        items
            .into_iter()
            .multi_cartesian_product()
            .filter_map(move |items| {
                Gear::new(
                    weapon.clone(),
                    Self::item_from_wrappers(&items, Slot::Helmet),
                    Self::item_from_wrappers(&items, Slot::Shield),
                    Self::item_from_wrappers(&items, Slot::BodyArmor),
                    Self::item_from_wrappers(&items, Slot::LegArmor),
                    Self::item_from_wrappers(&items, Slot::Boots),
                    Self::item_from_wrappers(&items, Slot::Amulet),
                    Self::item_from_wrappers(&items, Slot::Ring1),
                    Self::item_from_wrappers(&items, Slot::Ring2),
                    Self::item_from_wrappers(&items, Slot::Utility1),
                    Self::item_from_wrappers(&items, Slot::Utility2),
                    Self::item_from_wrappers(&items, Slot::Artifact1),
                    Self::item_from_wrappers(&items, Slot::Artifact2),
                    Self::item_from_wrappers(&items, Slot::Artifact3),
                    Self::item_from_wrappers(&items, Slot::Rune),
                    Self::item_from_wrappers(&items, Slot::Bag),
                )
            })
    }

    fn best_bag(&self, char: &CharacterController, filter: Filter) -> Option<Item> {
        let bags = self
            .items
            .filtered(|i| self.is_eligible(i, Type::Bag, filter, char));
        bags.into_iter().max_by_key(HasEffects::inventory_space)
    }

    fn item_from_wrappers(wrappers: &[ItemWrapper], slot: Slot) -> Option<Item> {
        wrappers.iter().find_map(|w| {
            match w {
                ItemWrapper::Armor(armor) => armor.as_ref(),
                ItemWrapper::Rings(set) => set.slot(slot),
                ItemWrapper::Artifacts(set) => set.slot(slot),
                ItemWrapper::Utility(set) => set.slot(slot),
            }
            .and_then(|i| i.type_is(slot.into()).then_some(i.clone()))
        })
    }

    fn is_eligible(
        &self,
        item: &Item,
        r#type: Type,
        filter: Filter,
        char: &CharacterController,
    ) -> bool {
        if !item.type_is(r#type) {
            return false;
        }
        if !char.meets_conditions_for(item) {
            return false;
        }
        let total_available = char.has_available(item.code());
        if filter.available_only && total_available < 1 {
            return false;
        }
        if item.r#type().is_ring() && total_available > 1
            || !item.r#type().is_ring() && total_available > 0
        {
            return true;
        }
        if [
            "steel_gloves",
            "leather_gloves",
            "conjurer_cloak",
            "stormforged_armor",
            "stormforged_pants",
            "lizard_skin_legs_armor",
            "life_crystal",
            "cursed_sceptre",
            "cursed_hat",
            "sanguine_edge_of_rosen",
            "dreadful_battleaxe",
            "diamond_sword",
            "diamond_amulet",
            "ancestral_talisman",
            "corrupted_skull",
            "malefic_crystal",
            "malefic_ring",
            "sapphire_book",
            "ruby_book",
            "emerald_book",
            "topaz_book",
            "backpack",
            "satchel",
            "iron_pickaxe",
            "iron_axe",
            FROZEN_FISHING_ROD,
            FROZEN_AXE,
            FROZEN_GLOVES,
            FROZEN_PICKAXE,
        ]
        .contains(&item.code())
        {
            return false;
        }
        if filter.craftable && item.is_craftable() && !self.account.can_craft(item.code()) {
            return false;
        }
        if !filter.from_npc && self.items.is_buyable(item.code()) {
            return false;
        }
        if !filter.from_task && item.is_crafted_from_task() {
            return false;
        }
        if !filter.from_monster
            && self
                .items
                .sources_of(item.code())
                .first()
                .is_some_and(ItemSource::is_monster)
        {
            return false;
        }
        true
    }
}

fn best_armor_by<'a>(
    criteria: ArmorCriteria,
    armors: &'a [Item],
    char: &CharacterController,
) -> Option<&'a Item> {
    let armors = armors.iter().filter(|i| match criteria {
        ArmorCriteria::DamageBoost { weapon, monster } => {
            weapon.average_dmg_boost_against_with(monster, *i) > 0.0
        }
        ArmorCriteria::DamageReduction { monster } => {
            i.average_dmg_reduction_against(monster) > 0.0
        }
        ArmorCriteria::Prospecting => i.prospecting() > 0,
        ArmorCriteria::Wisdom => i.wisdom() > 0,
        ArmorCriteria::Health => i.health() > 0,
        ArmorCriteria::Restore => i.restore() > 0,
    });
    let armors = match criteria {
        ArmorCriteria::DamageBoost { weapon, monster } => armors
            .max_set_by_key(|i| OrderedFloat(weapon.average_dmg_boost_against_with(monster, *i))),
        ArmorCriteria::DamageReduction { monster } => {
            armors.max_set_by_key(|i| OrderedFloat(i.average_dmg_reduction_against(monster)))
        }
        ArmorCriteria::Prospecting => armors.max_set_by_key(HasEffects::prospecting),
        ArmorCriteria::Wisdom => armors.max_set_by_key(HasEffects::wisdom),
        ArmorCriteria::Health => armors.max_set_by_key(HasEffects::health),
        ArmorCriteria::Restore => armors.max_set_by_key(HasEffects::restore),
    };
    armors
        .into_iter()
        .max_by_key(|i| char.has_available(i.code()))
}

#[derive(Copy, Clone)]
enum ArmorCriteria<'a> {
    DamageBoost {
        weapon: &'a Item,
        monster: &'a Monster,
    },
    DamageReduction {
        monster: &'a Monster,
    },
    Health,
    Restore,
    Prospecting,
    Wisdom,
}

fn gen_ring_sets(mut rings1: Vec<Option<Item>>, mut rings2: Vec<Option<Item>>) -> Vec<ItemWrapper> {
    if !rings1.contains(&None) {
        rings1.push(None);
    }
    if !rings2.contains(&None) {
        rings2.push(None);
    }
    [rings1, rings2]
        .iter()
        .multi_cartesian_product()
        .map(|rings| [rings[0].clone(), rings[1].clone()])
        .filter_map(RingSet::new)
        .sorted()
        .dedup()
        .map(ItemWrapper::from)
        .collect_vec()
}

fn gen_utility_sets(mut utilities: Vec<Option<Item>>) -> Vec<ItemWrapper> {
    if !utilities.contains(&None) {
        utilities.push(None);
    }
    [utilities.clone(), utilities]
        .iter()
        .multi_cartesian_product()
        .map(|utilities| [utilities[0].clone(), utilities[1].clone()])
        .filter_map(UtilitySet::new)
        .sorted()
        .dedup()
        .map(ItemWrapper::Utility)
        .collect_vec()
}

fn gen_artifacts_sets(mut artifacts: Vec<Option<Item>>) -> Vec<ItemWrapper> {
    if !artifacts.contains(&None) {
        artifacts.push(None);
    }
    [artifacts.clone(), artifacts.clone(), artifacts]
        .iter()
        .multi_cartesian_product()
        .map(|artifacts| {
            [
                artifacts[0].clone(),
                artifacts[1].clone(),
                artifacts[2].clone(),
            ]
        })
        .filter_map(ArtifactSet::new)
        .sorted()
        .dedup()
        .map(ItemWrapper::Artifacts)
        .collect_vec()
}

#[cfg(test)]
mod tests {
    // use sdk::{MonstersClient, models::CharacterSchema};
    //
    // use super::*;

    // #[test]
    // fn best_weapons_against() {
    //     let gear_finder = GearFinder::default();
    //     let char = CharacterController::default();
    //     let data = CharacterSchema {
    //         level: 30,
    //         ..Default::default()
    //     };
    //     char.update_data(data);
    //
    //     let weapons = gear_finder
    //         .best_weapons(
    //             &char,
    //             &MonstersClient::default().get("vampire").unwrap(),
    //             Default::default(),
    //         )
    //         .collect_vec();
    //     assert_eq!(
    //         weapons,
    //         vec![ItemsClient::default().get("death_knight_sword").unwrap()]
    //     );
    // }
}
