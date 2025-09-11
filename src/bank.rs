use artifactsmmo_sdk::{
    ItemContainer, Items, SlotLimited,
    bank::Bank as BankClient,
    items::ItemSchemaExt,
    models::{BankSchema, ItemSchema, SimpleItemSchema},
};
use itertools::Itertools;
use log::debug;
use std::{
    cmp::max,
    fmt::{self, Display, Formatter},
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
};
use thiserror::Error;

use crate::FOOD_BLACK_LIST;

#[derive(Default)]
pub struct BankController {
    client: Arc<BankClient>,
    items: Arc<Items>,
    pub reservations: RwLock<Vec<Arc<Reservation>>>,
    pub browsed: RwLock<()>,
    pub being_expanded: RwLock<()>,
}

impl BankController {
    pub fn new(client: Arc<BankClient>, items: Arc<Items>) -> Self {
        Self {
            client,
            items,
            reservations: RwLock::new(vec![]),
            browsed: RwLock::new(()),
            being_expanded: RwLock::new(()),
        }
    }

    pub fn details(&self) -> Arc<BankSchema> {
        self.client.details()
    }

    pub fn is_full(&self) -> bool {
        self.free_slots() == 0
    }

    pub fn gold(&self) -> u32 {
        self.client.gold()
    }

    pub fn next_expansion_cost(&self) -> u32 {
        self.client.details.read().unwrap().next_expansion_cost
    }

    /// Returns the quantity of the given item `code` that can be crafted with the mats available in bank
    /// for the given `owner`.
    //  NOTE: this should maybe return a Option to indicate that the item is not craftable and
    //  return None in this case
    pub fn has_mats_for(&self, item: &str, owner: Option<&str>) -> u32 {
        self.items
            .mats_of(item)
            .iter()
            .map(|mat| self.has_available(&mat.code, owner) / mat.quantity)
            .min()
            .unwrap_or(0)
    }

    /// Returns the quantity of each of the missing materials required to craft the `quantity` of the  item `code`
    /// for the given `owner`.
    pub fn missing_mats_for(
        &self,
        item: &str,
        quantity: u32,
        owner: Option<&str>,
    ) -> Vec<SimpleItemSchema> {
        self.items
            .mats_of(item)
            .into_iter()
            .filter(|m| self.has_available(&m.code, owner) < m.quantity * quantity)
            .update(|m| {
                m.quantity =
                    (m.quantity * quantity).saturating_sub(self.has_available(&m.code, owner))
            })
            .collect_vec()
    }

    /// Returns the total quantity of the missing materials required to craft the `quantity` of the
    /// item `code` for the given `owner`
    pub fn missing_mats_quantity(&self, item: &str, quantity: u32, owner: Option<&str>) -> u32 {
        self.missing_mats_for(item, quantity, owner)
            .iter()
            .map(|m| m.quantity)
            .sum()
    }

    pub fn consumable_food(&self, level: u32) -> Vec<Arc<ItemSchema>> {
        self.content()
            .iter()
            .filter_map(|i| {
                self.items.get(&i.code).filter(|i| {
                    i.is_food() && i.level <= level && !FOOD_BLACK_LIST.contains(&i.code.as_str())
                })
            })
            .collect_vec()
    }

    pub fn has_multiple_available(&self, items: &[SimpleItemSchema], owner: &str) -> bool {
        items
            .iter()
            .all(|i| self.has_available(&i.code, Some(owner)) >= i.quantity)
    }

    /// Returns the `quantity` of the given item `code` available to the given `owner`.
    /// If no owner is given returns the quantity not reserved.
    pub fn has_available(&self, item: &str, owner: Option<&str>) -> u32 {
        if let Some(owner) = owner {
            self.quantity_allowed(item, owner)
        } else {
            self.quantity_reservable(item)
        }
    }

    pub fn reserv_items(&self, items: &[SimpleItemSchema], owner: &str) -> Result<(), BankError> {
        for m in items.iter() {
            self.reserv_item(&m.code, m.quantity, owner)?
        }
        Ok(())
    }

    /// Make sure that the `quantity` of `item` is reserved to the `owner`.
    /// Create the reservation if possible. Increase the reservation quantity if
    /// necessary and possible.
    pub fn reserv_item(&self, item: &str, quantity: u32, owner: &str) -> Result<(), BankError> {
        let Some(res) = self.get_reservation(item, owner) else {
            return self.increase_reservation(item, quantity, owner);
        };
        let quantity_to_reserv = quantity.saturating_sub(res.quantity());
        if quantity_to_reserv == 0 {
            return Ok(());
        } else if quantity_to_reserv > self.quantity_reservable(item) {
            return Err(BankError::QuantityUnavailable(quantity));
        };
        res.inc_quantity(quantity_to_reserv);
        debug!("bank: increased '{item}' reservation by '{quantity_to_reserv}'",);
        Ok(())
    }

