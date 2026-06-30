use log::error;
use openapi::models::{
    AccessSchema, CharacterFightSchema, ConditionSchema, DropRateSchema, DropSchema,
    InventorySlotSchema, RewardsSchema, SimpleItemSchema, SkillInfoSchema, TransitionSchema,
};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::fs;

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

#[cfg(test)]
pub(crate) mod test_support;

pub(crate) trait Cached<D>
where
    D: for<'a> Deserialize<'a> + Serialize,
{
    fn path(&self) -> &str;

    /// Returns cached data, falling back to `fetch_from_source` when cache is unavailable
    fn fetch(&self) -> D {
        self.fetch_from_cache::<D>().unwrap_or_else(|_| {
            let data = self.fetch_from_source();
            if let Err(e) = self.cache(&data) {
                error!("failed to cache data: {e}");
            }
            data
        })
    }

    /// Reads and deserializes data from the local cache file
    fn fetch_from_cache<T: for<'a> Deserialize<'a>>(&self) -> anyhow::Result<T> {
        Ok(serde_json::from_str(&fs::read_to_string(self.path())?)?)
    }

    /// Writes data to the local cache file
    fn cache<T: Serialize>(&self, data: T) -> anyhow::Result<()> {
        Ok(fs::write(
            self.path(),
            &serde_json::to_string_pretty(&data)?,
        )?)
    }

    /// Returns data from the source of truth (e.g., the `ArtifactMMO` API)
    fn fetch_from_source(&self) -> D;

    /// Updates the local cache directly from the source of truth
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

impl Code for InventorySlotSchema {
    fn code(&self) -> &str {
        &self.code
    }
}

impl Code for SimpleItemSchema {
    fn code(&self) -> &str {
        &self.code
    }
}

impl Code for DropRateSchema {
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

impl Quantity for InventorySlotSchema {
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
                    .find(|i| i.code() == item_code)
                    .map_or(0, Quantity::quantity)
            })
            .sum()
    }
}

impl HasDrops for SkillInfoSchema {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.items
            .iter()
            .find(|i| i.code() == item_code)
            .map_or(0, Quantity::quantity)
    }
}

impl HasDrops for RewardsSchema {
    fn amount_of(&self, item_code: &str) -> u32 {
        self.items
            .iter()
            .find(|i| i.code() == item_code)
            .map_or(0, |i| i.quantity)
    }
}

impl<T> HasDrops for Vec<T>
where
    T: Code + Quantity,
{
    fn amount_of(&self, item_code: &str) -> u32 {
        self.iter()
            .find(|i| i.code() == item_code)
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
            .find(|d| d.code() == item_code)
            .map_or(0.0, DropRateSchemaExt::rate)
    }

    fn effective_drop_rate_of(&self, item_code: &str) -> f32 {
        self.drops()
            .iter()
            .find(|d| d.code() == item_code)
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
    fn conditions(&self) -> Option<&Vec<ConditionSchema>>;
}

impl HasConditions for AccessSchema {
    fn conditions(&self) -> Option<&Vec<ConditionSchema>> {
        self.conditions.as_ref()
    }
}

impl HasConditions for TransitionSchema {
    fn conditions(&self) -> Option<&Vec<ConditionSchema>> {
        self.conditions.as_ref()
    }
}

pub trait Level {
    fn level(&self) -> u32;
}

pub trait CanProvideXp: Level {
    fn provides_xp_at(&self, level: u32) -> bool {
        yields_xp(level, self.level())
    }
}

pub trait DropRateSchemaExt {
    fn effective_rate(&self) -> f32 {
        self.rate() * self.average_quantity()
    }

    fn average_quantity(&self) -> f32 {
        (self.min_quantity() + self.max_quantity()) as f32 / 2.0
    }

    fn rate(&self) -> f32;
    fn min_quantity(&self) -> u32;
    fn max_quantity(&self) -> u32;
}

impl DropRateSchemaExt for DropRateSchema {
    fn min_quantity(&self) -> u32 {
        self.min_quantity
    }

    fn max_quantity(&self) -> u32 {
        self.max_quantity
    }

    fn rate(&self) -> f32 {
        self.rate as f32 / 100.0
    }
}

pub struct ItemList<'a, T>(pub &'a [T])
where
    T: Code + Quantity;

impl<T> Display for ItemList<'_, T>
where
    T: Code + Quantity,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

/// Checks a character at the `char_level` would receive XP by crafting, killing,
/// or gathering an entity at `entity_level`
#[must_use]
pub const fn yields_xp(char_level: u32, entity_level: u32) -> bool {
    char_level >= entity_level && char_level.saturating_sub(entity_level) <= MAX_LEVEL_DIFF
}
