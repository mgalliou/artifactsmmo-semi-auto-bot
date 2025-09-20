use crate::{account::AccountController, character::CharacterController};
use artifactsmmo_sdk::{
    character::HasCharacterData, check_lvl_diff, gear::{Gear, Slot}, items::{ItemSchemaExt, Type}, models::{ItemSchema, MonsterSchema, ResourceSchema}, simulator::HasEffects, skill::Skill, CollectionClient, FightParams, ItemsClient, Level, Simulator, FROZEN_AXE, FROZEN_FISHING_ROD, FROZEN_GLOVES, FROZEN_PICKAXE
};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use std::{cmp::Ordering, sync::Arc};

#[derive(Default)]
pub struct GearFinder {
    items: Arc<ItemsClient>,
    account: Arc<AccountController>,
}

impl GearFinder {
    pub fn new(items: Arc<ItemsClient>, account: Arc<AccountController>) -> Self {
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
        monster: &MonsterSchema,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        self.gen_combat_gears(char, monster, filter)
            .filter_map(|g| {
                let fight =
                    Simulator::fight(char.level(), &g, monster, FightParams::default().average());
                fight.is_winning().then_some((fight, g))
            })
            .min_set_by_key(|(f, _)| f.cd + Simulator::time_to_rest(f.hp_lost as u32))
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
        monster: &MonsterSchema,
        filter: Filter,
    ) -> impl Iterator<Item = Gear> {
        self.best_weapons(char, monster, filter)
            .flat_map(move |w| self.gen_combat_gears_with_weapon(char, monster, filter, w))
    }