    /// Request the `quantity` of the given `item` to be added to exising reservation for the the given `owner`.
    /// Create the reservation if it does not exist.
    pub fn increase_reservation(
        &self,
        item: &str,
        quantity: u32,
        owner: &str,
    ) -> Result<(), BankError> {
        let Some(res) = self.get_reservation(item, owner) else {
            if quantity > self.has_available(item, Some(owner)) {
                return Err(BankError::QuantityUnavailable(quantity));
            }
            self.add_reservation(item, quantity, owner);
            return Ok(());
        };
        if quantity > self.quantity_reservable(item) {
            return Err(BankError::QuantityUnavailable(quantity));
        }
        res.inc_quantity(quantity);
        Ok(())
    }

    pub fn unreserv_items(&self, items: &[SimpleItemSchema], owner: &str) {
        for item in items.iter() {
            self.unreserv_item(&item.code, item.quantity, owner);
        }
    }

    pub fn unreserv_item(&self, item: &str, quantity: u32, owner: &str) {
        let Some(res) = self.get_reservation(item, owner) else {
            return;
        };
        if quantity >= res.quantity() {
            self.remove_reservation(&res)
        } else {
            res.dec_quantity(quantity);
            debug!("bank: decreased reservation by '{quantity}': {res}",);
        }
    }

    fn add_reservation(&self, item: &str, quantity: u32, owner: &str) {
        let res = Arc::new(Reservation {
            item: item.to_owned(),
            quantity: AtomicU32::new(quantity),
            owner: owner.to_owned(),
        });
        self.reservations.write().unwrap().push(res.clone());
        debug!("bank: added reservation: {res}");
    }

    fn remove_reservation(&self, reservation: &Reservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        debug!("bank: removed reservation: {reservation}");
    }

    pub fn reservations(&self) -> Vec<Arc<Reservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    /// Returns the quantity the given `owner` can withdraw from the bank.
    fn quantity_allowed(&self, item: &str, owner: &str) -> u32 {
        max(
            0,
            self.total_of(item)
                .saturating_sub(self.quantity_not_allowed(item, owner)),
        )
    }

    /// Returns the quantity of the given item `code` that is reserved to a different character
    /// than the given `owner`.
    fn quantity_not_allowed(&self, item: &str, owner: &str) -> u32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.owner != owner && r.item == item)
            .map(|r| r.quantity())
            .sum()
    }

    /// Returns the total quantity of the given `item` that is reserved by any character.
    fn quantity_reserved(&self, item: &str) -> u32 {
        self.reservations()
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

    fn quantity_reservable(&self, item: &str) -> u32 {
        self.total_of(item)
            .saturating_sub(self.quantity_reserved(item))
    }

    fn get_reservation(&self, item: &str, owner: &str) -> Option<Arc<Reservation>> {
        self.reservations()
            .into_iter()
            .find(|r| r.item == item && r.owner == owner)
    }
}

impl SlotLimited for BankController {}

impl ItemContainer for BankController {
    type Slot = SimpleItemSchema;

    fn content(&self) -> Arc<Vec<SimpleItemSchema>> {
        self.client.content()
    }

    fn total_items(&self) -> u32 {
        self.client.total_items()
    }

    fn total_of(&self, item: &str) -> u32 {
        self.client.total_of(item)
    }

    fn contains_multiple(&self, items: &[SimpleItemSchema]) -> bool {
        self.client.contains_multiple(items)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum BankError {
    #[error("Item unvailable")]
    ItemUnavailable,
    #[error("Quantity unavailable: {0}")]
    QuantityUnavailable(u32),
}

#[derive(Debug)]
pub struct Reservation {
    owner: String,
    item: String,
    quantity: AtomicU32,
}

impl Reservation {
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

impl Display for Reservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: '{}'x{}", self.owner, self.item, self.quantity(),)
    }
}

impl Clone for Reservation {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            quantity: AtomicU32::new(self.quantity()),
            owner: self.owner.clone(),
        }
    }
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item && self.quantity() == other.quantity() && self.owner == other.owner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserv_with_not_item() {
        let bank = BankController::default();
        let result = bank.increase_reservation("iron_ore", 50, "char1");
        assert_eq!(Err(BankError::QuantityUnavailable(50)), result);
    }

    #[test]
    fn reserv_with_item_available() {
        let bank = BankController::default();
        *bank.client.content.write().unwrap() = Arc::new(vec![SimpleItemSchema {
            code: "copper_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.increase_reservation("copper_ore", 50, "char1");
        let _ = bank.increase_reservation("copper_ore", 50, "char1");
        assert_eq!(100, bank.has_available("copper_ore", Some("char1")))
    }

    #[test]
    fn reserv_if_not_with_item_available() {
        let bank = BankController::default();
        *bank.client.content.write().unwrap() = Arc::new(vec![SimpleItemSchema {
            code: "gold_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.reserv_item("gold_ore", 50, "char1");
        let _ = bank.reserv_item("gold_ore", 50, "char1");
        assert_eq!(100, bank.has_available("gold_ore", Some("char1")));
        assert_eq!(50, bank.quantity_reserved("gold_ore"))
    }
}
