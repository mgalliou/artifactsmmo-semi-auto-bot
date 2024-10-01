use super::{api::bank::BankApi, config::Config, items::Items};
use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};
use itertools::Itertools;
use std::sync::{Arc, RwLock};

pub struct Bank {
    items: Arc<Items>,
    pub details: RwLock<BankSchema>,
    pub content: RwLock<Vec<SimpleItemSchema>>,
}

impl Bank {
    pub fn new(config: &Config, items: Arc<Items>) -> Bank {
        let api = BankApi::new(&config.base_url, &config.token);
        Bank {
            items,
            details: RwLock::new(*api.details().unwrap().data),
            content: RwLock::new(api.items(None).unwrap()),
        }
    }

    pub fn has_item(&self, code: &str) -> i32 {
        self.content.read().map_or(0, |c| {
            c.iter()
                .find(|i| i.code == code)
                .map(|i| i.quantity)
                .unwrap_or(0)
        })
    }

    ///. return the number of time the item `code` can be crafted with the mats available in bank
    pub fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .map(|mat| self.has_item(&mat.code) / mat.quantity)
            .min()
            .unwrap_or(0)
    }

    pub fn missing_mats_for(&self, code: &str, quantity: i32) -> Vec<SimpleItemSchema> {
        self.items
            .mats(code)
            .into_iter()
            .filter(|m| self.has_item(&m.code) < m.quantity * quantity)
            .update(|m| m.quantity = m.quantity * quantity - self.has_item(&m.code))
            .collect_vec()
    }

    pub fn missing_mats_quantity(&self, code: &str, quantity: i32) -> i32 {
        self.missing_mats_for(code, quantity)
            .iter()
            .map(|m| m.quantity)
            .sum()
    }

    pub fn update_content(&self, content: &Vec<SimpleItemSchema>) {
        if let Ok(mut c) = self.content.write() {
            c.clone_from(content)
        }
    }
}
