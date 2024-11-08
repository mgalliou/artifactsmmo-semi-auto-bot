use super::{api::bank::BankApi, config::Config, items::Items};
use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};
use itertools::Itertools;
use log::info;
use std::{
    cmp::max,
    fmt::{self, Display, Formatter},
    sync::{Arc, RwLock},
};

pub struct Bank {
    items: Arc<Items>,
    pub browsed: RwLock<()>,
    pub details: RwLock<BankSchema>,
    pub content: RwLock<Vec<SimpleItemSchema>>,
    pub reservations: RwLock<Vec<Arc<Reservation>>>,
    pub being_expanded: RwLock<()>,
}

impl Bank {
    pub fn new(config: &Config, items: &Arc<Items>) -> Bank {
        let api = BankApi::new(&config.base_url, &config.token);
        Bank {
            items: items.clone(),
            browsed: RwLock::new(()),
            details: RwLock::new(*api.details().unwrap().data),
            content: RwLock::new(api.items(None).unwrap()),
            reservations: RwLock::new(vec![]),
            being_expanded: RwLock::new(()),
        }
    }

    pub fn is_full(&self) -> bool {
        self.free_slots() <= 0
    }

    pub fn free_slots(&self) -> i32 {
        self.details.read().unwrap().slots - self.content.read().unwrap().len() as i32
    }

    pub fn gold(&self) -> i32 {
        self.details.read().unwrap().gold
    }

    pub fn next_expansion_cost(&self) -> i32 {
        self.details.read().unwrap().next_expansion_cost
    }

    /// Returns the total quantity of the given `item` code currently in the bank.
    fn total_of(&self, item: &str) -> i32 {
        self.content
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

    /// Returns the `quantity` of the given item `code` available to the given `owner`.
    /// If no owner is given returns the quantity not reserved.
    pub fn has_item(&self, code: &str, owner: Option<&str>) -> i32 {
        if let Some(owner) = owner {
            self.quantity_allowed(code, owner)
        } else {
            self.quantity_not_reserved(code)
        }
    }

    /// Returns the quantity the given `owner` can withdraw from the bank.
    fn quantity_allowed(&self, code: &str, owner: &str) -> i32 {
        max(
            0,
            self.total_of(code) - self.quantity_not_allowed(code, owner),
        )
    }

    /// Returns the quantity of the given item `code` that is reserved to a different character
    /// than the given `owner`.
    fn quantity_not_allowed(&self, code: &str, owner: &str) -> i32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.owner != owner && r.item == code)
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

    /// Returns the quantity of the given item `code` that can be crafted with the mats available in bank
    /// for the given `owner`.
    //  NOTE: this should maybe return a Option to indicate that the item is not craftable and
    //  return None in this case
    pub fn has_mats_for(&self, code: &str, owner: Option<&str>) -> i32 {
        self.items
            .mats(code)
            .iter()
            .map(|mat| self.has_item(&mat.code, owner) / mat.quantity)
            .min()
            .unwrap_or(0)
    }

    /// Returns the quantity of each of the missing materials required to craft the `quantity` of the  item `code`
    /// for the given `owner`.
    pub fn missing_mats_for(
        &self,
        code: &str,
        quantity: i32,
        owner: Option<&str>,
    ) -> Vec<SimpleItemSchema> {
        self.items
            .mats(code)
            .into_iter()
            .filter(|m| self.has_item(&m.code, owner) < m.quantity * quantity)
            .update(|m| m.quantity = m.quantity * quantity - self.has_item(&m.code, owner))
            .collect_vec()
    }

    /// Returns the total quantity of the missing materials required to craft the `quantity` of the
    /// item `code` for the given `owner`
    pub fn missing_mats_quantity(&self, code: &str, quantity: i32, owner: Option<&str>) -> i32 {
        self.missing_mats_for(code, quantity, owner)
            .iter()
            .map(|m| m.quantity)
            .sum()
    }

    pub fn update_content(&self, content: &Vec<SimpleItemSchema>) {
        self.content.write().unwrap().clone_from(content)
    }

    /// Request the `quantity` of the given `item` to be reserved for the the given `owner`.
    /// If the reservation already exist increase the `quantity` of the reservation.
    // NOTE: should fail if the item are not available
    pub fn reserv(&self, item: &str, quantity: i32, owner: &str) -> Result<(), BankError> {
        if let Some(res) = self.get_reservation(owner, item) {
            if quantity > self.quantity_not_reserved(item) {
                return Err(BankError::ItemUnavailable);
            }
            res.inc_quantity(quantity);
        } else {
            if quantity > self.has_item(item, Some(owner)) {
                return Err(BankError::ItemUnavailable);
            }
            let res = Arc::new(Reservation {
                item: item.to_owned(),
                quantity: RwLock::new(quantity),
                owner: owner.to_owned(),
            });
            self.reservations.write().unwrap().push(res.clone());
            info!("added reservation to bank: {}", res);
        }
        Ok(())
    }

    pub fn decrease_reservation(&self, item: &str, quantity: i32, owner: &str) {
        if let Some(res) = self.get_reservation(owner, item) {
            if quantity >= *res.quantity.read().unwrap() {
                self.remove_reservation(&res)
            } else {
                res.dec_quantity(quantity)
            }
        }
    }

    pub fn increase_reservation(&self, item: &str, quantity: i32, owner: &str) {
        if let Some(res) = self.get_reservation(owner, item) {
            res.inc_quantity(quantity)
        }
    }

    pub fn remove_reservation(&self, reservation: &Reservation) {
        self.reservations
            .write()
            .unwrap()
            .retain(|r| **r != *reservation);
        info!("removed reservation from bank: {}", reservation);
    }

    pub fn reservations(&self) -> Vec<Arc<Reservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    pub fn get_reservation(&self, owner: &str, item: &str) -> Option<Arc<Reservation>> {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .find(|r| r.item == item && r.owner == owner)
            .cloned()
    }
}

pub enum BankError {
    ItemUnavailable,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Reservation {
    owner: String,
    item: String,
    quantity: RwLock<i32>,
}

impl Reservation {
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

impl Display for Reservation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: '{}'x{}",
            self.owner,
            self.item,
            self.quantity.read().unwrap(),
        )
    }
}

impl Clone for Reservation {
    fn clone(&self) -> Self {
        Self {
            item: self.item.clone(),
            quantity: RwLock::new(*self.quantity.read().unwrap()),
            owner: self.owner.clone(),
        }
    }
}

impl PartialEq for Reservation {
    fn eq(&self, other: &Self) -> bool {
        self.item == other.item
            && *self.quantity.read().unwrap() == *other.quantity.read().unwrap()
            && self.owner == other.owner
    }
}
