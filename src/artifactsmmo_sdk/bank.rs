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
        }
    }

    /// Returns the amount of the given item `code` available to the given `owner`.
    /// If no owner is given returns the total amount present in the bank.
    pub fn has_item(&self, code: &str, owner: Option<&str>) -> i32 {
        self.content
            .read()
            .unwrap()
            .iter()
            .find(|i| i.code == code)
            .map(|i| {
                if let Some(owner) = owner {
                    self.quantity_allowed(code, owner)
                } else {
                    i.quantity
                }
            })
            .unwrap_or(0)
    }

    /// Returns the total reservation quantity of the given `item`
    pub fn quantity_reserved(&self, item: &str) -> i32 {
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

    /// Returns the quantity the given `owner` can withdraw from the bank.
    pub fn quantity_allowed(&self, code: &str, owner: &str) -> i32 {
        max(
            0,
            self.has_item(code, None) - self.quantity_not_allowed(code, owner),
        )
    }

    /// Returns the quantity the given `owner` can't withdraw from the bank.
    pub fn quantity_not_allowed(&self, code: &str, owner: &str) -> i32 {
        self.reservations
            .read()
            .unwrap()
            .iter()
            .filter(|r| r.owner != owner && r.item == code)
            .map(|r| *r.quantity.read().unwrap())
            .sum()
    }

    /// return the number of time the item `code` can be crafted with the mats available in bank
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

    pub fn missing_mats_quantity(&self, code: &str, quantity: i32, owner: Option<&str>) -> i32 {
        self.missing_mats_for(code, quantity, owner)
            .iter()
            .map(|m| m.quantity)
            .sum()
    }

    pub fn update_content(&self, content: &Vec<SimpleItemSchema>) {
        self.content.write().unwrap().clone_from(content)
    }

    /// Request the `quantity` of the given `item` to be reserved to the player.
    /// If the reservation already exist increase the `quantity` of the reservation
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

    fn quantity_not_reserved(&self, item: &str) -> i32 {
        self.has_item(item, None) - self.quantity_reserved(item)
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