    pub fn best_weapons(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        filter: Filter,
    ) -> impl Iterator<Item = Arc<ItemSchema>> {
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
        monster: &MonsterSchema,
        filter: Filter,
        weapon: Arc<ItemSchema>,
    ) -> impl IntoIterator<Item = Gear> {
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
            items.push(runes)
        }
        if let Some(bag) = self.best_bag(char, filter) {
            items.push(vec![ItemWrapper::Armor(Some(bag))]);
        }
        self.gen_all_gears(Some(weapon.clone()), &items)
    }

    fn gen_combat_ring_sets(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let rings = self.best_combat_armors(char, monster, weapon, Type::Ring, filter, &[]);
        let single_rings = rings
            .iter()
            .flatten()
            .filter(|i| filter.available_only && char.has_available(&i.code) <= 1)
            .cloned()
            .collect_vec();
        let rings2 =
            self.best_combat_armors(char, monster, weapon, Type::Ring, filter, &single_rings);
        gen_ring_sets(rings, rings2)
    }

    fn gen_combat_utility_sets(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let utilities = self.best_combat_utilities(char, monster, weapon, filter);
        gen_utility_sets(utilities)
    }

    fn gen_combat_artifact_sets(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<ItemWrapper> {
        let artifacts = self.best_combat_armors(char, monster, weapon, Type::Artifact, filter, &[]);
        gen_artifacts_sets(artifacts)
    }

    fn best_combat_armors(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        r#type: Type,
        filter: Filter,
        unique_items: &[Arc<ItemSchema>],
    ) -> Vec<Option<Arc<ItemSchema>>> {
        let mut bests: Vec<Arc<ItemSchema>> = vec![];
        let armors = self
            .items
            .filtered(|i| !unique_items.contains(i) && self.is_eligible(i, r#type, filter, char));
        let best_for_damage = best_armor_by(
            ArmorCriteria::DamageBoost { weapon, monster },
            &armors,
            char,
        );
        let best_reduction =
            best_armor_by(ArmorCriteria::DamageReduction { monster }, &armors, char);
        let best_health_increase = best_armor_by(ArmorCriteria::Health, &armors, char);
        if let Some(best_for_damage) = best_for_damage {
            bests.push(best_for_damage.clone());
        }
        if let Some(best_reduction) = best_reduction {
            bests.push(best_reduction.clone());
        }
        if r#type.is_artifact() {
            let best_wisdom = best_armor_by(ArmorCriteria::Wisdom, &armors, char);
            let best_prospecting = best_armor_by(ArmorCriteria::Prospecting, &armors, char);
            if let Some(best_wisdom) = best_wisdom
                && bests.iter().all(|u| u.wisdom() < best_wisdom.wisdom())
            {
                bests.push(best_wisdom.clone());
            }
            if let Some(best_prospecting) = best_prospecting
                && bests
                    .iter()
                    .all(|u| u.prospecting() < best_prospecting.prospecting())
            {
                bests.push(best_prospecting.clone());
            }
        }
        if let Some(best_health_increase) = best_health_increase
            && bests
                .iter()
                .all(|u| u.health() < best_health_increase.health())
        {
            bests.push(best_health_increase.clone());
        }
        bests
            .into_iter()
            .map(Some)
            .sorted_by(item_cmp)
            .dedup()
            .collect_vec()
    }

    fn best_combat_utilities(
        &self,
        char: &CharacterController,
        monster: &MonsterSchema,
        weapon: &ItemSchema,
        filter: Filter,
    ) -> Vec<Option<Arc<ItemSchema>>> {
        let mut bests: Vec<Arc<ItemSchema>> = vec![];
        let utilities = self
            .items
            .filtered(|i| self.is_eligible(i, Type::Utility, filter, char));
        let best_damage_boost = best_armor_by(
            ArmorCriteria::DamageBoost { weapon, monster },
            &utilities,
            char,
        );
        let best_reduction =
            best_armor_by(ArmorCriteria::DamageReduction { monster }, &utilities, char);
        let best_health_increase = best_armor_by(ArmorCriteria::Health, &utilities, char);
        let best_restore = best_armor_by(ArmorCriteria::Restore, &utilities, char);
        if let Some(best_for_damage) = best_damage_boost {
            bests.push(best_for_damage.clone());
        }
        if let Some(best_reduction) = best_reduction {
            bests.push(best_reduction.clone());
        }
        if let Some(best_health_increase) = best_health_increase {
            bests.push(best_health_increase.clone());
        }
        if let Some(best_restore) = best_restore {
            bests.push(best_restore.clone());
        }
        bests
            .into_iter()
            .map(Some)
            .sorted_by(item_cmp)
            .dedup()
            .collect_vec()
    }

    fn best_combat_runes(&self, char: &CharacterController, filter: Filter) -> Vec<ItemWrapper> {
        self.items
            .filtered(|i| self.is_eligible(i, Type::Rune, filter, char))
            .iter()
            .max_set_by_key(|i| i.burn())
            .into_iter()
            .map(|i| ItemWrapper::Armor(Some(i.clone())))
            .collect_vec()
    }

    fn best_to_craft(
        &self,
        item: &ItemSchema,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        let skill = item.skill_to_craft()?;
        if !check_lvl_diff(char.skill_level(skill), item.level()) {
            return None;
        }
        self.gen_skill_gears(char, skill, item.level(), filter, false)
            .into_iter()
            .max_set_by_key(|g| g.wisdom())
            .into_iter()
            .max_by_key(|g| g.prospecting())
    }

    fn best_to_gather(
        &self,
        resource: &ResourceSchema,
        char: &CharacterController,
        filter: Filter,
    ) -> Option<Gear> {
        let skill = resource.skill;
        let level = resource.level();
        self.gen_skill_gears(char, skill.into(), level, filter, true)
            .into_iter()
            .max_set_by_key(|g| g.prospecting())
            .into_iter()
            .max_by_key(|g| g.wisdom())
    }

    fn gen_skill_gears(
        &self,
        char: &CharacterController,
        skill: Skill,
        skill_level: u32,
        filter: Filter,
        with_tool: bool,
    ) -> Vec<Gear> {
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
                    self.best_skill_armors(char, skill, skill_level, item_type, filter, vec![]);
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
            items.push(vec![ItemWrapper::Armor(Some(bag))]);
        }
        self.gen_all_gears(tool, &items)
    }

    fn best_tool(
        &self,
        char: &CharacterController,
        skill: Skill,
        filter: Filter,
    ) -> Option<Arc<ItemSchema>> {
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
        unique_items: Vec<Arc<ItemSchema>>,
    ) -> Vec<Option<Arc<ItemSchema>>> {
        let mut bests: Vec<Arc<ItemSchema>> = vec![];
        let armors = self.items.filtered(|i| {
            !unique_items.contains(i)
                && self.is_eligible(i, r#type, filter, char)
                && ((i.wisdom() > 0 && check_lvl_diff(char.skill_level(skill), skill_level))
                    || (i.prospecting() > 0 && skill.is_gathering()))
        });
        let best_for_wisdom = best_armor_by(ArmorCriteria::Wisdom, &armors, char);

        let best_for_prospecting = best_armor_by(ArmorCriteria::Prospecting, &armors, char);
        if let Some(best_for_wisdom) = best_for_wisdom {
            bests.push(best_for_wisdom.clone());
        }
        if let Some(best_for_prospecting) = best_for_prospecting {
            bests.push(best_for_prospecting.clone());
        }
        bests
            .iter()
            .map(|i| Some(i.clone()))
            .sorted_by(item_cmp)
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
        let rings = self.best_skill_armors(char, skill, skill_level, Type::Ring, filter, vec![]);
        let single_rings = rings
            .iter()
            .flatten()
            .filter(|i| filter.available_only && char.has_available(&i.code) <= 1)
            .cloned()
            .collect_vec();
        let rings2 =
            self.best_skill_armors(char, skill, skill_level, Type::Ring, filter, single_rings);
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
            self.best_skill_armors(char, skill, skill_level, Type::Artifact, filter, vec![]);
        gen_artifacts_sets(artifacts)
    }

    fn gen_all_gears(
        &self,
        weapon: Option<Arc<ItemSchema>>,
        items: &[Vec<ItemWrapper>],
    ) -> Vec<Gear> {
        items
            .iter()
            .multi_cartesian_product()
            .filter_map(|items| {
                Gear::new(
                    weapon.clone(),
                    self.item_from_wrappers(&items, Slot::Helmet),
                    self.item_from_wrappers(&items, Slot::Shield),
                    self.item_from_wrappers(&items, Slot::BodyArmor),
                    self.item_from_wrappers(&items, Slot::LegArmor),
                    self.item_from_wrappers(&items, Slot::Boots),
                    self.item_from_wrappers(&items, Slot::Amulet),
                    self.item_from_wrappers(&items, Slot::Ring1),
                    self.item_from_wrappers(&items, Slot::Ring2),
                    self.item_from_wrappers(&items, Slot::Utility1),
                    self.item_from_wrappers(&items, Slot::Utility2),
                    self.item_from_wrappers(&items, Slot::Artifact1),
                    self.item_from_wrappers(&items, Slot::Artifact2),
                    self.item_from_wrappers(&items, Slot::Artifact3),
                    self.item_from_wrappers(&items, Slot::Rune),
                    self.item_from_wrappers(&items, Slot::Bag),
                )
            })
            .collect_vec()
    }

    fn best_bag(&self, char: &CharacterController, filter: Filter) -> Option<Arc<ItemSchema>> {
        let bags = self
            .items
            .filtered(|i| self.is_eligible(i, Type::Bag, filter, char));
        bags.into_iter().max_by_key(|i| i.inventory_space())
    }

    fn item_from_wrappers(&self, wrappers: &[&ItemWrapper], slot: Slot) -> Option<Arc<ItemSchema>> {
        wrappers.iter().find_map(|w| {
            match w {
                ItemWrapper::Armor(armor) => armor,
                ItemWrapper::Rings(set) => set.slot(slot),
                ItemWrapper::Artifacts(set) => set.slot(slot),
                ItemWrapper::Utility(set) => set.slot(slot),
            }
            .as_ref()
            .and_then(|i| i.type_is(slot.into()).then_some(i.clone()))
        })
    }

    fn is_eligible(
        &self,
        item: &ItemSchema,
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
        let total_available = char.has_available(&item.code);
        if filter.available_only && total_available < 1 {
            return false;
        }
        if total_available > 0 {
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
            "corrupted_skull",
            "malefic_crystal",
            "sapphire_book",
            "ruby_book",
            "emerald_book",
            "topaz_book",
            "backpack",
            FROZEN_FISHING_ROD,
            FROZEN_AXE,
            FROZEN_GLOVES,
            FROZEN_PICKAXE,
        ]
        .contains(&item.code.as_str())
        {
            return false;
        }
        if filter.craftable && item.is_craftable() && !self.account.can_craft(&item.code) {
            return false;
        }
        if !filter.from_npc && self.items.is_buyable(&item.code) {
            return false;
        }
        if !filter.from_task && item.is_crafted_from_task() {
            return false;
        }
        if !filter.from_monster
            && self
                .items
                .best_source_of(&item.code)
                .is_some_and(|s| s.is_monster())
        {
            return false;
        }
        true
    }
}

fn best_armor_by<'a>(
    criteria: ArmorCriteria,
    armors: &'a [Arc<ItemSchema>],
    char: &CharacterController,
) -> Option<&'a Arc<ItemSchema>> {
    let armors = armors.iter().filter(|i| match criteria {
        ArmorCriteria::DamageBoost { weapon, monster } => {
            weapon.average_dmg_boost_against_with(i.as_ref(), monster) > 0.0
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
        ArmorCriteria::DamageBoost { weapon, monster } => armors.max_set_by_key(|i| {
            OrderedFloat(weapon.average_dmg_boost_against_with(i.as_ref(), monster))
        }),
        ArmorCriteria::DamageReduction { monster } => {
            armors.max_set_by_key(|i| OrderedFloat(i.average_dmg_reduction_against(monster)))
        }
        ArmorCriteria::Prospecting => armors.max_set_by_key(|i| i.prospecting()),
        ArmorCriteria::Wisdom => armors.max_set_by_key(|i| i.wisdom()),
        ArmorCriteria::Health => armors.max_set_by_key(|i| i.health()),
        ArmorCriteria::Restore => armors.max_set_by_key(|i| i.restore()),
    };
    armors
        .into_iter()
        .max_by_key(|i| char.has_available(&i.code))
}

enum ArmorCriteria<'a> {
    DamageBoost {
        weapon: &'a ItemSchema,
        monster: &'a MonsterSchema,
    },
    DamageReduction {
        monster: &'a MonsterSchema,
    },
    Health,
    Restore,
    Prospecting,
    Wisdom,
}

