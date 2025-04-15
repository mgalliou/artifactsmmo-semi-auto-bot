use artifactsmmo_sdk::{
    base_bank::BaseBank,
    items::ItemSchemaExt,
    models::{BankSchema, ItemSchema, SimpleItemSchema},
    ITEMS,
};
use itertools::Itertools;
use log::info;
use std::{
    cmp::max,
    fmt::{self, Display, Formatter},
    sync::{Arc, LazyLock, RwLock},
};

pub static BANK: LazyLock<Bank> = LazyLock::new(Bank::new);

#[derive(Default)]
pub struct Bank {
    inner: BaseBank,
    pub browsed: RwLock<()>,
    pub being_expanded: RwLock<()>,
    pub reservations: RwLock<Vec<Arc<Reservation>>>,
}

impl Bank {
    pub fn new() -> Self {
        Self {
            browsed: RwLock::new(()),
            inner: BaseBank::new(),
            reservations: RwLock::new(vec![]),
            being_expanded: RwLock::new(()),
        }
    }

    pub fn details(&self) -> Arc<BankSchema> {
        self.inner.details()
    }

    pub fn content(&self) -> Arc<Vec<SimpleItemSchema>> {
        self.inner.content()
    }

    pub fn is_full(&self) -> bool {
        self.free_slots() <= 0
    }

    pub fn free_slots(&self) -> i32 {
        self.inner.details.read().unwrap().slots - self.inner.content.read().unwrap().len() as i32
    }

    pub fn gold(&self) -> i32 {
        self.inner.details.read().unwrap().gold
    }

    pub fn next_expansion_cost(&self) -> i32 {
        self.inner.details.read().unwrap().next_expansion_cost
    }

