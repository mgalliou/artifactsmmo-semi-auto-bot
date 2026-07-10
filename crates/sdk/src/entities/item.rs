use crate::{
    CanProvideXp, Code, HasConditions, Level, Quantity, Skill, TASKS_REWARDS_SPECIFICS,
    items::{SubType, Type},
    simulator::{EffectCode, HasEffects},
    yields_xp,
};
use core::cmp::Ordering;
use core::fmt::{self, Display, Formatter};
use itertools::Itertools;
use openapi::models::{
    ConditionSchema, CraftSchema, ItemSchema, SimpleEffectSchema, SimpleItemSchema,
};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item(Arc<ItemSchema>);

impl Item {
    #[must_use]
    pub(crate) fn new(schema: ItemSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn is_crafted_with(&self, item_code: &str) -> bool {
        self.mats().iter().any(|m| m.code() == item_code)
    }

    #[must_use]
    pub fn mats_quantity(&self) -> u32 {
        self.mats().iter().map(Quantity::quantity).sum()
    }

    #[must_use]
    pub fn mats(&self) -> &[SimpleItemSchema] {
        self.craft_schema()
            .and_then(|s| s.items.as_deref())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn mats_for(&self, quantity: u32) -> Vec<SimpleItemSchema> {
        self.craft_schema()
            .iter()
            .filter_map(|i| i.items.clone())
            .flatten()
            .update(|i| i.quantity *= quantity)
            .collect_vec()
    }

    #[must_use]
    pub fn recycled_quantity(&self) -> u32 {
        let q = self.mats_quantity();
        q / 5 + u32::from(!q.is_multiple_of(5))
    }

    pub fn skill_to_craft(&self) -> Option<Skill> {
        self.craft_schema()
            .and_then(|schema| schema.skill)
            .map(Skill::from)
    }

    #[must_use]
    pub fn skill_to_craft_is(&self, skill: Skill) -> bool {
        self.skill_to_craft().is_some_and(|s| s == skill)
    }

    #[must_use]
    pub fn is_crafted_from_task(&self) -> bool {
        TASKS_REWARDS_SPECIFICS
            .iter()
            .any(|i| self.is_crafted_with(i))
    }

    #[must_use]
    pub fn is_craftable(&self) -> bool {
        self.craft_schema().is_some()
    }

    #[must_use]
    pub fn craft_quantity(&self) -> u32 {
        self.craft_schema()
            .and_then(|s| s.quantity.map(|q| q as u32))
            .unwrap_or(0)
    }

    #[must_use]
    pub fn is_tradeable(&self) -> bool {
        self.0.tradeable
    }

    #[must_use]
    pub fn is_recyclable(&self) -> bool {
        self.skill_to_craft()
            .is_some_and(|s| s.is_weaponcrafting() || s.is_gearcrafting() || s.is_jewelrycrafting())
    }

    #[must_use]
    pub fn craft_schema(&self) -> Option<&CraftSchema> {
        self.0.craft.as_deref()
    }

    #[must_use]
    pub fn is_equipable(&self) -> bool {
        match self.r#type() {
            Type::BodyArmor
            | Type::Weapon
            | Type::LegArmor
            | Type::Helmet
            | Type::Boots
            | Type::Shield
            | Type::Amulet
            | Type::Ring
            | Type::Artifact
            | Type::Utility
            | Type::Bag
            | Type::Rune => true,
            Type::Consumable | Type::Currency | Type::Resource => false,
        }
    }

    #[must_use]
    pub fn is_tool(&self) -> bool {
        self.subtype_is(SubType::Tool)
    }

    #[must_use]
    pub fn is_consumable(&self) -> bool {
        self.type_is(Type::Consumable)
    }

    #[must_use]
    pub fn is_food(&self) -> bool {
        self.is_consumable() && self.subtype_is(SubType::Food)
    }

    #[must_use]
    pub fn is_gold_bag(&self) -> bool {
        self.is_consumable() && self.subtype_is(SubType::Bag)
    }

    #[must_use]
    pub fn type_is(&self, r#type: Type) -> bool {
        self.0.r#type == r#type
    }

    #[must_use]
    pub fn r#type(&self) -> Type {
        Type::from_str(&self.0.r#type).expect("type to be valid")
    }

    #[must_use]
    pub fn subtype_is(&self, subtype: SubType) -> bool {
        self.subtype().is_some_and(|st| st == subtype)
    }

    #[must_use]
    pub fn subtype(&self) -> Option<SubType> {
        SubType::from_str(&self.0.subtype).ok()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    #[must_use]
    pub fn effects(&self) -> &[SimpleEffectSchema] {
        self.0.effects.as_deref().unwrap_or_default()
    }

    #[must_use]
    pub fn is_upgrade_of(&self, other: &Self) -> bool {
        self.type_is(other.r#type())
            && other.effects().iter().all(|e| {
                if e.code == EffectCode::InventorySpace
                    || e.code == EffectCode::Mining
                    || e.code == EffectCode::Woodcutting
                    || e.code == EffectCode::Fishing
                    || e.code == EffectCode::Alchemy
                {
                    self.effect_value(&e.code) <= e.value
                } else {
                    self.effect_value(&e.code) >= e.value
                }
            })
    }
}

impl Eq for Item {}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> Ordering {
        self.code().cmp(other.code())
    }
}

impl HasEffects for Item {
    fn effect_value(&self, effect: &str) -> i32 {
        self.0
            .effects
            .iter()
            .flatten()
            .find(|e| e.code == effect)
            .map_or(0, |e| e.value)
    }
}

impl Code for Item {
    fn code(&self) -> &str {
        &self.0.code
    }
}

impl Level for Item {
    fn level(&self) -> u32 {
        self.0.level
    }
}

impl HasConditions for Item {
    fn conditions(&self) -> Option<&Vec<ConditionSchema>> {
        self.0.conditions.as_ref()
    }
}

impl CanProvideXp for Item {
    fn provides_xp_at(&self, level: u32) -> bool {
        self.is_craftable() && yields_xp(level, self.level())
    }
}

impl Display for Item {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::item;

    #[test]
    fn item_ord_is_alphabetical_by_code() {
        let copper = item("copper_ring");
        let dreadful = item("dreadful_ring");
        let emerald = item("emerald_ring");
        let forest = item("forest_ring");
        let gold = item("gold_ring");
        let iron = item("iron_ring");

        assert!(copper < dreadful);
        assert!(dreadful < emerald);
        assert!(emerald < forest);
        assert!(forest < gold);
        assert!(gold < iron);
    }

    #[test]
    fn item_ord_option_none_less_than_some() {
        let none: Option<Item> = None;
        let some = Some(item("forest_ring"));
        assert!(none < some);
    }

    #[test]
    fn item_ord_option_some_ordered_by_code() {
        let a = Some(item("copper_ring"));
        let b = Some(item("dreadful_ring"));
        assert!(a < b);
    }
}
