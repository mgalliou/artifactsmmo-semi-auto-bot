use std::collections::HashMap;

use crate::{account::AccountController, character::CharacterController};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use sdk::{
    CanProvideXp, Code, CollectionClient, FROZEN_AXE, FROZEN_FISHING_ROD, FROZEN_GLOVES,
    FROZEN_PICKAXE, ItemsClient, Level, MAX_LEVEL,
    entities::{Character, Item, Monster, Resource},
    gear::{Gear, Slot},
    items::{
        ItemSource,
        Type::{self, Rune},
    },
    simulator::{FightParams, FightSimulation, HasEffects, Participant, time_to_rest},
    skill::Skill,
    yields_xp,
};

pub use artifact_set::ArtifactSet;
pub use component::{GearComponent, ItemSlot};
pub use filter::Filter;
pub use ring_set::RingSet;
use strum::IntoEnumIterator;
use strum_macros::EnumIs;
pub use utility_set::UtilitySet;

mod artifact_set;
mod component;
mod filter;
mod ring_set;
mod utility_set;

#[derive(Clone, EnumIs)]
pub enum GearPurpose {
    Combat(Monster),
    Crafting(Item),
    Gathering(Resource),
}

#[derive(Clone)]
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
        let owned_items = char
            .available_items()
            .keys()
            .filter_map(|code| self.items.get(code))
            .filter(Item::is_equipable)
            .collect_vec();
        let mut item_pool: Vec<Item> = if filter.available_only {
            vec![]
        } else {
            self.items
                .iter()
                .filter(|i| self.is_eligible(i, filter, char))
                .collect()
        };
        item_pool = [item_pool, owned_items].concat();
        item_pool.sort();
        item_pool.dedup();
        item_pool.retain(|i| char.meets_conditions_for(i));
        let resolver = GearResolver {
            purpose,
            level: char.level(),
            skill_levels: Skill::iter()
                .map(|skill| (skill, char.skill_level(skill)))
                .collect(),
            item_pool,
            available_items: char.available_items(),
            available_only: filter.available_only,
            use_utilities: filter.utilities,
        };
        resolver.resolve()
    }

    fn is_eligible(&self, item: &Item, filter: Filter, char: &CharacterController) -> bool {
        if !char.meets_conditions_for(item) {
            return false;
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

struct GearResolver {
    pub purpose: GearPurpose,
    pub level: u32,
    pub skill_levels: HashMap<Skill, u32>,
    pub item_pool: Vec<Item>,
    pub available_items: HashMap<String, u32>,
    pub available_only: bool,
    pub use_utilities: bool,
}

impl GearResolver {
    /// Resolve the best gear based on the internal properties:
    /// `level` is the combat level of the character
    /// `skill_levels` is the skill levels of the character
    /// `items` is a pre-filtered pool of item that the resolver will use
    /// `available` is the list of items available to the character with its quantity,
    /// items available are from inventory, bank, and current equipment
    /// `available_only` tell the resolver to only use the available items
    /// `use_utilities` tell the resolver to include utilities in the resolution
    ///
    /// When resolving gears with both `item_pool` and `available_only`, items from `available_items`
    /// are prioritized in case of a tie, and items from `item_pool` are considered of infinite quantity
    /// When `available_only` is set, items from `item_pool` are ignored
    fn resolve(&self) -> Option<Gear> {
        match &self.purpose {
            GearPurpose::Combat(monster) => self.best_to_kill(monster),
            GearPurpose::Crafting(item) => self.best_to_craft(item),
            GearPurpose::Gathering(resource) => self.best_to_gather(resource),
        }
    }

    /// Return the best gear to kill the given monster, if no gear allow the character to win the
    /// fight, returns None
    fn best_to_kill(&self, monster: &Monster) -> Option<Gear> {
        self.gen_combat_gears(monster)
            .filter_map(|g| {
                let sim = FightSimulation::new(
                    Participant::new("char1".into())
                        .with_level(self.level)
                        .with_gear(g.clone()),
                    monster.clone(),
                )
                .with_params(FightParams::averaged());
                let fight = sim.run();
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
            .max_set_by_key(|(_, g)| g.wisdom())
            .into_iter()
            .max_by_key(|(_, g)| Slot::iter().filter(|s| g.item_in(*s).is_some()).count())
            .map(|(_, g)| g)
    }

    /// Return the best gear to craft the given Item, if the character would not get XP from the
    /// craft no gear is returned
    fn best_to_craft(&self, item: &Item) -> Option<Gear> {
        let skill = item.skill_to_craft()?;
        if !yields_xp(self.skill_level(skill), item.level()) {
            return None;
        }
        self.gen_skill_gears(skill)
            .max_set_by_key(HasEffects::wisdom)
            .into_iter()
            .max_by_key(HasEffects::prospecting)
    }

    /// Return the best gear to gather the given resource, if the character would not get XP from the
    /// resource, wisdom is not taken into account
    fn best_to_gather(&self, resource: &Resource) -> Option<Gear> {
        self.gen_skill_gears(resource.skill())
            .max_set_by_key(HasEffects::prospecting)
            .into_iter()
            .max_by_key(HasEffects::wisdom)
    }

    fn gen_combat_gears(&self, monster: &Monster) -> impl Iterator<Item = Gear> {
        self.best_weapons(monster)
            .into_iter()
            .flat_map(|w| self.gen_combat_gears_with_weapon(monster, w))
    }

    pub fn best_weapons(&self, monster: &Monster) -> Vec<&Item> {
        self.item_pool
            .iter()
            .filter(|i| !i.is_tool())
            // sort by damage descending (negate), then alphabetically by code as tiebreaker
            .sorted_by_key(|&i| (OrderedFloat(-i.average_dmg_against(monster)), i.code()))
            .take(2)
            .collect_vec()
    }

    fn gen_combat_gears_with_weapon(
        &self,
        monster: &Monster,
        weapon: &Item,
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
            let armors = self.best_combat_armors(monster, weapon, item_type, &[]);
            (!armors.is_empty()).then(|| armors.iter().map(GearComponent::from).collect_vec())
        })
        .collect_vec();

        let ring_sets = self.gen_combat_ring_sets(monster, weapon);
        push_if_not_empty(&mut items, ring_sets);
        if self.use_utilities {
            let utilities_sets = self.gen_combat_utility_sets(monster, weapon);
            push_if_not_empty(&mut items, utilities_sets);
        }
        let artifact_sets = self.gen_combat_artifact_sets(monster, weapon);
        push_if_not_empty(&mut items, artifact_sets);
        let runes = self.best_combat_runes();
        push_if_not_empty(&mut items, runes);
        if let Some(bag) = self.best_bag() {
            items.push(vec![GearComponent::from(bag)]);
        }
        Self::gen_all_gears(Some(weapon), items)
    }

    fn gen_combat_ring_sets(&self, monster: &Monster, weapon: &Item) -> Vec<GearComponent> {
        self.gen_ring_sets_with(|unique| {
            self.best_combat_armors(monster, weapon, Type::Ring, unique)
        })
    }

    fn gen_combat_utility_sets(&self, monster: &Monster, weapon: &Item) -> Vec<GearComponent> {
        let utilities = self.best_combat_utilities(monster, weapon);
        gen_utility_sets(utilities)
    }

    fn gen_combat_artifact_sets(&self, monster: &Monster, weapon: &Item) -> Vec<GearComponent> {
        let artifacts = self.best_combat_armors(monster, weapon, Type::Artifact, &[]);
        gen_artifacts_sets(artifacts)
    }

    fn best_combat_armors(
        &self,
        monster: &Monster,
        weapon: &Item,
        r#type: Type,
        unique_items: &[Item],
    ) -> Vec<Item> {
        let mut bests: Vec<&Item> = vec![];
        let armors = self
            .item_pool
            .iter()
            .filter(|i| i.type_is(r#type) && !unique_items.contains(i))
            .cloned()
            .collect_vec();
        if let Some(best) =
            self.best_by_among(GearCriteria::DamageBoost { weapon, monster }, &armors)
        {
            bests.push(best);
        }
        if let Some(best) = self.best_by_among(GearCriteria::DamageReduction { monster }, &armors) {
            bests.push(best);
        }
        if r#type.is_artifact() {
            if let Some(best) = self.best_by_among(GearCriteria::Prospecting, &armors)
                && bests.iter().all(|u| u.prospecting() < best.prospecting())
            {
                bests.push(best);
            }
            if monster.provides_xp_at(self.level)
                && let Some(best) = self.best_by_among(GearCriteria::Wisdom, &armors)
                && bests.iter().all(|u| u.wisdom() < best.wisdom())
            {
                bests.push(best);
            }
        }
        if let Some(best) = self.best_by_among(GearCriteria::Health, &armors)
            && bests.iter().all(|u| u.health() < best.health())
        {
            bests.push(best);
        }
        bests.into_iter().sorted().dedup().cloned().collect()
    }

    fn best_combat_utilities(&self, monster: &Monster, weapon: &Item) -> Vec<Item> {
        let mut bests = vec![];
        let utilities = self
            .item_pool
            .iter()
            .filter(|i| i.type_is(Type::Utility))
            .cloned()
            .collect_vec();
        if let Some(best) =
            self.best_by_among(GearCriteria::DamageBoost { weapon, monster }, &utilities)
        {
            bests.push(best);
        }
        if let Some(best) =
            self.best_by_among(GearCriteria::DamageReduction { monster }, &utilities)
        {
            bests.push(best);
        }
        if let Some(best) = self.best_by_among(GearCriteria::Health, &utilities) {
            bests.push(best);
        }
        if let Some(best) = self.best_by_among(GearCriteria::Restore, &utilities) {
            bests.push(best);
        }
        bests.into_iter().sorted().dedup().cloned().collect_vec()
    }

    fn best_combat_runes(&self) -> Vec<GearComponent> {
        self.item_pool
            .iter()
            .filter(|i| i.type_is(Rune))
            .max_set_by_key(HasEffects::burn)
            .into_iter()
            .map(Into::into)
            .collect_vec()
    }

    fn gen_skill_gears(&self, skill: Skill) -> impl Iterator<Item = Gear> {
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
                let armors = self.best_skill_armors(item_type, skill, &[]);
                (!armors.is_empty()).then(|| armors.iter().map(GearComponent::from).collect())
            })
            .collect_vec();
        let ring_sets = self.gen_skill_rings_sets(skill);
        push_if_not_empty(&mut items, ring_sets);
        let artifact_sets = self.gen_skill_artifacts_sets(skill);
        push_if_not_empty(&mut items, artifact_sets);
        let tool = self.best_tool(skill);
        if let Some(bag) = self.best_bag() {
            items.push(vec![GearComponent::from(bag)]);
        }
        Self::gen_all_gears(tool, items)
    }

    fn best_tool(&self, skill: Skill) -> Option<&Item> {
        if !self.purpose.is_gathering() {
            return None;
        }
        self.item_pool
            .iter()
            .filter(|i| i.is_tool() && i.skill_cooldown_reduction(skill) != 0)
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn best_skill_armors(&self, r#type: Type, skill: Skill, unique_items: &[Item]) -> Vec<Item> {
        let mut bests: Vec<&Item> = vec![];
        let armors = self
            .item_pool
            .iter()
            .filter(|i| {
                i.type_is(r#type)
                    && !unique_items.contains(i)
                    && ((i.prospecting() > 0 && skill.is_gathering())
                        || (i.wisdom() > 0
                            && self.skill_level(skill) < MAX_LEVEL
                            && yields_xp(self.skill_level(skill), self.entity_level())))
            })
            .cloned()
            .collect_vec();
        if let Some(best) = self.best_by_among(GearCriteria::Prospecting, &armors)
            && bests.iter().all(|u| u.prospecting() < best.prospecting())
        {
            bests.push(best);
        }
        if let Some(best) = self.best_by_among(GearCriteria::Wisdom, &armors)
            && bests.iter().all(|u| u.wisdom() < best.wisdom())
        {
            bests.push(best);
        }
        bests.into_iter().sorted().dedup().cloned().collect_vec()
    }

    fn gen_skill_rings_sets(&self, skill: Skill) -> Vec<GearComponent> {
        self.gen_ring_sets_with(|unique| self.best_skill_armors(Type::Ring, skill, unique))
    }

    fn gen_skill_artifacts_sets(&self, skill: Skill) -> Vec<GearComponent> {
        let artifacts = self.best_skill_armors(Type::Artifact, skill, &[]);
        gen_artifacts_sets(artifacts)
    }

    fn best_bag(&self) -> Option<Item> {
        self.item_pool
            .iter()
            .filter(|i| i.type_is(Type::Bag))
            .cloned()
            .max_by_key(HasEffects::inventory_space)
    }

    fn gen_all_gears(
        weapon: Option<&Item>,
        items: Vec<Vec<GearComponent>>,
    ) -> impl Iterator<Item = Gear> {
        items
            .into_iter()
            .multi_cartesian_product()
            .filter_map(move |items| {
                Gear::new(
                    weapon.cloned(),
                    item_from_components(&items, Slot::Helmet),
                    item_from_components(&items, Slot::Shield),
                    item_from_components(&items, Slot::BodyArmor),
                    item_from_components(&items, Slot::LegArmor),
                    item_from_components(&items, Slot::Boots),
                    item_from_components(&items, Slot::Amulet),
                    item_from_components(&items, Slot::Ring1),
                    item_from_components(&items, Slot::Ring2),
                    item_from_components(&items, Slot::Utility1),
                    item_from_components(&items, Slot::Utility2),
                    item_from_components(&items, Slot::Artifact1),
                    item_from_components(&items, Slot::Artifact2),
                    item_from_components(&items, Slot::Artifact3),
                    item_from_components(&items, Slot::Rune),
                    item_from_components(&items, Slot::Bag),
                )
            })
    }

    fn gen_ring_sets_with(
        &self,
        fetch_armors: impl Fn(&[Item]) -> Vec<Item>,
    ) -> Vec<GearComponent> {
        let rings = fetch_armors(&[]);
        let single_rings = rings
            .iter()
            .filter(|i| {
                self.available_only && self.available_items.get(i.code()).is_some_and(|q| *q == 1)
            })
            .cloned()
            .collect_vec();
        let rings2 = fetch_armors(&single_rings);
        gen_ring_sets(rings, rings2)
    }

    fn best_by_among<'a>(&self, criteria: GearCriteria, armors: &'a [Item]) -> Option<&'a Item> {
        let armors = armors.iter().filter(|i| match criteria {
            GearCriteria::DamageBoost { weapon, monster } => {
                weapon.average_dmg_boost_against_with(monster, *i) > 0.0
            }
            GearCriteria::DamageReduction { monster } => {
                i.average_dmg_reduction_against(monster) > 0.0
            }
            GearCriteria::Prospecting => i.prospecting() > 0,
            GearCriteria::Wisdom => i.wisdom() > 0,
            GearCriteria::Health => i.health() > 0,
            GearCriteria::Restore => i.restore() > 0,
        });
        let armors = match criteria {
            GearCriteria::DamageBoost { weapon, monster } => armors.max_set_by_key(|i| {
                OrderedFloat(weapon.average_dmg_boost_against_with(monster, *i))
            }),
            GearCriteria::DamageReduction { monster } => {
                armors.max_set_by_key(|i| OrderedFloat(i.average_dmg_reduction_against(monster)))
            }
            GearCriteria::Prospecting => armors.max_set_by_key(HasEffects::prospecting),
            GearCriteria::Wisdom => armors.max_set_by_key(HasEffects::wisdom),
            GearCriteria::Health => armors.max_set_by_key(HasEffects::health),
            GearCriteria::Restore => armors.max_set_by_key(HasEffects::restore),
        };
        armors
            .into_iter()
            .max_by_key(|i| self.available_items.get(i.code()))
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        *self.skill_levels.get(&skill).unwrap_or(&1_u32)
    }

    fn entity_level(&self) -> u32 {
        match &self.purpose {
            GearPurpose::Combat(monster) => monster.level(),
            GearPurpose::Crafting(item) => item.level(),
            GearPurpose::Gathering(resource) => resource.level(),
        }
    }
}

fn push_if_not_empty(items: &mut Vec<Vec<GearComponent>>, set: Vec<GearComponent>) {
    if !set.is_empty() {
        items.push(set);
    }
}

fn item_from_components(components: &[GearComponent], slot: Slot) -> Option<Item> {
    components.iter().find_map(|w| {
        match w {
            GearComponent::Armor(armor) => armor.slot(),
            GearComponent::Rings(set) => set.slot(slot),
            GearComponent::Artifacts(set) => set.slot(slot),
            GearComponent::Utility(set) => set.slot(slot),
        }
        .and_then(|i| i.type_is(slot.into()).then(|| i.clone()))
    })
}

#[derive(Copy, Clone)]
enum GearCriteria<'a> {
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

/// Generate every single ring sets possible from two collections of rings, one for each slots
fn gen_ring_sets(rings1: Vec<Item>, rings2: Vec<Item>) -> Vec<GearComponent> {
    let mut rings1_slot = rings1.into_iter().map(Some).collect_vec();
    rings1_slot.push(None);
    let mut rings2_slot = rings2.into_iter().map(Some).collect_vec();
    rings2_slot.push(None);
    [rings1_slot, rings2_slot]
        .iter()
        .multi_cartesian_product()
        .filter_map(|rings| RingSet::new([rings[0].clone(), rings[1].clone()]))
        .sorted()
        .dedup()
        .map(GearComponent::Rings)
        .collect_vec()
}

fn gen_utility_sets(utilities: Vec<Item>) -> Vec<GearComponent> {
    let mut utilities = utilities.into_iter().map(Some).collect_vec();
    utilities.push(None);
    [utilities.clone(), utilities]
        .iter()
        .multi_cartesian_product()
        .filter_map(|utilities| UtilitySet::new([utilities[0].clone(), utilities[1].clone()]))
        .sorted()
        .dedup()
        .map(GearComponent::Utility)
        .collect_vec()
}

fn gen_artifacts_sets(artifacts: Vec<Item>) -> Vec<GearComponent> {
    let mut artifacts = artifacts.into_iter().map(Some).collect_vec();
    artifacts.push(None);
    [artifacts.clone(), artifacts.clone(), artifacts]
        .iter()
        .multi_cartesian_product()
        .filter_map(|artifacts| {
            ArtifactSet::new([
                artifacts[0].clone(),
                artifacts[1].clone(),
                artifacts[2].clone(),
            ])
        })
        .sorted()
        .dedup()
        .map(GearComponent::Artifacts)
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use sdk::{
        CollectionClient,
        test_utils::{ITEMS, item, monster},
    };

    #[test]
    fn resolver_best_weapons_against() {
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(monster("vampire")),
            level: 30,
            skill_levels: HashMap::new(),
            item_pool: ITEMS.iter().filter(|i| i.level() <= 30).collect(),
            available_items: HashMap::new(),
            available_only: false,
            use_utilities: false,
        };

        let weapons = resolver.best_weapons(&monster("vampire"));
        assert_eq!(
            weapons.first().unwrap().code(),
            item("greater_dreadful_staff").code(),
        );
    }

    #[test]
    fn resolve_best_gear_against_blue_slime() {
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(monster("blue_slime")),
            level: 10,
            skill_levels: HashMap::new(),
            item_pool: ITEMS.iter().filter(|i| i.level() <= 10).collect(),
            available_items: HashMap::new(),
            available_only: false,
            use_utilities: false,
        };
        let gear = resolver.resolve();
        assert_eq!(
            gear,
            Some(
                Gear::default()
                    .with_weapon(item("iron_sword"))
                    .with_helmet(item("adventurer_helmet"))
                    .with_shield(item("iron_shield"))
                    .with_body_armor(item("iron_armor"))
                    .with_leg_armor(item("iron_legs_armor"))
                    .with_boots(item("iron_boots"))
                    .with_amulet(item("fire_and_earth_amulet"))
                    .with_ring1(item("forest_ring"))
                    .with_ring2(item("forest_ring"))
                    .with_artifact1(item("novice_guide"))
                    .with_bag(item("backpack")),
            ),
        );
    }

    #[test]
    fn gen_ring_sets_no_duplicates() {
        let items = vec![item("copper_ring"), item("forest_ring")];
        let result = gen_ring_sets(items.clone(), items);
        let mut seen = HashSet::new();
        for wrapper in &result {
            let GearComponent::Rings(set) = wrapper else {
                panic!("expected Rings")
            };
            let key = (
                set.ring1().map_or("", Code::code),
                set.ring2().map_or("", Code::code),
            );
            assert!(seen.insert(key), "duplicate ring pair: {key:?}");
        }
    }

    #[test]
    fn gen_utility_sets_no_duplicates() {
        let items = vec![item("antidote"), item("health_potion")];
        let result = gen_utility_sets(items);
        let mut seen = HashSet::new();
        for wrapper in &result {
            let GearComponent::Utility(set) = wrapper else {
                panic!("expected Utility")
            };
            let key = (
                set.utility1().map_or("", Code::code),
                set.utility2().map_or("", Code::code),
            );
            assert!(seen.insert(key), "duplicate utility pair: {key:?}");
        }
    }

    #[test]
    fn gen_artifacts_sets_no_duplicates() {
        let items = vec![item("novice_guide"), item("life_crystal")];
        let result = gen_artifacts_sets(items);
        let mut seen = HashSet::new();
        for wrapper in &result {
            let GearComponent::Artifacts(set) = wrapper else {
                panic!("expected Artifacts")
            };
            let key = (
                set.artifact1().map_or("", Code::code),
                set.artifact2().map_or("", Code::code),
                set.artifact3().map_or("", Code::code),
            );
            assert!(seen.insert(key), "duplicate artifact triple: {key:?}");
        }
    }

    #[test]
    fn unique_ring_not_in_both_slots() {
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(monster("blue_slime")),
            level: 10,
            skill_levels: HashMap::new(),
            item_pool: ITEMS.iter().filter(|i| i.level() <= 10).collect_vec(),
            available_items: HashMap::from([("forest_ring".to_string(), 1)]),
            available_only: true,
            use_utilities: false,
        };
        let gear = resolver.resolve().unwrap();
        // With only 1 copy, forest_ring goes in one slot and a different ring in the other
        assert_eq!(gear.ring1, Some(item("forest_ring")));
        assert_eq!(gear.ring2, Some(item("iron_ring")));
    }

    #[test]
    fn duplicate_ring_with_two_copies() {
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(monster("blue_slime")),
            level: 10,
            skill_levels: HashMap::new(),
            item_pool: ITEMS.iter().filter(|i| i.level() <= 10).collect_vec(),
            available_items: HashMap::from([("forest_ring".to_string(), 2)]),
            available_only: true,
            use_utilities: false,
        };
        let gear = resolver.resolve().unwrap();
        // With 2 copies, forest_ring can fill both slots
        assert_eq!(gear.ring1, Some(item("forest_ring")));
        assert_eq!(gear.ring2, Some(item("forest_ring")));
    }

    #[test]
    fn prioritizes_available_items() {
        // lizard_skin_armor and stormforged_armor tie on DamageBoost against
        // vampire with dreadful_staff (both give 6.48). The tiebreaker in
        // best_armor_by should pick the one in available_items.
        let armors = vec![item("lizard_skin_armor"), item("stormforged_armor")];
        let weapon = item("dreadful_staff");
        let vamp = monster("vampire");
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(vamp.clone()),
            level: 25,
            skill_levels: HashMap::new(),
            item_pool: vec![],
            available_items: HashMap::from([("lizard_skin_armor".to_string(), 1)]),
            available_only: true,
            use_utilities: false,
        };
        let best = resolver.best_by_among(
            GearCriteria::DamageBoost {
                weapon: &weapon,
                monster: &vamp,
            },
            &armors,
        );
        assert_eq!(best.map(Item::code), Some("lizard_skin_armor"));

        // Reverse: put stormforged_armor in available_items instead
        let resolver2 = GearResolver {
            available_items: HashMap::from([("stormforged_armor".to_string(), 1)]),
            ..resolver
        };
        let best2 = resolver2.best_by_among(
            GearCriteria::DamageBoost {
                weapon: &weapon,
                monster: &vamp,
            },
            &armors,
        );
        assert_eq!(best2.map(Item::code), Some("stormforged_armor"));
    }

    #[test]
    fn resolve_best_gear_against_chicken() {
        let resolver = GearResolver {
            purpose: GearPurpose::Combat(monster("chicken")),
            level: 1,
            skill_levels: HashMap::new(),
            item_pool: ITEMS.iter().filter(|i| i.level() <= 1).collect(),
            available_items: HashMap::new(),
            available_only: false,
            use_utilities: false,
        };
        let gear = resolver.resolve();
        assert_eq!(
            gear,
            Some(
                Gear::default()
                    .with_weapon(item("wooden_staff"))
                    .with_shield(item("wooden_shield"))
                    .with_helmet(item("copper_helmet"))
                    .with_boots(item("copper_boots"))
                    .with_ring1(item("copper_ring"))
                    .with_ring2(item("copper_ring")),
            ),
        );
    }
}
