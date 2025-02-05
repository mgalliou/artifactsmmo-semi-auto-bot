use crate::{items::ItemSchemaExt, ITEMS};
use artifactsmmo_openapi::models::ItemSchema;
use itertools::Itertools;
use std::sync::Arc;

use super::CharacterData;

pub struct BaseInventory {
    data: CharacterData,
}

impl BaseInventory {
    pub fn new(data: CharacterData) -> Self {
        Self { data }
    }

    /// Returns the amount of item in the `Character` inventory.
    pub fn total_items(&self) -> i32 {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .map(|i| i.quantity)
            .sum()
    }

    /// Returns the maximum number of item the inventory can contain.
    pub fn max_items(&self) -> i32 {
        self.data.read().unwrap().inventory_max_items
    }

    /// Returns the free spaces in the `Character` inventory.
    pub fn free_space(&self) -> i32 {
        self.max_items() - self.total_items()
    }

    /// Checks if the `Character` inventory is full (all slots are occupied or
    /// `inventory_max_items` is reached).
    pub fn is_full(&self) -> bool {
        self.total_items() >= self.max_items()
            || self
                .data
                .read()
                .unwrap()
                .inventory
                .iter()
                .flatten()
                .all(|s| s.quantity > 0)
    }

    /// Returns the amount of the given item `code` in the `Character` inventory.
    pub fn total_of(&self, item: &str) -> i32 {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }

    pub fn contains_mats_for(&self, item: &str, quantity: i32) -> bool {
        ITEMS
            .mats_of(item)
            .iter()
            .all(|m| self.total_of(&m.code) >= m.quantity * quantity)
    }

    pub fn consumable_food(&self) -> Vec<Arc<ItemSchema>> {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter_map(|i| {
                ITEMS
                    .get(&i.code)
                    .filter(|i| i.is_consumable_at(self.data.read().unwrap().level))
            })
            .collect_vec()
    }
}
