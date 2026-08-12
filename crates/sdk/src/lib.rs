use log::error;
use openapi::models::{
    AccessSchema, CharacterFightSchema, ConditionSchema, DropRateSchema, DropSchema,
    InventorySlotSchema, RewardsSchema, SimpleItemSchema, SkillInfoSchema, TaskTradeSchema,
    TransitionSchema,
};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::ops::Deref;
use std::string::String;

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
pub mod event_bus;
pub mod gear;
pub mod simulator;
pub mod skill;

pub use event_bus::{EventBus, SdkEvent};

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub(crate) trait Cached<D>
where
    D: for<'a> Deserialize<'a> + Serialize,
{
    const FILE: &'static str;

    fn cache_dir(&self) -> &str;

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
        Ok(ron::from_str(&fs::read_to_string(self.cache_path())?)?)
    }

    /// Writes data to the local cache file
    fn cache<T: Serialize>(&self, data: T) -> anyhow::Result<()> {
        Ok(fs::write(
            self.cache_path(),
            &ron::ser::to_string_pretty(&data, PrettyConfig::default())?,
        )?)
    }

    fn cache_path(&self) -> String {
        format!("{}/{}.{}", self.cache_dir(), Self::FILE, "ron")
    }

    /// Returns data from the source of truth (e.g., the `ArtifactMMO` API)
    fn fetch_from_source(&self) -> D;

    /// Updates the local cache directly from the source of truth
    fn refresh(&self);
}

pub trait Code {
    fn code(&self) -> &str;
}