fn gen_ring_sets(
    mut rings1: Vec<Option<Arc<ItemSchema>>>,
    mut rings2: Vec<Option<Arc<ItemSchema>>>,
) -> Vec<ItemWrapper> {
    if !rings1.contains(&None) {
        rings1.push(None)
    }
    if !rings2.contains(&None) {
        rings2.push(None)
    }
    [rings1, rings2]
        .iter()
        .multi_cartesian_product()
        .map(|rings| [rings[0].clone(), rings[1].clone()])
        .filter_map(RingSet::new)
        .sorted_by(|a, b| {
            if a == b {
                Ordering::Equal
            } else {
                match item_cmp(a.ring1(), b.ring1()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => item_cmp(a.ring2(), b.ring2()),
                    Ordering::Greater => Ordering::Greater,
                }
            }
        })
        .dedup()
        .map(ItemWrapper::Rings)
        .collect_vec()
}

fn gen_utility_sets(mut utilities: Vec<Option<Arc<ItemSchema>>>) -> Vec<ItemWrapper> {
    if !utilities.contains(&None) {
        utilities.push(None)
    }
    [utilities.clone(), utilities]
        .iter()
        .multi_cartesian_product()
        .map(|utilities| [utilities[0].clone(), utilities[1].clone()])
        .filter_map(UtilitySet::new)
        .sorted_by(|a, b| {
            if a == b {
                Ordering::Equal
            } else {
                match item_cmp(a.utility1(), b.utility1()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => item_cmp(a.utility2(), b.utility2()),
                    Ordering::Greater => Ordering::Greater,
                }
            }
        })
        .dedup()
        .map(ItemWrapper::Utility)
        .collect_vec()
}

