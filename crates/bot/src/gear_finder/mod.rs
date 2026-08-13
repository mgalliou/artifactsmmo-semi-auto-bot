use itertools::Itertools;
use log::warn;
use ordered_float::OrderedFloat;
use sdk::{
    CanProvideXp, Code, CollectionClient, ItemsClient, Level, MAX_LEVEL,
    entities::{Item, Monster, Resource},
    gear::{Gear, Slot},
    items::{
        ItemSource,
        Type::{self, Rune},
    },
    simulator::{FightParams, FightSimulation, HasEffects, Participant, time_to_rest},
    skill::Skill,
    yields_xp,
};
use std::collections::{HashMap, HashSet};

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
}

impl GearFinder {
    #[must_use]
    pub const fn new(items: ItemsClient) -> Self {
        Self { items }
    }

    #[must_use]
    pub fn best_for(&self, purpose: GearPurpose) -> GearResolver {
        GearResolver::new(self.items.clone(), purpose)
    }
}

type CanCraftFn = Box<dyn Fn(&str) -> bool>;

pub struct GearResolver {
    items: ItemsClient,
    purpose: GearPurpose,
    skill_levels: HashMap<Skill, u32>,
    available_items: HashMap<String, u32>,
    filter: Filter,
    excluded_items: HashSet<String>,
    can_craft: Option<CanCraftFn>,
    item_pool: Vec<Item>,
}