impl<T> Code for &T
where
    T: Code + ?Sized,
{
    fn code(&self) -> &str {
        (**self).code()
    }
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

impl Code for TaskTradeSchema {
    fn code(&self) -> &str {
        &self.code
    }
}

impl<T: AsRef<str>> Code for (T, u32) {
    fn code(&self) -> &str {
        self.0.as_ref()
    }
}

pub trait Quantity {
    fn quantity(&self) -> u32;
}

impl<T> Quantity for &T
where
    T: Quantity + ?Sized,
{
    fn quantity(&self) -> u32 {
        (**self).quantity()
    }
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

impl Quantity for TaskTradeSchema {
    fn quantity(&self) -> u32 {
        self.quantity as u32
    }
}

impl<T: AsRef<str>> Quantity for (T, u32) {
    fn quantity(&self) -> u32 {
        self.1
    }
}

/// Trait used on struct containing droped items
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
            .map_or(0, Quantity::quantity)
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

/// Trait used on struct containing a drop table
pub trait HasDropTable {
    type Drops: DropRateSchemaExt;

    /// Returns the drop probability of an item per kill/gather
    fn probability_of(&self, item_code: &str) -> f32 {
        self.drops()
            .iter()
            .find(|d| d.code() == item_code)
            .map_or(0.0, DropRateSchemaExt::probability)
    }

    /// Returns the expected item quantity per kill/gather
    fn expected_quantity(&self) -> f32 {
        self.drops()
            .iter()
            .map(DropRateSchemaExt::expected_quantity)
            .sum()
    }

    /// Returns the expected item quantity of an item per kill/gather
    fn expected_quantity_of(&self, item_code: &str) -> f32 {
        self.drops()
            .iter()
            .find(|d| d.code() == item_code)
            .map_or(0.0, DropRateSchemaExt::expected_quantity)
    }

    /// Returns the expected number of distinct item drops per kill/gather
    fn expected_slots(&self) -> f32 {
        self.drops()
            .iter()
            .map(DropRateSchemaExt::probability)
            .sum()
    }

    fn drops(&self) -> &[Self::Drops];
}

impl<T, U> HasDropTable for T
where
    T: Deref<Target = [U]>,
    U: DropRateSchemaExt,
{
    type Drops = U;

    fn drops(&self) -> &[Self::Drops] {
        self
    }
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

pub trait DropRateSchemaExt: Code {
    /// Returns the expected quantity per kill/gather
    fn expected_quantity(&self) -> f32 {
        self.probability() * self.average_quantity()
    }

    /// Returns the drop probability, where `rate` is a 1-in-`rate` chance
    ///
    /// A `rate` of `0` is invalid data and is treated as a `0` chance.
    fn probability(&self) -> f32 {
        if self.rate() == 0 {
            0.0
        } else {
            1.0 / self.rate() as f32
        }
    }

    /// Returns the average item quantity when the item drops
    ///
    /// `min_quantity` is expected to be lower than or equal to `max_quantity`.
    fn average_quantity(&self) -> f32 {
        f64::midpoint(
            f64::from(self.min_quantity()),
            f64::from(self.max_quantity()),
        ) as f32
    }

    fn min_quantity(&self) -> u32;

    fn max_quantity(&self) -> u32;

    fn rate(&self) -> u32;
}

impl DropRateSchemaExt for DropRateSchema {
    fn min_quantity(&self) -> u32 {
        self.min_quantity
    }

    fn max_quantity(&self) -> u32 {
        self.max_quantity
    }

    fn rate(&self) -> u32 {
        self.rate
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

#[cfg(test)]
mod tests {
    use crate::{
        DropRateSchemaExt, HasDropTable,
        models::DropRateSchema,
        test_utils::{monster, resource},
    };

    fn assert_close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-5, "expected {b}, got {a}");
    }

    fn assert_close_rel(a: f32, b: f32) {
        assert!((a - b).abs() <= b * 1e-4, "expected {b} ± 0.01%, got {a}");
    }

    #[test]
    fn chicken_drop_table() {
        let chicken = monster("chicken");
        assert_close(chicken.probability_of("raw_chicken"), 0.5);
        assert_close(chicken.probability_of("egg"), 1.0 / 12.0);
        assert_close(chicken.probability_of("feather"), 0.125);
        assert_close(chicken.probability_of("golden_egg"), 0.001);
        assert_close(chicken.probability_of("event_ticket"), 0.01);
        assert_close(chicken.expected_quantity_of("raw_chicken"), 0.5);
        assert_close(chicken.expected_quantity_of("egg"), 1.0 / 12.0);
        assert_close(chicken.expected_slots(), 0.719_333);
        assert_close(chicken.expected_quantity(), 0.719_333);
    }

    #[test]
    fn gold_rocks_drop_table() {
        let gold_rocks = resource("gold_rocks");
        assert_close(gold_rocks.probability_of("gold_ore"), 1.0);
        assert_close(gold_rocks.probability_of("topaz_stone"), 0.01);
        assert_close(gold_rocks.probability_of("event_ticket"), 0.005);
        assert_close(gold_rocks.expected_quantity_of("gold_ore"), 1.0);
        assert_close(gold_rocks.expected_slots(), 1.045);
        assert_close(gold_rocks.expected_quantity(), 1.045);
    }

    #[test]
    fn missing_item_returns_zero() {
        let chicken = monster("chicken");
        assert_close(chicken.probability_of("nonexistent"), 0.0);
        assert_close(chicken.expected_quantity_of("nonexistent"), 0.0);
    }

    #[test]
    fn zero_rate_is_not_infinite() {
        let drop = DropRateSchema::new("broken".into(), 0, 1, 1);
        assert_close(drop.probability(), 0.0);
        assert_close(drop.expected_quantity(), 0.0);

        let table = vec![drop];
        assert_close(table.expected_slots(), 0.0);
        assert_close(table.expected_quantity(), 0.0);
        assert_close(table.probability_of("broken"), 0.0);
    }

    #[test]
    fn large_quantities_no_overflow() {
        let drop = DropRateSchema::new("huge".into(), 1, 2_200_000_000, 2_200_000_000);
        assert_close_rel(drop.average_quantity(), 2_200_000_000.0);
        assert_close_rel(drop.expected_quantity(), 2_200_000_000.0);
    }
}
