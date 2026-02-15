use crate::{
    CanProvideXp, Code, HasConditions, Level, Skill, TASKS_REWARDS_SPECIFICS, check_lvl_diff,
    items::{SubType, Type},
    simulator::HasEffects,
};
use openapi::models::{
    ConditionSchema, CraftSchema, ItemSchema, SimpleEffectSchema, SimpleItemSchema,
};
use core::fmt;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item(Arc<ItemSchema>);

impl Item {
    pub fn new(schema: ItemSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn is_crafted_with(&self, item_code: &str) -> bool {
        self.mats().iter().any(|m| m.code == item_code)
    }

    pub fn mats_quantity(&self) -> u32 {
        self.mats().iter().map(|m| m.quantity).sum()
    }

    pub fn mats(&self) -> Vec<SimpleItemSchema> {
        self.craft_schema()
            .iter()
            .filter_map(|i| i.items.clone())
            .flatten()
            .collect_vec()
    }

    pub fn mats_for(&self, quantity: u32) -> Vec<SimpleItemSchema> {
        self.craft_schema()
            .iter()
            .filter_map(|i| i.items.clone())
            .flatten()
            .update(|i| i.quantity *= quantity)
            .collect_vec()
    }

    pub fn recycled_quantity(&self) -> u32 {
        let q = self.mats_quantity();
        q / 5 + if q.is_multiple_of(5) { 0 } else { 1 }
    }

    pub fn skill_to_craft(&self) -> Option<Skill> {
        self.craft_schema()
            .and_then(|schema| schema.skill)
            .map(Skill::from)
    }

    pub fn skill_to_craft_is(&self, skill: Skill) -> bool {
        self.skill_to_craft().is_some_and(|s| s == skill)
    }

    pub fn is_crafted_from_task(&self) -> bool {
        TASKS_REWARDS_SPECIFICS
            .iter()
            .any(|i| self.is_crafted_with(i))
    }

    pub fn is_craftable(&self) -> bool {
        self.craft_schema().is_some()
    }

    pub fn is_tradable(&self) -> bool {
        self.0.tradeable
    }

    pub fn is_recyclable(&self) -> bool {
        self.skill_to_craft()
            .is_some_and(|s| s.is_weaponcrafting() || s.is_gearcrafting() || s.is_jewelrycrafting())
    }

    pub fn craft_schema(&self) -> Option<&CraftSchema> {
        self.0.craft.as_deref()
    }

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

    pub fn is_tool(&self) -> bool {
        self.subtype_is(SubType::Tool)
    }

    pub fn is_consumable(&self) -> bool {
        self.type_is(Type::Consumable)
    }

    pub fn is_food(&self) -> bool {
        self.is_consumable() && self.subtype_is(SubType::Food)
    }

    pub fn is_gold_bag(&self) -> bool {
        self.is_consumable() && self.subtype_is(SubType::Bag)
    }

    pub fn type_is(&self, r#type: Type) -> bool {
        self.0.r#type == r#type
    }

    pub fn r#type(&self) -> Type {
        Type::from_str(&self.0.r#type).expect("type to be valid")
    }

    pub fn subtype_is(&self, subtype: SubType) -> bool {
        self.subtype().is_some_and(|st| st == subtype)
    }

    pub fn subtype(&self) -> Option<SubType> {
        SubType::from_str(&self.0.subtype).ok()
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }
}

impl HasEffects for Item {
    fn effects(&self) -> Vec<SimpleEffectSchema> {
        self.0.effects.iter().flatten().cloned().collect_vec()
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
    fn conditions(&self) -> &Option<Vec<ConditionSchema>> {
        &self.0.conditions
    }
}

impl CanProvideXp for Item {
    fn provides_xp_at(&self, level: u32) -> bool {
        self.is_craftable() && check_lvl_diff(level, self.level())
    }
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
