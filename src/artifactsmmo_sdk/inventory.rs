use super::{
    character::CharacterError,
    items::{Items, Type, FOOD_BLACK_LIST},
    ItemSchemaExt,
};
use artifactsmmo_openapi::models::{CharacterSchema, InventorySlot, ItemSchema};
use core::fmt;
use itertools::Itertools;
use log::info;
use std::{
    fmt::{Display, Formatter},
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct Inventory {
    items: Arc<Items>,
    data: Arc<RwLock<CharacterSchema>>,
    reservations: RwLock<Vec<Arc<InventoryReservation>>>,
}

impl Inventory {
    pub fn new(data: &Arc<RwLock<CharacterSchema>>, items: &Arc<Items>) -> Self {
        Inventory {
            items: items.clone(),
            data: data.clone(),
            reservations: RwLock::new(vec![]),
        }
    }

    /// Returns a copy of the inventory to be used while depositing or
    /// withdrawing items.
    pub fn copy(&self) -> Vec<InventorySlot> {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .cloned()
            .collect_vec()
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

    pub fn consumable_food(&self) -> Vec<&ItemSchema> {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|&i| {
                    i.is_of_type(Type::Consumable)
                        && i.heal() > 0
                        && i.level <= self.data.read().unwrap().level
                        && !FOOD_BLACK_LIST.contains(&i.code.as_str())
                })
            })
            .collect_vec()
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, item: &str) -> i32 {
        self.total_of(item) - self.quantity_reserved(item)
    }

    pub fn reserv(&self, item: &str, quantity: i32) -> Result<(), CharacterError> {
        let Some(res) = self.get_reservation(item) else {
            return self.increase_reservation(item, quantity);
        };
        if res.quantity() >= quantity {
            Ok(())
        } else if self.quantity_not_reserved(item) >= quantity - res.quantity() {
            res.inc_quantity(quantity - res.quantity());
            info!(
                "inventory({}): increased reservation quantity by '{}': [{}]",
                self.data.read().unwrap().name,
                quantity,
                res
            );
            Ok(())
        } else {
            Err(CharacterError::QuantityUnavailable(quantity))
        }
    }

    fn increase_reservation(&self, item: &str, quantity: i32) -> Result<(), CharacterError> {
        let Some(res) = self.get_reservation(item) else {
            if quantity > self.total_of(item) {
                return Err(CharacterError::QuantityUnavailable(quantity));
            }
            self.add_reservation(item, quantity);
            return Ok(());
        };
        if quantity > self.quantity_not_reserved(item) {
            return Err(CharacterError::QuantityUnavailable(quantity));
        }
        res.inc_quantity(quantity);
        Ok(())
    }

    pub fn decrease_reservation(&self, item: &str, quantity: i32) {
        let Some(res) = self.get_reservation(item) else {
            return;
        };
        if quantity >= *res.quantity.read().unwrap() {
            self.remove_reservation(&res)
        } else {
            res.dec_quantity(quantity);
            info!(
                "inventory({}): decreased reservation quantity by '{}': [{}]",
                self.data.read().unwrap().name,
                quantity,
                res
            );
        }
    }

    fn add_reservation(&self, item: &str, quantity: i32) {
        let res = Arc::new(InventoryReservation {
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
        });
        self.reservations.write().unwrap().push(res.clone());
        info!(
            "{}: added reservation to inventory: {}",
            self.data.read().unwrap().name,
            res
        );
    }

    pub fn remove_reservation(&self, reservation: &InventoryReservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        info!(
            "inventory({}): removed reservation: {}",
            self.data.read().unwrap().name,
            reservation
        );
    }

    fn quantity_reserved(&self, item: &str) -> i32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter_map(|r| {
                if r.item == item {
                    Some(r.quantity())
                } else {
                    None
                }
            })
            .sum()
    }

    fn quantity_not_reserved(&self, item: &str) -> i32 {
        self.total_of(item) - self.quantity_reserved(item)
    }

    fn get_reservation(&self, item: &str) -> Option<Arc<InventoryReservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .find(|r| r.item == item)
            .cloned()
    }
}

#[derive(Debug)]
pub struct InventoryReservation {
    item: String,
    quantity: RwLock<i32>,
}

impl InventoryReservation {
    pub fn inc_quantity(&self, i: i32) {
        *self.quantity.write().unwrap() += i;
    }

    pub fn dec_quantity(&self, i: i32) {
        *self.quantity.write().unwrap() -= i;
    }

    pub fn quantity(&self) -> i32 {
        *self.quantity.read().unwrap()
    }
}

impl Display for InventoryReservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "'{}'x{}", self.item, self.quantity.read().unwrap(),)
    }
}

impl PartialEq for InventoryReservation {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item && self.quantity() == other.quantity()
    }
}
