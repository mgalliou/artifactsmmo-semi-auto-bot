use fs_extra::file::{read_to_string, write_all};
use log::error;
use openapi::models::{
    AccessSchema, CharacterFightSchema, ConditionSchema, DropRateSchema, DropSchema, InventorySlot,
    RewardsSchema, SimpleItemSchema, SkillInfoSchema, TransitionSchema,
};
use serde::{Deserialize, Serialize};
use std::{fmt, path::Path};

pub use openapi::models;
pub use sdk_derive::CollectionClient;

pub use client::*;
pub use consts::*;
pub use container::*;
pub use gear::*;
pub use skill::*;

pub mod client;
pub mod consts;
pub mod container;
pub mod entities;
pub mod gear;
pub mod simulator;
pub mod skill;

pub(crate) trait Persist<D: for<'a> Deserialize<'a> + Serialize> {
    const PATH: &'static str;

    fn load(&self) -> D {
        self.load_from_file::<D>().unwrap_or_else(|_| {
            let data = self.load_from_api();
            if let Err(e) = Self::persist(&data) {
                error!("failed to persist data: {e}");
            }
            data
        })
    }

    fn load_from_api(&self) -> D;

    fn load_from_file<T: for<'a> Deserialize<'a>>(&self) -> Result<T, Box<dyn std::error::Error>> {
        Ok(serde_json::from_str(&read_to_string(Path::new(
            Self::PATH,
        ))?)?)
    }

    fn persist<T: Serialize>(data: T) -> Result<(), Box<dyn std::error::Error>> {
        Ok(write_all(
            Path::new(Self::PATH),
            &serde_json::to_string_pretty(&data)?,
        )?)
    }

    fn refresh(&self);
}

pub trait Code {
    fn code(&self) -> &str;
}

impl Code for DropSchema {
    fn code(&self) -> &str {
        &self.code
    }
}

impl Code for InventorySlot {
    fn code(&self) -> &str {
        &self.code
    }
}

impl Code for SimpleItemSchema {
    fn code(&self) -> &str {
        &self.code
    }
}

pub trait Quantity {
    fn quantity(&self) -> u32;
}

impl Quantity for DropSchema {
    fn quantity(&self) -> u32 {
        self.quantity as u32
    }
}

impl Quantity for InventorySlot {
    fn quantity(&self) -> u32 {
        self.quantity as u32
    }
}

impl Quantity for SimpleItemSchema {
    fn quantity(&self) -> u32 {
        self.quantity
    }
}

pub trait HasDrops {
    fn amount_of(&self, item_code: &str) -> u32;
}

impl HasDrops for CharacterFightSchema {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.characters
            .iter()
            .map(|c| {
                c.drops
                    .iter()
                    .find(|i| i.code == item_code)
                    .map_or(0, Quantity::quantity)
            })
            .sum()
    }
}

impl HasDrops for SkillInfoSchema {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.items
            .iter()
            .find(|i| i.code == item_code)
            .map_or(0, Quantity::quantity)
    }
}

impl HasDrops for RewardsSchema {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.items
            .iter()
            .find(|i| i.code == item_code)
            .map_or(0, |i| i.quantity)
    }
}

impl HasDrops for Vec<SimpleItemSchema> {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.iter()
            .find(|i| i.code == item_code)
            .map_or(0, |i| i.quantity)
    }
}

impl HasDrops for Vec<DropSchema> {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.iter()
            .find(|i| i.code == item_code)
            .map_or(0, Quantity::quantity)
    }
}

pub trait DropsItems {
    fn average_drop_quantity(&self) -> u32 {
        self.drops()
            .iter()
            .map(DropRateSchemaExt::effective_rate)
            .sum::<f32>()
            .ceil() as u32
    }

    fn drop_rate_of(&self, item_code: &str) -> f32 {
        self.drops()
            .iter()
            .find(|d| d.code == item_code)
            .map_or(0.0, DropRateSchemaExt::rate)
    }

    fn effective_drop_rate_of(&self, item_code: &str) -> f32 {
        self.drops()
            .iter()
            .find(|d| d.code == item_code)
            .map_or(0.0, DropRateSchemaExt::effective_rate)
    }

    fn average_drop_slots(&self) -> u32 {
        self.drops()
            .iter()
            .map(DropRateSchemaExt::rate)
            .sum::<f32>()
            .ceil() as u32
    }

    fn min_drop_quantity(&self) -> u32 {
        self.drops().iter().map(|i| i.min_quantity).sum()
    }

    fn max_drop_quantity(&self) -> u32 {
        self.drops().iter().map(|i| i.max_quantity).sum()
    }

    fn drops(&self) -> &Vec<DropRateSchema>;
}

pub trait HasConditions {
    fn conditions(&self) -> &Option<Vec<ConditionSchema>>;
}

impl HasConditions for AccessSchema {
    fn conditions(&self) -> &Option<Vec<ConditionSchema>> {
        &self.conditions
    }
}

impl HasConditions for &TransitionSchema {
    fn conditions(&self) -> &Option<Vec<ConditionSchema>> {
        &self.conditions
    }
}

pub trait Level {
    fn level(&self) -> u32;
}

pub trait CanProvideXp: Level {
    fn provides_xp_at(&self, level: u32) -> bool {
        check_lvl_diff(level, self.level())
    }
}

pub trait DropRateSchemaExt {
    fn average_quantity(&self) -> f32;
    fn rate(&self) -> f32;
    fn effective_rate(&self) -> f32;
}

impl DropRateSchemaExt for DropRateSchema {
    fn average_quantity(&self) -> f32 {
        (self.min_quantity + self.max_quantity) as f32 / 2.0
    }

    fn rate(&self) -> f32 {
        self.rate as f32 / 100.0
    }

    fn effective_rate(&self) -> f32 {
        self.rate() * self.average_quantity()
    }
}

/// Checks a character at the `char_level` would receive XP by crafting, killing,
/// or gathering an entity at the `entity_level`
pub const fn check_lvl_diff(char_level: u32, entity_level: u32) -> bool {
    char_level >= entity_level && char_level.saturating_sub(entity_level) <= MAX_LEVEL_DIFF
}

pub struct ItemList<'a, T: Code + Quantity>(pub &'a [T]);

impl<T> fmt::Display for ItemList<'_, T>
where
    T: Code + Quantity,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut empty = true;
        for item in self.0 {
            if !empty {
                write!(f, ", ")?;
            }
            write!(f, "'{}'x{}", item.code(), item.quantity())?;
            empty = false;
        }
        Ok(())
    }
}
