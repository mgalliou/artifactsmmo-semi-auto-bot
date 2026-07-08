use crate::{Level, Skill, Slot};
use chrono::{DateTime, FixedOffset};
use derive_more::{Deref, Display, From};
use openapi::models::{CharacterSchema, InventorySlotSchema, MapLayer, TaskType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use strum::IntoEnumIterator;

pub trait Character: Level {
    fn name(&self) -> CharacterName;
    fn position(&self) -> (MapLayer, i32, i32);
    fn skill_level(&self, skill: Skill) -> u32;
    fn skill_xp(&self, skill: Skill) -> i32;
    fn skill_max_xp(&self, skill: Skill) -> i32;
    fn hp(&self) -> i32;
    fn max_hp(&self) -> i32;
    fn missing_hp(&self) -> i32;
    fn task(&self) -> TaskCode;
    fn task_type(&self) -> Option<TaskType>;
    fn task_progress(&self) -> u32;
    fn task_total(&self) -> u32;
    fn task_missing(&self) -> u32;
    fn task_finished(&self) -> bool;
    fn inventory_items(&self) -> Arc<Vec<InventorySlotSchema>>;
    fn inventory_max_items(&self) -> u32;
    fn gold(&self) -> u32;
    fn equiped_in(&self, slot: Slot) -> String;
    fn has_equiped(&self, item_code: &str) -> u32;
    fn quantity_in_slot(&self, slot: Slot) -> u32;
    fn cooldown_expiration(&self) -> Option<DateTime<FixedOffset>>;
}

#[derive(Debug, Clone)]
pub struct CharacterHandle(Arc<RwLock<RawCharacter>>);

impl CharacterHandle {
    #[must_use]
    pub fn load(&self) -> RawCharacter {
        self.0.read().unwrap().clone()
    }

    pub(crate) fn store(&self, data: RawCharacter) {
        *self.0.write().unwrap() = data;
    }
}

impl Character for CharacterHandle {
    fn name(&self) -> CharacterName {
        self.0.read().unwrap().name()
    }

    fn position(&self) -> (MapLayer, i32, i32) {
        self.0.read().unwrap().position()
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        self.0.read().unwrap().skill_level(skill)
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        self.0.read().unwrap().skill_xp(skill)
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        self.0.read().unwrap().skill_max_xp(skill)
    }

    fn hp(&self) -> i32 {
        self.0.read().unwrap().hp()
    }

    fn max_hp(&self) -> i32 {
        self.0.read().unwrap().max_hp()
    }

    fn missing_hp(&self) -> i32 {
        self.0.read().unwrap().missing_hp()
    }

    fn task(&self) -> TaskCode {
        self.0.read().unwrap().task()
    }

    fn task_type(&self) -> Option<TaskType> {
        self.0.read().unwrap().task_type()
    }

    fn task_progress(&self) -> u32 {
        self.0.read().unwrap().task_progress()
    }

    fn task_total(&self) -> u32 {
        self.0.read().unwrap().task_total()
    }

    fn task_missing(&self) -> u32 {
        self.0.read().unwrap().task_missing()
    }

    fn task_finished(&self) -> bool {
        self.0.read().unwrap().task_finished()
    }

    fn inventory_items(&self) -> Arc<Vec<InventorySlotSchema>> {
        self.0.read().unwrap().inventory_items()
    }

    fn inventory_max_items(&self) -> u32 {
        self.0.read().unwrap().inventory_max_items()
    }

    fn gold(&self) -> u32 {
        self.0.read().unwrap().gold()
    }

    fn equiped_in(&self, slot: Slot) -> String {
        self.0.read().unwrap().equiped_in(slot)
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        self.0.read().unwrap().has_equiped(item_code)
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        self.0.read().unwrap().quantity_in_slot(slot)
    }

    fn cooldown_expiration(&self) -> Option<DateTime<FixedOffset>> {
        self.0.read().unwrap().cooldown_expiration()
    }
}

impl Level for CharacterHandle {
    fn level(&self) -> u32 {
        self.0.read().unwrap().level()
    }
}

impl From<CharacterSchema> for CharacterHandle {
    fn from(value: CharacterSchema) -> Self {
        Self(Arc::new(RwLock::new(RawCharacter::from(value))))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawCharacter {
    schema: Arc<CharacterSchema>,
    name: CharacterName,
    task: TaskCode,
    inventory: Arc<Vec<InventorySlotSchema>>,
}

impl Character for RawCharacter {
    fn name(&self) -> CharacterName {
        self.name.clone()
    }

    fn position(&self) -> (MapLayer, i32, i32) {
        (self.schema.layer, self.schema.x, self.schema.y)
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        let inner = &self.schema;
        (match skill {
            Skill::Combat => inner.level,
            Skill::Mining => inner.mining_level,
            Skill::Woodcutting => inner.woodcutting_level,
            Skill::Fishing => inner.fishing_level,
            Skill::Weaponcrafting => inner.weaponcrafting_level,
            Skill::Gearcrafting => inner.gearcrafting_level,
            Skill::Jewelrycrafting => inner.jewelrycrafting_level,
            Skill::Cooking => inner.cooking_level,
            Skill::Alchemy => inner.alchemy_level,
        }) as u32
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        let inner = &self.schema;
        match skill {
            Skill::Combat => inner.xp,
            Skill::Mining => inner.mining_xp,
            Skill::Woodcutting => inner.woodcutting_xp,
            Skill::Fishing => inner.fishing_xp,
            Skill::Weaponcrafting => inner.weaponcrafting_xp,
            Skill::Gearcrafting => inner.gearcrafting_xp,
            Skill::Jewelrycrafting => inner.jewelrycrafting_xp,
            Skill::Cooking => inner.cooking_xp,
            Skill::Alchemy => inner.alchemy_xp,
        }
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        let inner = &self.schema;
        match skill {
            Skill::Combat => inner.max_xp,
            Skill::Mining => inner.mining_max_xp,
            Skill::Woodcutting => inner.woodcutting_max_xp,
            Skill::Fishing => inner.fishing_max_xp,
            Skill::Weaponcrafting => inner.weaponcrafting_max_xp,
            Skill::Gearcrafting => inner.gearcrafting_max_xp,
            Skill::Jewelrycrafting => inner.jewelrycrafting_max_xp,
            Skill::Cooking => inner.cooking_max_xp,
            Skill::Alchemy => inner.alchemy_max_xp,
        }
    }

    fn hp(&self) -> i32 {
        self.schema.hp
    }

    fn max_hp(&self) -> i32 {
        self.schema.max_hp
    }

    fn missing_hp(&self) -> i32 {
        self.max_hp() - self.hp()
    }

    fn gold(&self) -> u32 {
        self.schema.gold as u32
    }

    fn task(&self) -> TaskCode {
        self.task.clone()
    }

    fn task_type(&self) -> Option<TaskType> {
        match self.schema.task_type.as_str() {
            "monsters" => Some(TaskType::Monsters),
            "items" => Some(TaskType::Items),
            _ => None,
        }
    }

    fn task_progress(&self) -> u32 {
        self.schema.task_progress as u32
    }

    fn task_total(&self) -> u32 {
        self.schema.task_total as u32
    }

    fn task_missing(&self) -> u32 {
        self.task_total().saturating_sub(self.task_progress())
    }

    fn task_finished(&self) -> bool {
        !self.task().is_empty() && self.task_missing() < 1
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
    fn cooldown_expiration(&self) -> Option<DateTime<FixedOffset>> {
        self.schema.cooldown_expiration
    }

    fn equiped_in(&self, slot: Slot) -> String {
        let inner = &self.schema;
        match slot {
            Slot::Weapon => &inner.weapon_slot,
            Slot::Shield => &inner.shield_slot,
            Slot::Helmet => &inner.helmet_slot,
            Slot::BodyArmor => &inner.body_armor_slot,
            Slot::LegArmor => &inner.leg_armor_slot,
            Slot::Boots => &inner.boots_slot,
            Slot::Ring1 => &inner.ring1_slot,
            Slot::Ring2 => &inner.ring2_slot,
            Slot::Amulet => &inner.amulet_slot,
            Slot::Artifact1 => &inner.artifact1_slot,
            Slot::Artifact2 => &inner.artifact2_slot,
            Slot::Artifact3 => &inner.artifact3_slot,
            Slot::Utility1 => &inner.utility1_slot,
            Slot::Utility2 => &inner.utility2_slot,
            Slot::Bag => &inner.bag_slot,
            Slot::Rune => &inner.rune_slot,
        }
        .clone()
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        Slot::iter()
            .filter(|&s| self.equiped_in(s) == item_code)
            .map(|s| self.quantity_in_slot(s))
            .sum()
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        match slot {
            Slot::Utility1 => self.schema.utility1_slot_quantity,
            Slot::Utility2 => self.schema.utility2_slot_quantity,
            Slot::Weapon
            | Slot::Shield
            | Slot::Helmet
            | Slot::BodyArmor
            | Slot::LegArmor
            | Slot::Boots
            | Slot::Ring1
            | Slot::Ring2
            | Slot::Amulet
            | Slot::Artifact1
            | Slot::Artifact2
            | Slot::Artifact3
            | Slot::Bag
            | Slot::Rune => u32::from(!self.equiped_in(slot).is_empty()),
        }
    }

    fn inventory_items(&self) -> Arc<Vec<InventorySlotSchema>> {
        self.inventory.clone()
    }

    fn inventory_max_items(&self) -> u32 {
        self.schema.inventory_max_items as u32
    }
}

impl Level for RawCharacter {
    fn level(&self) -> u32 {
        self.schema.level as u32
    }
}

impl From<CharacterSchema> for RawCharacter {
    fn from(value: CharacterSchema) -> Self {
        Self {
            name: value.name.clone().into(),
            task: value.task.clone().into(),
            inventory: value.inventory.clone().unwrap_or_default().into(),
            schema: value.into(),
        }
    }
}

#[derive(
    Debug, Default, Clone, Hash, PartialEq, Eq, Display, Deref, From, Serialize, Deserialize,
)]
#[deref(forward)]
#[from(forward)]
#[serde(transparent)]
pub struct CharacterName(Arc<str>);

impl CharacterName {
    pub fn new(name: impl Into<Self>) -> Self {
        name.into()
    }
}

impl From<&Self> for CharacterName {
    fn from(name: &Self) -> Self {
        name.clone()
    }
}

#[derive(
    Debug, Default, Clone, Hash, PartialEq, Eq, Display, Deref, From, Serialize, Deserialize,
)]
#[deref(forward)]
#[from(forward)]
#[serde(transparent)]
pub struct TaskCode(Arc<str>);

impl TaskCode {
    pub fn new(name: impl Into<Self>) -> Self {
        name.into()
    }
}

impl From<&Self> for TaskCode {
    fn from(name: &Self) -> Self {
        name.clone()
    }
}
