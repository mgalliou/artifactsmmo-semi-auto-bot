use artifactsmmo_sdk::{
    HasQuantity, ItemContainer, Items, LimitedContainer, SlotLimited, SpaceLimited,
    char::{Character as CharacterClient, HasCharacterData},
    items::ItemSchemaExt,
    models::{InventorySlot, ItemSchema, SimpleItemSchema},
};
use core::fmt;
use itertools::Itertools;
use log::debug;
use std::{
    fmt::{Display, Formatter},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};
use thiserror::Error;

use crate::FOOD_BLACK_LIST;

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

    pub fn simple_content(&self) -> Vec<SimpleItemSchema> {
        self.content()
            .iter()
            .filter(|i| !i.code.is_empty())
            .map(|s| SimpleItemSchema {
                code: s.code.clone(),
                quantity: s.quantity(),
            })
            .collect_vec()
    }

    pub fn missing_mats_for(&self, item: &str, quantity: u32) -> Vec<SimpleItemSchema> {
        self.items
            .mats_for(item, quantity)
            .iter()
            .filter_map(|m| {
                let missing = m.quantity.saturating_sub(self.has_available(&m.code));
                (missing > 0).then_some(SimpleItemSchema {
                    code: m.code.clone(),
                    quantity: missing,
                })
            })
            .collect_vec()
    }

    pub fn consumable_food(&self) -> Vec<Arc<ItemSchema>> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food()
                        && i.level <= self.client.level()
                        && !FOOD_BLACK_LIST.contains(&i.code.as_ref())
                })
            })
            .collect_vec()
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, item: &str) -> u32 {
        self.total_of(item)
            .saturating_sub(self.quantity_reserved(item))
    }

    /// Make sure the `quantity` of `item` is reserved
    pub fn reserv_items(
        &self,
        items: &[SimpleItemSchema],
    ) -> Result<(), InventoryReservationError> {
        for item in items.iter() {
            self.reserv_item(&item.code, item.quantity)?
        }
        Ok(())
    }

    /// Make sure the `quantity` of `item` is reserved
    pub fn reserv_item(&self, item: &str, quantity: u32) -> Result<(), InventoryReservationError> {
        let Some(res) = self.get_reservation(item) else {
            self.add_reservation(item, quantity);
            return Ok(());
        };
        let quantity_to_reserv = quantity.saturating_sub(self.quantity_reserved(item));
        if quantity_to_reserv == 0 {
            return Ok(());
        } else if quantity_to_reserv > self.quantity_reservable(item) {
            return Err(InventoryReservationError::InsufficientQuantity);
        }
        res.inc_quantity(quantity_to_reserv);
        debug!(
            "{}: increased '{item}' inventory reservation by {quantity}",
            self.client.name(),
        );
        Ok(())
    }

    /// Decrease the reserved quantity of `item`
    pub fn unreserv_items(&self, items: &[SimpleItemSchema]) {
        for item in items.iter() {
            self.unreserv_item(&item.code, item.quantity);
        }
    }

    /// Decrease the reserved quantity of `item`
    pub fn unreserv_item(&self, item: &str, quantity: u32) {
        let Some(res) = self.get_reservation(item) else {
            return;
        };
        if quantity >= res.quantity() {
            self.remove_reservation(&res);
        } else {
            res.dec_quantity(quantity);
            debug!(
                "{}: decreased '{item}' inventory reservation by {quantity}",
                self.client.name(),
            );
        }
    }

    fn add_reservation(&self, item: &str, quantity: u32) {
        let res = Arc::new(InventoryReservation {
            item: item.to_owned(),
            quantity: AtomicU32::new(quantity),
        });
        self.reservations.write().unwrap().push(res.clone());
        debug!("{}: added inventory reservation: {res}", self.client.name(),);
    }

    pub fn remove_reservation(&self, reservation: &InventoryReservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        debug!(
            "{}: removed inventory reservation: {reservation}",
            self.client.name(),
        );
    }

    fn quantity_reserved(&self, item: &str) -> u32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter_map(|r| (r.item == item).then_some(r.quantity()))
            .sum()
    }

    pub fn is_reserved(&self, item: &str) -> bool {
        self.quantity_reserved(item) > 0
    }

    fn quantity_reservable(&self, item: &str) -> u32 {
        self.total_of(item)
            .saturating_sub(self.quantity_reserved(item))
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

impl ItemContainer for Inventory {
    type Slot = InventorySlot;

    fn content(&self) -> Arc<Vec<InventorySlot>> {
        self.client.inventory().content()
    }
}

impl SpaceLimited for Inventory {
    fn max_items(&self) -> u32 {
        self.client.inventory().max_items()
    }
}

impl LimitedContainer for Inventory {}
impl SlotLimited for Inventory {}

#[derive(Debug)]
pub struct InventoryReservation {
    item: String,
    quantity: AtomicU32,
}

#[derive(Debug, Error)]
pub enum InventoryReservationError {
    #[error("Insufficient item quantity in inventory")]
    InsufficientQuantity,
}

impl InventoryReservation {
    pub fn inc_quantity(&self, n: u32) {
        self.quantity.fetch_add(n, SeqCst);
    }

    pub fn dec_quantity(&self, n: u32) {
        let new = self.quantity().saturating_sub(n);
        self.quantity.store(new, SeqCst);
    }

    pub fn quantity(&self) -> u32 {
        self.quantity.load(SeqCst)
    }
}

impl Display for InventoryReservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "'{}'x{}", self.item, self.quantity())
    }
}

impl PartialEq for InventoryReservation {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item && self.quantity() == other.quantity()
    }
}