    /// Returns the total quantity of the given `item` code currently in the bank.
    pub fn total_of(&self, item: &str) -> i32 {
        self.inner
            .content
            .read()
            .unwrap()
            .iter()
            .find_map(|i| {
                if i.code == item {
                    Some(i.quantity)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    /// Returns the quantity of the given item `code` that can be crafted with the mats available in bank
    /// for the given `owner`.
    //  NOTE: this should maybe return a Option to indicate that the item is not craftable and
    //  return None in this case
    pub fn has_mats_for(&self, item: &str, owner: Option<&str>) -> i32 {
        ITEMS
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
        quantity: i32,
        owner: Option<&str>,
    ) -> Vec<SimpleItemSchema> {
        ITEMS
            .mats_of(item)
            .into_iter()
            .filter(|m| self.has_available(&m.code, owner) < m.quantity * quantity)
            .update(|m| m.quantity = m.quantity * quantity - self.has_available(&m.code, owner))
            .collect_vec()
    }

    /// Returns the total quantity of the missing materials required to craft the `quantity` of the
    /// item `code` for the given `owner`
    pub fn missing_mats_quantity(&self, item: &str, quantity: i32, owner: Option<&str>) -> i32 {
        self.missing_mats_for(item, quantity, owner)
            .iter()
            .map(|m| m.quantity)
            .sum()
    }

    pub fn consumable_food(&self, level: i32) -> Vec<Arc<ItemSchema>> {
        self.inner
            .content
            .read()
            .unwrap()
            .iter()
            .filter_map(|i| ITEMS.get(&i.code).filter(|i| i.is_consumable_at(level)))
            .collect_vec()
    }

    /// Returns the `quantity` of the given item `code` available to the given `owner`.
    /// If no owner is given returns the quantity not reserved.
    pub fn has_available(&self, item: &str, owner: Option<&str>) -> i32 {
        if let Some(owner) = owner {
            self.quantity_allowed(item, owner)
        } else {
            self.quantity_not_reserved(item)
        }
    }

    /// Make sure that the `quantity` of `item` is reserved to the `owner`.
    /// Create the reservation if possible. Increase the reservation quantity if
    /// necessary and possible.
    pub fn reserv(&self, item: &str, quantity: i32, owner: &str) -> Result<(), BankError> {
        let Some(res) = self.get_reservation(item, owner) else {
            return self.increase_reservation(item, quantity, owner);
        };
        if res.quantity() >= quantity {
            Ok(())
        } else if self.quantity_not_reserved(item) >= quantity - res.quantity() {
            res.inc_quantity(quantity - res.quantity());
            info!(
                "bank: increased reservation quantity by '{}': [{}]",
                quantity, res
            );
            Ok(())
        } else {
            Err(BankError::QuantityUnavailable(quantity))
        }
    }

    /// Request the `quantity` of the given `item` to be added to exising reservation for the the given `owner`.
    /// Create the reservation if it does not exist.
    pub fn increase_reservation(
        &self,
        item: &str,
        quantity: i32,
        owner: &str,
    ) -> Result<(), BankError> {
        let Some(res) = self.get_reservation(item, owner) else {
            if quantity > self.has_available(item, Some(owner)) {
                return Err(BankError::QuantityUnavailable(quantity));
            }
            self.add_reservation(item, quantity, owner);
            return Ok(());
        };
        if quantity > self.quantity_not_reserved(item) {
            return Err(BankError::QuantityUnavailable(quantity));
        }
        res.inc_quantity(quantity);
        Ok(())
    }

    pub fn decrease_reservation(&self, item: &str, quantity: i32, owner: &str) {
        let Some(res) = self.get_reservation(item, owner) else {
            return;
        };
        if quantity >= res.quantity() {
            self.remove_reservation(&res)
        } else {
            res.dec_quantity(quantity);
            info!(
                "bank: decreased reservation quantity by '{}': [{}]",
                quantity, res
            );
        }
    }

    fn add_reservation(&self, item: &str, quantity: i32, owner: &str) {
        let res = Arc::new(Reservation {
            item: item.to_owned(),
            quantity: RwLock::new(quantity),
            owner: owner.to_owned(),
        });
        self.reservations.write().unwrap().push(res.clone());
        info!("bank: added reservation to bank: {}", res);
    }

    fn remove_reservation(&self, reservation: &Reservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        info!("bank: removed reservation: {}", reservation);
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
    fn quantity_allowed(&self, item: &str, owner: &str) -> i32 {
        max(
            0,
            self.total_of(item) - self.quantity_not_allowed(item, owner),
        )
    }

    /// Returns the quantity of the given item `code` that is reserved to a different character
    /// than the given `owner`.
    fn quantity_not_allowed(&self, item: &str, owner: &str) -> i32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.owner != owner && r.item == item)
            .map(|r| r.quantity())
            .sum()
    }

    /// Returns the total quantity of the given `item` that is reserved by any character.
    fn quantity_reserved(&self, item: &str) -> i32 {
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

    fn quantity_not_reserved(&self, item: &str) -> i32 {
        self.total_of(item) - self.quantity_reserved(item)
    }

    fn get_reservation(&self, item: &str, owner: &str) -> Option<Arc<Reservation>> {
        self.reservations()
            .into_iter()
            .find(|r| r.item == item && r.owner == owner)
    }
}

#[derive(Debug, PartialEq)]
pub enum BankError {
    ItemUnavailable,
    QuantityUnavailable(i32),
}

#[derive(Debug)]
pub struct Reservation {
    owner: String,
    item: String,
    quantity: RwLock<i32>,
}

impl Reservation {
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

impl Display for Reservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: '{}'x{}", self.owner, self.item, self.quantity(),)
    }
}

impl Clone for Reservation {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            quantity: RwLock::new(self.quantity()),
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
        let bank = Bank::default();
        let result = bank.increase_reservation("iron_ore", 50, "char1");
        assert_eq!(Err(BankError::QuantityUnavailable(50)), result);
    }

    #[test]
    fn reserv_with_item_available() {
        let bank = Bank::default();
        *bank.inner.content.write().unwrap() = Arc::new(vec![SimpleItemSchema {
            code: "copper_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.increase_reservation("copper_ore", 50, "char1");
        let _ = bank.increase_reservation("copper_ore", 50, "char1");
        assert_eq!(100, bank.has_available("copper_ore", Some("char1")))
    }

    #[test]
    fn reserv_if_not_with_item_available() {
        let bank = Bank::default();
        *bank.inner.content.write().unwrap() = Arc::new(vec![SimpleItemSchema {
            code: "gold_ore".to_owned(),
            quantity: 100,
        }]);
        let _ = bank.reserv("gold_ore", 50, "char1");
        let _ = bank.reserv("gold_ore", 50, "char1");
        assert_eq!(100, bank.has_available("gold_ore", Some("char1")));
        assert_eq!(50, bank.quantity_reserved("gold_ore"))
    }
}
