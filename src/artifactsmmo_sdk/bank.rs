use super::{api::bank::BankApi, config::Config, items::Items};
use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};
use itertools::Itertools;
use log::info;
use std::{
    cmp::max,
    sync::{Arc, RwLock},
};

pub struct Bank {
    items: Arc<Items>,
    pub browsed: RwLock<()>,
    pub details: RwLock<BankSchema>,
    pub content: RwLock<Vec<SimpleItemSchema>>,
    pub reservations: RwLock<Vec<Reservation>>,
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

    pub fn has_item(&self, code: &str, owner: Option<&str>) -> i32 {
        self.content.read().map_or(0, |c| {
            c.iter()
                .find(|i| i.code == code)
                .map(|i| {
                    if let Some(owner) = owner {
                        self.quantity_allowed(code, owner)
                    } else {
                        i.quantity
                    }
                })
                .unwrap_or(0)
        })
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
        self.reservations.read().map_or(0, |r| {
            r.iter()
                .filter(|r| r.owner != owner && r.item == code)
                .map(|r| *r.quantity.read().unwrap())
                .sum()
        })
    }

    /// return the number of time the item `code` can be crafted with the mats available in bank
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
        if let Ok(mut c) = self.content.write() {
            c.clone_from(content)
        }
    }

    pub fn reserv(&self, item: &str, quantity: i32, owner: &str) {
        if let Ok(mut reservations) = self.reservations.write() {
            let res = Reservation {
                item: item.to_owned(),
                quantity: RwLock::new(quantity),
                owner: owner.to_owned(),
            };
            info!("adding reservation to bank: {:?}", res);
            reservations.push(res);
        }
    }

    pub fn update_reservations(&self, item: &str, quantity: i32, owner: &str) {
        if let Ok(mut reservations) = self.reservations.write() {
            let res = reservations
                .iter()
                .find(|r| r.item == item && r.owner == owner)
                .cloned();
            if let Some(res) = res {
                if *res.quantity.read().unwrap() <= quantity {
                    reservations.retain(|r| *r != res.clone());
                    info!("removed reservation from bank: {:?}", res);
                } else if let Ok(mut q) = res.quantity.write() {
                    *q -= quantity;
                    info!("updated quantity of reservation: {:?}", res);
                }
            }
        }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Reservation {
    item: String,
    quantity: RwLock<i32>,
    owner: String,
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
