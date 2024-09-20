use std::sync::{Arc, RwLock};

use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};

use super::{account::Account, api::bank::BankApi, items::Items};

pub struct Bank {
    items: Arc<Items>,
    pub details: RwLock<BankSchema>,
    pub content: RwLock<Vec<SimpleItemSchema>>,
}

impl Bank {
    pub fn new(account: &Account, items: Arc<Items>) -> Bank {
        let api = BankApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
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

    pub fn update_content(&self, content: &Vec<SimpleItemSchema>) {
        if let Ok(mut c) = self.content.write() {
            c.clone_from(content)
        }
    }
}
