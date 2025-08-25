use artifactsmmo_sdk::{
    Items,
    char::{Character as CharacterClient, HasCharacterData},
    consts::FOOD_BLACK_LIST,
    items::ItemSchemaExt,
    models::{InventorySlot, ItemSchema, SimpleItemSchema},
};
use core::fmt;
use itertools::Itertools;
use log::debug;
use std::{
    fmt::{Display, Formatter},
    sync::{Arc, RwLock},
};
use thiserror::Error;

#[derive(Default)]
pub struct Inventory {
    client: Arc<CharacterClient>,
    reservations: RwLock<Vec<Arc<InventoryReservation>>>,
    items: Arc<Items>,
}

impl Inventory {
    pub fn new(client: Arc<CharacterClient>, items: Arc<Items>) -> Self {
        Inventory {
            client,
            reservations: RwLock::new(vec![]),
            items,
        }
    }

    /// Returns a copy of the inventory to be used while depositing or
    /// withdrawing items.
    pub fn content(&self) -> Vec<InventorySlot> {
        self.client.inventory.content()
    }

    pub fn simple_content(&self) -> Vec<SimpleItemSchema> {
        self.content()
            .iter()
            .filter(|i| !i.code.is_empty())
            .map(|s| SimpleItemSchema {
                code: s.code.clone(),
                quantity: s.quantity,
            })
            .collect_vec()
    }

    /// Returns the amount of item in the `Character` inventory.
    pub fn total_items(&self) -> i32 {
        self.client.inventory.total_items()
    }

    /// Returns the maximum number of item the inventory can contain.
    pub fn max_items(&self) -> i32 {
        self.client.inventory.max_items()
    }

    /// Returns the free spaces in the `Character` inventory.
    pub fn free_space(&self) -> i32 {
        self.client.inventory.free_space()
    }

    pub fn has_space_for(&self, item: &str, quantity: i32) -> bool {
        self.client.inventory.has_space_for(item, quantity)
    }

    /// Checks if the `Character` inventory is full (all slots are occupied or
    /// `inventory_max_items` is reached).
    pub fn is_full(&self) -> bool {
        self.total_items() >= self.max_items() || self.content().iter().all(|s| s.quantity > 0)
    }

    /// Returns the amount of the given item `code` in the `Character` inventory.
    pub fn total_of(&self, item: &str) -> i32 {
        self.client.inventory.total_of(item)
    }

    pub fn contains_mats_for(&self, item: &str, quantity: i32) -> bool {
        self.client.inventory.contains_mats_for(item, quantity)
    }

    pub fn consumable_food(&self) -> Vec<Arc<ItemSchema>> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food()
                        && i.level <= self.client.level()
                        && !FOOD_BLACK_LIST.contains(&i.code.as_str())
                })
            })
            .collect_vec()
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, item: &str) -> i32 {
        self.total_of(item) - self.quantity_reserved(item)
    }

    /// Make sure the `quantity` of `item` is reserved
    pub fn reserv(&self, item: &str, quantity: i32) -> Result<(), ReservationError> {
        let quantity_to_reserv = quantity - self.quantity_reserved(item);
        if quantity_to_reserv == 0 {
            return Ok(());
        } else if quantity_to_reserv > self.quantity_reservable(item) {
            return Err(ReservationError::InsufficientQuantity);
        }
        let Some(res) = self.get_reservation(item) else {
            self.add_reservation(item, quantity);
            return Ok(());
        };
        res.inc_quantity(quantity_to_reserv);
        debug!(
            "inventory({}): increased reservation quantity by '{}': [{}]",
            self.client.name(),
            quantity,
            res
        );
        Ok(())
    }

    /// Decrease the reserved quantity of `item`
    pub fn decrease_reservation(&self, item: &str, quantity: i32) {
        let Some(res) = self.get_reservation(item) else {
            return;
        };
        if quantity >= *res.quantity.read().unwrap() {
            self.remove_reservation(&res);
        } else {
            res.dec_quantity(quantity);
            debug!(
                "inventory({}): decreased reservation quantity by '{}': [{}]",
                self.client.name(),
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
        debug!(
            "{}: added reservation to inventory: {}",
            self.client.name(),
            res
        );
    }

    pub fn remove_reservation(&self, reservation: &InventoryReservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        debug!(
            "inventory({}): removed reservation: {}",
            self.client.name(),
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

    fn quantity_reservable(&self, item: &str) -> i32 {
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

#[derive(Debug, Error)]
pub enum ReservationError {
    #[error("Insufficient item quantity in inventory: ")]
    InsufficientQuantity,
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
