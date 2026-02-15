use crate::{FOOD_CONSUMPTION_BLACKLIST, HasReservation, InventoryDiscriminant, Reservation};
use core::fmt;
use itertools::Itertools;
use log::debug;
use sdk::{
    CharacterClient, Code, CollectionClient, DropsItems, ItemContainer, ItemsClient, Level,
    LimitedContainer, Quantity, SlotLimited, SpaceLimited,
    character::{HasCharacterData, inventory::Inventory},
    entities::Item,
    models::{InventorySlot, SimpleItemSchema},
};
use std::{
    fmt::{Display, Formatter},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};
use thiserror::Error;

#[derive(Default)]
pub struct InventoryController {
    client: Arc<CharacterClient>,
    reservations: RwLock<Vec<Arc<InventoryReservation>>>,
    items: Arc<ItemsClient>,
}

impl InventoryController {
    pub fn new(client: Arc<CharacterClient>, items: Arc<ItemsClient>) -> Self {
        InventoryController {
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

    pub fn missing_among(&self, items: &[SimpleItemSchema]) -> Vec<SimpleItemSchema> {
        items
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

    pub fn consumable_food(&self) -> Vec<Item> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food()
                        && i.level() <= self.client.level()
                        && !FOOD_CONSUMPTION_BLACKLIST.contains(&i.code())
                })
            })
            .collect_vec()
    }

    /// Returns the amount not reserved of the given item `code` in the `Character` inventory.
    pub fn has_available(&self, item: &str) -> u32 {
        self.total_of(item)
            .saturating_sub(self.quantity_reserved(item))
    }

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
        let Some(res) = self.get_reservation(item.into()) else {
            return self.add_reservation(item, quantity);
        };
        let quantity_to_reserv = quantity.saturating_sub(res.quantity());
        if quantity_to_reserv == 0 {
            return Ok(());
        };
        self.inc_reservation(item, quantity_to_reserv)
    }

    pub fn inc_reservation(
        &self,
        item: &str,
        quantity: u32,
    ) -> Result<(), InventoryReservationError> {
        if let Some(res) = self.get_reservation(item.into()) {
            if quantity > self.quantity_reservable(item) {
                return Err(InventoryReservationError::QuantityUnavailable(quantity));
            }
            res.inc_quantity(quantity);
        } else {
            self.add_reservation(item, quantity)?;
        }
        debug!(
            "{}: increased '{item}' inventory reservation by '{quantity}'",
            self.client.name()
        );
        Ok(())
    }

    /// Decrease the reserved quantity of `item`
    pub fn decrease_reservations(&self, items: &[SimpleItemSchema]) {
        for item in items.iter() {
            self.decrease_reservation(&item.code, item.quantity);
        }
    }

    /// Decrease the reserved quantity of `item`
    pub fn decrease_reservation(&self, item: &str, quantity: u32) {
        let Some(res) = self.get_reservation(item.into()) else {
            return;
        };
        res.dec_quantity(quantity);
        debug!(
            "{}: decreased '{item}' inventory reservation by {quantity}",
            self.client.name(),
        );
        if res.quantity() < 1 {
            self.remove_reservation(&res);
        }
    }

    fn add_reservation(&self, item: &str, quantity: u32) -> Result<(), InventoryReservationError> {
        if quantity > self.quantity_reservable(item) {
            return Err(InventoryReservationError::QuantityUnavailable(quantity));
        }
        let res = Arc::new(InventoryReservation {
            item: item.to_owned(),
            quantity: AtomicU32::new(quantity),
        });
        self.reservations.write().unwrap().push(res.clone());
        debug!("{}: added inventory reservation: {res}", self.client.name(),);
        Ok(())
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
}

impl Inventory for InventoryController {}

impl ItemContainer for InventoryController {
    type Slot = InventorySlot;

    fn content(&self) -> Arc<Vec<InventorySlot>> {
        self.client.inventory().content()
    }
}

impl LimitedContainer for InventoryController {
    fn is_full(&self) -> bool {
        self.client.inventory().is_full()
    }

    fn has_room_for_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        self.client.inventory().has_room_for_multiple(items)
    }

    fn has_room_for_drops_from<H: DropsItems>(&self, entity: &H) -> bool {
        self.client.inventory().has_room_for_drops_from(entity)
    }
}

impl SpaceLimited for InventoryController {
    fn max_items(&self) -> u32 {
        self.client.inventory().max_items()
    }
}

impl SlotLimited for InventoryController {
    fn free_slots(&self) -> u32 {
        self.client.inventory().free_slots()
    }
}

impl HasReservation for InventoryController {
    type Reservation = InventoryReservation;
    type Discriminant = InventoryDiscriminant;

    fn reservations(&self) -> Vec<Arc<Self::Reservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    fn discriminate(reservation: &InventoryReservation) -> InventoryDiscriminant {
        reservation.item.as_str().into()
    }
}

#[derive(Debug)]
pub struct InventoryReservation {
    item: String,
    quantity: AtomicU32,
}

#[derive(Debug, Error)]
pub enum InventoryReservationError {
    #[error("Quantity not available: {0}")]
    QuantityUnavailable(u32),
}

impl InventoryReservation {
    pub fn inc_quantity(&self, n: u32) {
        self.quantity.fetch_add(n, SeqCst);
    }

    pub fn dec_quantity(&self, n: u32) {
        let new = self.quantity().saturating_sub(n);
        self.quantity.store(new, SeqCst);
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

impl Reservation for InventoryReservation {
    fn quantity_atomic(&self) -> &AtomicU32 {
        &self.quantity
    }
}

impl Quantity for InventoryReservation {
    fn quantity(&self) -> u32 {
        self.quantity.load(SeqCst)
    }
}

impl Code for InventoryReservation {
    fn code(&self) -> &str {
        &self.item
    }
}