fn gen_artifacts_sets(mut artifacts: Vec<Option<Arc<ItemSchema>>>) -> Vec<ItemWrapper> {
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
        .sorted_by(|a, b| {
            if a == b {
                Ordering::Equal
            } else {
                match item_cmp(a.artifact1(), b.artifact1()) {
                    Ordering::Less => Ordering::Less,
                    Ordering::Equal => match item_cmp(a.artifact2(), b.artifact2()) {
                        Ordering::Less => Ordering::Less,
                        Ordering::Equal => item_cmp(a.artifact3(), b.artifact3()),
                        Ordering::Greater => Ordering::Greater,
                    },
                    Ordering::Greater => Ordering::Greater,
                }
            }
        })
        .dedup()
        .map(ItemWrapper::Artifacts)
        .collect_vec()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Filter {
    pub available_only: bool,
    pub craftable: bool,
    pub from_task: bool,
    pub from_npc: bool,
    pub from_monster: bool,
    pub utilities: bool,
}

impl Default for Filter {
    fn default() -> Self {
        Self {
            available_only: false,
            craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: false,
            utilities: false,
        }
    }
}

impl Filter {
    pub const fn available_only() -> Self {
        Self {
            available_only: true,
            craftable: false,
            from_task: false,
            from_npc: false,
            from_monster: false,
            utilities: false,
        }
    }

    pub const fn everything() -> Self {
        Self {
            available_only: false,
            craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: true,
            utilities: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ItemWrapper {
    Armor(Option<Arc<ItemSchema>>),
    Rings(RingSet),
    Artifacts(ArtifactSet),
    Utility(UtilitySet),
}

#[derive(Clone, Debug, PartialEq)]
struct RingSet {
    rings: [Option<Arc<ItemSchema>>; 2],
}

impl RingSet {
    fn new(mut rings: [Option<Arc<ItemSchema>>; 2]) -> Option<Self> {
        if rings[0].is_none() && rings[1].is_none() {
            None
        } else {
            rings.sort_by(item_cmp);
            Some(RingSet { rings })
        }
    }

    fn slot(&self, slot: Slot) -> &Option<Arc<ItemSchema>> {
        match slot {
            Slot::Ring1 => self.ring1(),
            Slot::Ring2 => self.ring2(),
            _ => &None,
        }
    }

    fn ring1(&self) -> &Option<Arc<ItemSchema>> {
        &self.rings[0]
    }

    fn ring2(&self) -> &Option<Arc<ItemSchema>> {
        &self.rings[1]
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ArtifactSet {
    artifacts: [Option<Arc<ItemSchema>>; 3],
}

impl ArtifactSet {
    fn new(mut artifacts: [Option<Arc<ItemSchema>>; 3]) -> Option<Self> {
        if artifacts[0].is_some() && artifacts[0] == artifacts[1]
            || artifacts[1].is_some() && artifacts[1] == artifacts[2]
            || artifacts[0].is_some() && artifacts[0] == artifacts[2]
            || (artifacts[0].is_none() && artifacts[1].is_none() && artifacts[2].is_none())
        {
            None
        } else {
            artifacts.sort_by(item_cmp);
            Some(ArtifactSet { artifacts })
        }
    }

    fn slot(&self, slot: Slot) -> &Option<Arc<ItemSchema>> {
        match slot {
            Slot::Artifact1 => self.artifact1(),
            Slot::Artifact2 => self.artifact2(),
            Slot::Artifact3 => self.artifact3(),
            _ => &None,
        }
    }

    fn artifact1(&self) -> &Option<Arc<ItemSchema>> {
        &self.artifacts[0]
    }

    fn artifact2(&self) -> &Option<Arc<ItemSchema>> {
        &self.artifacts[1]
    }

    fn artifact3(&self) -> &Option<Arc<ItemSchema>> {
        &self.artifacts[2]
    }
}

#[derive(Debug, Clone, PartialEq)]
struct UtilitySet {
    utilities: [Option<Arc<ItemSchema>>; 2],
}

impl UtilitySet {
    fn new(mut utilities: [Option<Arc<ItemSchema>>; 2]) -> Option<Self> {
        if utilities[0].is_some() && utilities[0] == utilities[1]
            || utilities[0].is_none() && utilities[1].is_none()
        {
            None
        } else {
            utilities.sort_by(item_cmp);
            Some(UtilitySet { utilities })
        }
    }

    fn slot(&self, slot: Slot) -> &Option<Arc<ItemSchema>> {
        match slot {
            Slot::Utility1 => self.utility1(),
            Slot::Utility2 => self.utility2(),
            _ => &None,
        }
    }

    fn utility1(&self) -> &Option<Arc<ItemSchema>> {
        &self.utilities[0]
    }

    fn utility2(&self) -> &Option<Arc<ItemSchema>> {
        &self.utilities[1]
    }
}

fn item_cmp(a: &Option<Arc<ItemSchema>>, b: &Option<Arc<ItemSchema>>) -> Ordering {
    if a == b {
        return Ordering::Equal;
    }
    let Some(a) = a else { return Ordering::Greater };
    let Some(b) = b else { return Ordering::Less };
    a.code.cmp(&b.code)
}

#[derive(Clone, Copy)]
pub enum GearPurpose<'a> {
    Combat(&'a MonsterSchema),
    Crafting(&'a ItemSchema),
    Gathering(&'a ResourceSchema),
}

#[cfg(test)]
mod tests {
    use artifactsmmo_sdk::{MonstersClient, models::CharacterSchema};

    use super::*;

    #[test]
    fn best_weapons_against() {
        let gear_finder = GearFinder::default();
        let char = CharacterController::default();
        let data = CharacterSchema {
            level: 30,
            ..Default::default()
        };
        char.update_data(data);

        let weapons = gear_finder
            .best_weapons(
                &char,
                &MonstersClient::default().get("vampire").unwrap(),
                Default::default(),
            )
            .collect_vec();
        assert_eq!(
            weapons,
            vec![ItemsClient::default().get("death_knight_sword").unwrap()]
        );
    }
}
