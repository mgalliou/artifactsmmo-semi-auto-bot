use crate::{Level, Skill, Slot};
use chrono::{DateTime, Utc};
use openapi::models::{CharacterSchema, InventorySlot, MapLayer, TaskType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::IntoEnumIterator;

pub trait CharacterTrait {
    fn name(&self) -> &str;
    fn position(&self) -> (MapLayer, i32, i32);
    fn skill_level(&self, skill: Skill) -> u32;
    fn skill_xp(&self, skill: Skill) -> i32;
    fn skill_max_xp(&self, skill: Skill) -> i32;
    fn hp(&self) -> i32;
    fn max_hp(&self) -> i32;
    fn missing_hp(&self) -> i32;
    fn task(&self) -> String;
    fn task_type(&self) -> Option<TaskType>;
    fn task_progress(&self) -> u32;
    fn task_total(&self) -> u32;
    fn task_missing(&self) -> u32;
    fn task_finished(&self) -> bool;
    fn inventory_items(&self) -> Option<Vec<InventorySlot>>;
    fn inventory_max_items(&self) -> i32;
    fn gold(&self) -> u32;
    fn equiped_in(&self, slot: Slot) -> String;
    fn has_equiped(&self, item_code: &str) -> u32;
    fn quantity_in_slot(&self, slot: Slot) -> u32;
    fn cooldown_expiration(&self) -> Option<DateTime<Utc>>;
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawCharacter(Arc<CharacterSchema>);

impl RawCharacter {
    pub fn new(schema: CharacterSchema) -> Self {
        Self(Arc::new(schema))
    }
}

impl CharacterTrait for RawCharacter {
    fn name(&self) -> &str {
        &self.0.name
    }

    fn position(&self) -> (MapLayer, i32, i32) {
        (self.0.layer, self.0.x, self.0.y)
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        let inner = &self.0;
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
        let inner = &self.0;
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
        let inner = &self.0;
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
        self.0.hp
    }

    fn max_hp(&self) -> i32 {
        self.0.max_hp
    }

    fn missing_hp(&self) -> i32 {
        self.max_hp() - self.hp()
    }

    fn gold(&self) -> u32 {
        self.0.gold as u32
    }

    fn task(&self) -> String {
        self.0.task.clone()
    }

    fn task_type(&self) -> Option<TaskType> {
        match self.0.task_type.as_str() {
            "monsters" => Some(TaskType::Monsters),
            "items" => Some(TaskType::Items),
            _ => None,
        }
    }

    fn task_progress(&self) -> u32 {
        self.0.task_progress as u32
    }

    fn task_total(&self) -> u32 {
        self.0.task_total as u32
    }

    fn task_missing(&self) -> u32 {
        self.task_total().saturating_sub(self.task_progress())
    }

    fn task_finished(&self) -> bool {
        !self.task().is_empty() && self.task_missing() < 1
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.0
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }

    fn equiped_in(&self, slot: Slot) -> String {
        let inner = &self.0;
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
        .to_string()
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        Slot::iter()
            .filter_map(|s| (self.equiped_in(s) == item_code).then_some(self.quantity_in_slot(s)))
            .sum()
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        match slot {
            Slot::Utility1 => self.0.utility1_slot_quantity,
            Slot::Utility2 => self.0.utility2_slot_quantity,
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
            | Slot::Rune => {
                if self.equiped_in(slot).is_empty() {
                    0
                } else {
                    1
                }
            }
        }
    }

    fn inventory_items(&self) -> Option<Vec<InventorySlot>> {
        self.0.inventory.clone()
    }

    fn inventory_max_items(&self) -> i32 {
        self.0.inventory_max_items
    }
}

impl Level for RawCharacter {
    fn level(&self) -> u32 {
        self.0.level as u32
    }
}
