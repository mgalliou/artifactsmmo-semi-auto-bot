use super::character::CharacterError;
use artifactsmmo_openapi::models::{CharacterSchema, InventorySlot};
use core::fmt;
use itertools::Itertools;
use log::info;
use std::{
    fmt::{Display, Formatter},
    sync::{Arc, RwLock},
};

pub struct Inventory {
    data: Arc<RwLock<CharacterSchema>>,
    reservations: RwLock<Vec<Arc<InventoryReservation>>>,
}

impl Inventory {
    pub fn new(data: &Arc<RwLock<CharacterSchema>>) -> Inventory {
        Inventory {
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
    pub fn total(&self) -> i32 {
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
        self.max_items() - self.total()
    }

    /// Checks if the `Character` inventory is full (all slots are occupied or
    /// `inventory_max_items` is reached).
    pub fn is_full(&self) -> bool {
        self.total() >= self.max_items()
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
    pub fn contains(&self, code: &str) -> i32 {
        self.data
            .read()
            .unwrap()
            .inventory
            .iter()
            .flatten()
            .find(|i| i.code == code)
            .map_or(0, |i| i.quantity)
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, code: &str) -> i32 {
        self.contains(code) - self.quantity_reserved(code)
    }

    pub fn quantity_reserved(&self, item: &str) -> i32 {
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

    pub fn quantity_not_reserved(&self, item: &str) -> i32 {
        self.contains(item) - self.quantity_reserved(item)
    }

    pub fn get_reservation(&self, item: &str) -> Option<Arc<InventoryReservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .find(|r| r.item == item)
            .cloned()
    }

    pub fn reserv_items_if_not(&self, item: &str, quantity: i32) -> Result<(), CharacterError> {
        let Some(res) = self.get_reservation(item) else {
            return self.reserv(item, quantity);
        };
        if res.quantity() >= quantity {
            Ok(())
        } else if self.quantity_not_reserved(item) >= quantity - res.quantity() {
            res.inc_quantity(quantity - res.quantity());
            Ok(())
        } else {
            Err(CharacterError::QuantityUnavailable(quantity))
        }
    }

    fn reserv(&self, item: &str, quantity: i32) -> Result<(), CharacterError> {
        let Some(res) = self.get_reservation(item) else {
            if quantity > self.contains(item) {
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

    fn add_reservation(&self, item: &str, quantity: i32) {
        let res = Arc::new(InventoryReservation {
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
        });
        self.reservations.write().unwrap().push(res.clone());
        info!("added reservation to bank: {}", res);
    }

    pub fn remove_reservation(&self, reservation: &InventoryReservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        info!("removed reservation from bank: {}", reservation);
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
        info!("increased quantity of reservation by '{}': [{}]", i, self);
    }

    pub fn dec_quantity(&self, i: i32) {
        *self.quantity.write().unwrap() -= i;
        info!("decreased quantity of reservation by '{}': [{}]", i, self);
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