impl GearResolver {
    fn new(items: ItemsClient, purpose: GearPurpose) -> Self {
        Self {
            items,
            purpose,
            skill_levels: HashMap::new(),
            available_items: HashMap::new(),
            filter: Filter::default(),
            excluded_items: HashSet::new(),
            can_craft: None,
            item_pool: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_skill_levels(mut self, levels: HashMap<Skill, u32>) -> Self {
        self.skill_levels = levels;
        self
    }

    #[must_use]
    pub fn with_available_items(mut self, items: HashMap<String, u32>) -> Self {
        self.available_items = items;
        self
    }

    #[must_use]
    pub fn with_excluded_items(mut self, items: Vec<String>) -> Self {
        self.excluded_items = items
            .into_iter()
            .filter(|code| {
                if self.items.get(code).is_none() {
                    warn!("excluded item '{code}' does not exist, ignoring");
                    false
                } else {
                    true
                }
            })
            .collect();
        self
    }

    #[must_use]
    pub const fn with_filter(mut self, filter: Filter) -> Self {
        self.filter = filter;
        self
    }

    #[must_use]
    pub fn with_can_craft(mut self, f: impl Fn(&str) -> bool + 'static) -> Self {
        self.can_craft = Some(Box::new(f));
        self
    }

    fn meets_skill_requirements(&self, item: &Item) -> bool {
        let (skill, level) = item.required_skill_level();
        self.skill_level(skill) >= level
    }

    fn create_item_pool(&self) -> Vec<Item> {
        let owned_items = self
            .available_items
            .iter()
            .filter(|(_, quantity)| **quantity > 0)
            .filter_map(|(code, _)| self.items.get(code))
            .filter(Item::is_equipable)
            .collect_vec();
        let mut item_pool = if self.filter.is_available_only() {
            vec![]
        } else {
            self.items
                .iter()
                .filter(|i| self.is_eligible(i))
                .collect_vec()
        };
        item_pool = [item_pool, owned_items].concat();
        item_pool.sort();
        item_pool.dedup();
        item_pool.retain(|i| self.meets_skill_requirements(i));
        item_pool
    }

    fn is_eligible(&self, item: &Item) -> bool {
        let Filter::Catalog {
            force_craftable,
            from_task,
            from_npc,
            from_monster,
            ..
        } = self.filter
        else {
            return false;
        };
        if !item.is_equipable() {
            return false;
        }
        if self.excluded_items.contains(item.code()) {
            return false;
        }
        if !from_npc && self.items.is_buyable(item.code()) {
            return false;
        }
        if !from_task && item.is_crafted_from_task() {
            return false;
        }
        if !from_monster
            && self
                .items
                .sources_of(item.code())
                .iter()
                .any(ItemSource::is_monster)
        {
            return false;
        }
        if let Some(can_craft) = &self.can_craft
            && force_craftable
            && item.is_craftable()
            && !can_craft(item.code())
        {
            return false;
        }
        true
    }

    /// Resolve the best gear based on the internal properties:
    /// `level` is the combat level of the character
    /// `skill_levels` is the skill levels of the character
    /// `items` is the item catalog from which the resolver builds its candidate pool
    /// `available` is the list of items available to the character with its quantity,
    /// items available are from inventory, bank, and current equipment
    /// `filter` filters items in the base item pool without filtering `available_items`.
    /// [`Filter::AvailableOnly`] ignores catalog items. In [`Filter::Catalog`], setting
    /// `force_craftable` checks craftable items in the base pool against the `can_craft` function.
    /// This does not filter `available_items`.
    ///
    /// When resolving gears with both catalog and available items, items from `available_items`
    /// are prioritized in case of a tie, and catalog items are considered of infinite quantity
    #[must_use]
    pub fn resolve(mut self) -> Option<Gear> {
        self.item_pool = self.create_item_pool();
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
                        .with_level(self.skill_level(Skill::Combat))
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

    fn best_weapons(&self, monster: &Monster) -> Vec<&Item> {
        self.item_pool
            .iter()
            .filter(|i| i.type_is(Type::Weapon) && !i.is_tool())
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
            let armors = self.best_combat_armors(monster, weapon, item_type);
            (!armors.is_empty()).then(|| armors.iter().map(GearComponent::from).collect_vec())
        })
        .collect_vec();

        let ring_sets = self.gen_combat_ring_sets(monster, weapon);
        push_if_not_empty(&mut items, ring_sets);
        if self.filter.utilities_allowed() {
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
        let rings = self.best_combat_armors(monster, weapon, Type::Ring);
        self.gen_ring_sets(&rings)
    }

    fn gen_combat_utility_sets(&self, monster: &Monster, weapon: &Item) -> Vec<GearComponent> {
        let utilities = self.best_combat_utilities(monster, weapon);
        gen_utility_sets(utilities)
    }

    fn gen_combat_artifact_sets(&self, monster: &Monster, weapon: &Item) -> Vec<GearComponent> {
        let artifacts = self.best_combat_armors(monster, weapon, Type::Artifact);
        gen_artifacts_sets(artifacts)
    }

    fn best_combat_armors(&self, monster: &Monster, weapon: &Item, r#type: Type) -> Vec<Item> {
        let mut bests: Vec<&Item> = vec![];
        let armors = self
            .item_pool
            .iter()
            .filter(|i| i.type_is(r#type))
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
            if monster.provides_xp_at(self.skill_level(Skill::Combat))
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
                let armors = self.best_skill_armors(item_type, skill);
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
            .filter(|i| i.is_tool() && i.skill_cooldown_reduction(skill) < 0)
            .min_by_key(|i| i.skill_cooldown_reduction(skill))
    }

    fn best_skill_armors(&self, r#type: Type, skill: Skill) -> Vec<Item> {
        let mut bests: Vec<&Item> = vec![];
        let armors = self
            .item_pool
            .iter()
            .filter(|i| {
                i.type_is(r#type)
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
        let rings = self.best_skill_armors(Type::Ring, skill);
        self.gen_ring_sets(&rings)
    }

    fn gen_skill_artifacts_sets(&self, skill: Skill) -> Vec<GearComponent> {
        let artifacts = self.best_skill_armors(Type::Artifact, skill);
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

    fn gen_ring_sets(&self, rings: &[Item]) -> Vec<GearComponent> {
        let rings = rings.iter().cloned().sorted().dedup().collect_vec();
        let mut sets = vec![];
        for (ring1_index, ring1) in rings.iter().enumerate() {
            for ring2 in &rings[ring1_index..] {
                if self.filter.is_available_only()
                    && ring1 == ring2
                    && self.available_items.get(ring1.code()) == Some(&1)
                {
                    continue;
                }
                if let Some(set) = RingSet::from_items(ring1, Some(ring2)) {
                    sets.push(set);
                }
            }
            if let Some(set) = RingSet::from_items(ring1, None) {
                sets.push(set);
            }
        }
        sets.into_iter().map(GearComponent::Rings).collect()
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

fn gen_utility_sets(utilities: Vec<Item>) -> Vec<GearComponent> {
    let utilities = utilities.into_iter().sorted().dedup().collect_vec();
    let mut sets = vec![];
    for (utility1_index, utility1) in utilities.iter().enumerate() {
        for utility2 in &utilities[utility1_index + 1..] {
            if let Some(set) = UtilitySet::from_items(utility1, Some(utility2)) {
                sets.push(set);
            }
        }
        if let Some(set) = UtilitySet::from_items(utility1, None) {
            sets.push(set);
        }
    }
    sets.into_iter().map(GearComponent::Utility).collect()
}

fn gen_artifacts_sets(items: Vec<Item>) -> Vec<GearComponent> {
    let candidates = items.into_iter().sorted().dedup().collect_vec();
    let mut sets = vec![];
    for (artifact1_index, artifact1) in candidates.iter().enumerate() {
        for artifact2_index in artifact1_index + 1..candidates.len() {
            let artifact2 = &candidates[artifact2_index];
            for artifact3 in &candidates[artifact2_index + 1..] {
                if let Some(set) =
                    ArtifactSet::from_items(artifact1, Some(artifact2), Some(artifact3))
                {
                    sets.push(set);
                }
            }
            if let Some(set) = ArtifactSet::from_items(artifact1, Some(artifact2), None) {
                sets.push(set);
            }
        }
        if let Some(set) = ArtifactSet::from_items(artifact1, None, None) {
            sets.push(set);
        }
    }
    sets.into_iter().map(GearComponent::Artifacts).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdk::test_utils::{ITEMS, item, monster, resource};

    #[test]
    fn resolver_best_weapons_against() {
        let gear = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("vampire")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 30)]))
            .resolve()
            .unwrap();
        let weapon = gear.item_in(Slot::Weapon).unwrap();
        assert_eq!(weapon.code(), "obsidian_battleaxe");
    }

    #[test]
    fn resolve_best_gear_against_blue_slime() {
        let gear = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("blue_slime")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 10)]))
            .with_filter(Filter::Catalog {
                force_craftable: true,
                from_task: false,
                from_npc: true,
                from_monster: true,
                utilities: false,
            })
            .resolve();
        assert_eq!(
            gear.unwrap().to_string(),
            Gear::default()
                .with_weapon(item("forest_staff"))
                .with_helmet(item("adventurer_helmet"))
                .with_shield(item("iron_shield"))
                .with_body_armor(item("iron_armor"))
                .with_leg_armor(item("iron_legs_armor"))
                .with_boots(item("iron_boots"))
                .with_amulet(item("fire_and_earth_amulet"))
                .with_ring1(item("forest_ring"))
                .with_ring2(item("iron_ring"))
                .with_artifact1(item("lich_race_medal"))
                .with_artifact2(item("novice_guide"))
                .with_bag(item("backpack"))
                .to_string()
        );
    }

    #[test]
    fn gen_ring_sets_are_canonical_and_ordered() {
        let items = vec![
            item("forest_ring"),
            item("copper_ring"),
            item("forest_ring"),
        ];
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")));
        let result = resolver.gen_ring_sets(&items);
        let codes = result
            .iter()
            .map(|component| match component {
                GearComponent::Rings(set) => (
                    set.ring1().map_or("", Code::code),
                    set.ring2().map_or("", Code::code),
                ),
                _ => panic!("expected Rings"),
            })
            .collect_vec();

        assert_eq!(
            codes,
            [
                ("copper_ring", "copper_ring"),
                ("copper_ring", "forest_ring"),
                ("copper_ring", ""),
                ("forest_ring", "forest_ring"),
                ("forest_ring", ""),
            ]
        );
    }

    #[test]
    fn gen_ring_sets_respect_single_copy_availability() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
            .with_available_items(HashMap::from([("copper_ring".into(), 1)]))
            .with_filter(Filter::available_only());
        let result = resolver.gen_ring_sets(&[item("copper_ring")]);
        let GearComponent::Rings(set) = &result[0] else {
            panic!("expected Rings")
        };

        assert_eq!(result.len(), 1);
        assert_eq!(set.ring1().map(Code::code), Some("copper_ring"));
        assert_eq!(set.ring2(), None);
    }

    #[test]
    fn gen_ring_sets_ignore_owned_quantity_in_catalog_mode() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
            .with_available_items(HashMap::from([("copper_ring".into(), 1)]));
        let result = resolver.gen_ring_sets(&[item("copper_ring")]);
        let GearComponent::Rings(set) = &result[0] else {
            panic!("expected Rings")
        };

        assert_eq!(result.len(), 2);
        assert_eq!(set.ring1().map(Code::code), Some("copper_ring"));
        assert_eq!(set.ring2().map(Code::code), Some("copper_ring"));
    }

