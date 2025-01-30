use crate::gear::Slot;
use artifactsmmo_openapi::models::{CharacterSchema, TaskType};
use chrono::{DateTime, Utc};
use std::sync::{Arc, RwLock};

pub use base_character::{BaseCharacter, HasDrops};
pub use character::{Character, CharacterError};
pub use skill::Skill;

pub mod action;
pub mod base_character;
pub mod character;
pub mod skill;

pub trait HasCharacterData {
    fn data(&self) -> Arc<RwLock<CharacterSchema>>;

    fn name(&self) -> String {
        self.data().read().unwrap().name.to_owned()
    }

    /// Returns the `Character` position (coordinates).
    fn position(&self) -> (i32, i32) {
        let binding = self.data();
        let d = binding.read().unwrap();
        let (x, y) = (d.x, d.y);
        (x, y)
    }

    fn level(&self) -> i32 {
        self.data().read().unwrap().level
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        let binding = self.data();
        let d = binding.read().unwrap();
        match skill {
            Skill::Combat => d.xp,
            Skill::Mining => d.mining_xp,
            Skill::Woodcutting => d.woodcutting_xp,
            Skill::Fishing => d.fishing_xp,
            Skill::Weaponcrafting => d.weaponcrafting_xp,
            Skill::Gearcrafting => d.gearcrafting_xp,
            Skill::Jewelrycrafting => d.jewelrycrafting_xp,
            Skill::Cooking => d.cooking_xp,
            Skill::Alchemy => d.alchemy_xp,
        }
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        let binding = self.data();
        let d = binding.read().unwrap();
        match skill {
            Skill::Combat => d.max_xp,
            Skill::Mining => d.mining_max_xp,
            Skill::Woodcutting => d.woodcutting_max_xp,
            Skill::Fishing => d.fishing_max_xp,
            Skill::Weaponcrafting => d.weaponcrafting_max_xp,
            Skill::Gearcrafting => d.gearcrafting_max_xp,
            Skill::Jewelrycrafting => d.jewelrycrafting_max_xp,
            Skill::Cooking => d.cooking_max_xp,
            Skill::Alchemy => d.alchemy_max_xp,
        }
    }

    fn max_health(&self) -> i32 {
        self.data().read().unwrap().max_hp
    }

    fn health(&self) -> i32 {
        self.data().read().unwrap().hp
    }

    fn missing_hp(&self) -> i32 {
        self.max_health() - self.health()
    }

    /// Returns the `Character` level in the given `skill`.
    fn skill_level(&self, skill: Skill) -> i32 {
        let binding = self.data();
        let d = binding.read().unwrap();
        match skill {
            Skill::Combat => d.level,
            Skill::Mining => d.mining_level,
            Skill::Woodcutting => d.woodcutting_level,
            Skill::Fishing => d.fishing_level,
            Skill::Weaponcrafting => d.weaponcrafting_level,
            Skill::Gearcrafting => d.gearcrafting_level,
            Skill::Jewelrycrafting => d.jewelrycrafting_level,
            Skill::Cooking => d.cooking_level,
            Skill::Alchemy => d.alchemy_level,
        }
    }

    fn gold(&self) -> i32 {
        self.data().read().unwrap().gold
    }

    fn quantity_in_slot(&self, s: Slot) -> i32 {
        match s {
            Slot::Utility1 => self.data().read().unwrap().utility1_slot_quantity,
            Slot::Utility2 => self.data().read().unwrap().utility2_slot_quantity,
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
            | Slot::Artifact3 => 1,
        }
    }

    fn task(&self) -> String {
        self.data().read().unwrap().task.to_owned()
    }

    fn task_type(&self) -> Option<TaskType> {
        match self.data().read().unwrap().task_type.as_str() {
            "monsters" => Some(TaskType::Monsters),
            "items" => Some(TaskType::Items),
            _ => None,
        }
    }

    fn task_progress(&self) -> i32 {
        self.data().read().unwrap().task_progress
    }

    fn task_total(&self) -> i32 {
        self.data().read().unwrap().task_total
    }

    fn task_missing(&self) -> i32 {
        self.task_total() - self.task_progress()
    }

    fn task_finished(&self) -> bool {
        !self.task().is_empty() && self.task_progress() >= self.task_total()
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data()
            .read()
            .unwrap()
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }
}