    #[test]
    fn create_item_pool_ignores_zero_quantity_available_items() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
            .with_available_items(HashMap::from([
                ("iron_sword".into(), 0),
                ("forest_ring".into(), 0),
                ("copper_ring".into(), 1),
            ]))
            .with_filter(Filter::available_only());
        let item_pool = resolver.create_item_pool();

        assert_eq!(item_pool, [item("copper_ring")]);
    }

    #[test]
    fn available_only_can_include_utilities() {
        let resolve = |filter| {
            GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
                .with_skill_levels(HashMap::from([(Skill::Combat, 30)]))
                .with_available_items(HashMap::from([
                    ("wooden_staff".into(), 1),
                    ("health_potion".into(), 1),
                ]))
                .with_filter(filter)
                .resolve()
                .unwrap()
        };

        assert_eq!(resolve(Filter::available_only()).utility1, None);
        assert_eq!(
            resolve(Filter::AvailableOnly { utilities: true })
                .utility1
                .as_ref()
                .map(Code::code),
            Some("health_potion")
        );
    }

    #[test]
    fn zero_quantity_ring_is_not_equipped() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 10)]))
            .with_available_items(HashMap::from([
                ("iron_sword".into(), 1),
                ("forest_ring".into(), 0),
            ]))
            .with_filter(Filter::available_only());
        let gear = resolver.resolve().unwrap();

        assert_eq!(gear.ring1, None);
        assert_eq!(gear.ring2, None);
    }

    #[test]
    fn gen_utility_sets_are_canonical_and_ordered() {
        let items = vec![
            item("health_potion"),
            item("antidote"),
            item("health_potion"),
        ];
        let result = gen_utility_sets(items);
        let codes = result
            .iter()
            .map(|component| match component {
                GearComponent::Utility(set) => (
                    set.utility1().map_or("", Code::code),
                    set.utility2().map_or("", Code::code),
                ),
                _ => panic!("expected Utility"),
            })
            .collect_vec();

        assert_eq!(
            codes,
            [
                ("antidote", "health_potion"),
                ("antidote", ""),
                ("health_potion", ""),
            ]
        );
    }

    #[test]
    fn gen_artifacts_sets_are_canonical_and_ordered() {
        let items = vec![
            item("novice_guide"),
            item("corrupted_skull"),
            item("life_crystal"),
            item("novice_guide"),
        ];
        let result = gen_artifacts_sets(items);
        let codes = result
            .iter()
            .map(|component| match component {
                GearComponent::Artifacts(set) => (
                    set.artifact1().map_or("", Code::code),
                    set.artifact2().map_or("", Code::code),
                    set.artifact3().map_or("", Code::code),
                ),
                _ => panic!("expected Artifacts"),
            })
            .collect_vec();

        assert_eq!(
            codes,
            [
                ("corrupted_skull", "life_crystal", "novice_guide"),
                ("corrupted_skull", "life_crystal", ""),
                ("corrupted_skull", "novice_guide", ""),
                ("corrupted_skull", "", ""),
                ("life_crystal", "novice_guide", ""),
                ("life_crystal", "", ""),
                ("novice_guide", "", ""),
            ]
        );
    }

    #[test]
    fn unique_ring_not_in_both_slots() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("blue_slime")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 10)]))
            .with_available_items(HashMap::from([
                ("iron_sword".into(), 1),
                ("forest_ring".into(), 1),
            ]))
            .with_filter(Filter::available_only());
        let gear = resolver.resolve().unwrap();
        assert!(gear.ring1.is_some());
        assert_ne!(gear.ring1, gear.ring2);
    }

    #[test]
    fn two_distinct_single_copy_rings_can_be_equipped() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("blue_slime")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 10)]))
            .with_available_items(HashMap::from([
                ("iron_sword".into(), 1),
                ("forest_ring".into(), 1),
                ("iron_ring".into(), 1),
            ]))
            .with_filter(Filter::available_only());
        let gear = resolver.resolve().unwrap();

        assert_eq!(gear.ring1.as_ref().unwrap().code(), "forest_ring");
        assert_eq!(gear.ring2.as_ref().unwrap().code(), "iron_ring");
    }

    #[test]
    fn duplicate_ring_with_two_copies() {
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("blue_slime")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 10)]))
            .with_available_items(HashMap::from([
                ("iron_sword".into(), 1),
                ("forest_ring".into(), 2),
            ]))
            .with_filter(Filter::available_only());
        let gear = resolver.resolve().unwrap();
        assert_eq!(gear.ring1.unwrap().code(), item("forest_ring").code());
    }

    #[test]
    fn resolve_best_gear_against_chicken() {
        let gear = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(monster("chicken")))
            .with_skill_levels(HashMap::from([(Skill::Combat, 1)]))
            .resolve();
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

    #[test]
    fn resolve_best_gear_against_iron_ore() {
        let resolver = GearResolver::new(
            ITEMS.clone(),
            GearPurpose::Gathering(resource("iron_rocks")),
        )
        .with_skill_levels(HashMap::from([(Skill::Mining, 15), (Skill::Combat, 15)]))
        .with_filter(Filter::Catalog {
            force_craftable: true,
            from_task: true,
            from_npc: true,
            from_monster: true,
            utilities: false,
        });
        let gear = resolver.resolve().unwrap();
        assert_eq!(gear.item_in(Slot::Weapon).unwrap(), &item("iron_pickaxe"));
        assert_eq!(gear.item_in(Slot::Helmet).unwrap(), &item("wolf_ears"));
        assert_eq!(
            gear.item_in(Slot::LegArmor).unwrap(),
            &item("adventurer_pants")
        );
        assert_eq!(
            gear.item_in(Slot::Boots).unwrap(),
            &item("adventurer_boots")
        );
        assert_eq!(gear.item_in(Slot::Amulet).unwrap(), &item("wisdom_amulet"));
        assert_eq!(gear.item_in(Slot::Ring1).unwrap(), &item("life_ring"));
        assert_eq!(gear.item_in(Slot::Ring2).unwrap(), &item("life_ring"));
        assert_eq!(
            gear.item_in(Slot::Artifact1).unwrap(),
            &item("novice_guide")
        );
    }

    #[test]
    fn prioritizes_available_items() {
        // lizard_skin_armor and stormforged_armor tie on DamageBoost against
        // vampire with dreadful_staff (both give 6.48). The tiebreaker in
        // best_by_among should pick the one in available_items.
        let vamp = monster("vampire");
        let filter = Filter::Catalog {
            from_task: true,
            from_npc: true,
            from_monster: false,
            force_craftable: true,
            utilities: false,
        };
        let excluded_items = vec!["snakeskin_armor".into(), "steel_armor".into()];
        let resolver = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(vamp.clone()))
            .with_skill_levels(HashMap::from([(Skill::Combat, 25)]))
            .with_available_items(HashMap::from([("lizard_skin_armor".into(), 1)]))
            .with_excluded_items(excluded_items.clone())
            .with_filter(filter);
        let gear = resolver.resolve().unwrap();
        assert_eq!(gear.weapon.unwrap().code(), "dreadful_staff");
        assert_eq!(gear.body_armor.unwrap().code(), "lizard_skin_armor");

        // Reverse: put stormforged_armor in available_items instead
        let gear = GearResolver::new(ITEMS.clone(), GearPurpose::Combat(vamp))
            .with_skill_levels(HashMap::from([(Skill::Combat, 25)]))
            .with_available_items(HashMap::from([("stormforged_armor".into(), 1)]))
            .with_excluded_items(excluded_items)
            .with_filter(filter)
            .resolve()
            .unwrap();
        assert_eq!(gear.weapon.unwrap().code(), "dreadful_staff");
        assert_eq!(gear.body_armor.unwrap().code(), "stormforged_armor");
    }
}
