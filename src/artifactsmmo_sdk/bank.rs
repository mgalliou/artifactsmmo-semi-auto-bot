use std::sync::Arc;

use artifactsmmo_openapi::models::{BankSchema, SimpleItemSchema};

use super::{account::Account, api::bank::BankApi, items::Items};

pub struct Bank {
    items: Arc<Items>,
    pub details: BankSchema,
    pub content: Vec<SimpleItemSchema>,
}

impl Bank {
    pub fn new(account: &Account, items: Arc<Items>) -> Bank {
        let api = BankApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Bank {
            items,
            details: *api.details().unwrap().data,
            content: api.items(None, None, None).unwrap(),
        }
    }

    pub fn has_item(&self, code: &str) -> Option<&SimpleItemSchema> {
        self.content.iter().find(|i| i.code == code)
    }

    ///. return the number of time the item `code` can be crafted with the mats available in bank
    pub fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats(code)
            .iter()
            .map(|mat| {
                self.has_item(&mat.code)
                    .map_or(0, |schema| schema.quantity / mat.quantity)
            })
            .min()
            .unwrap_or(0)
    }
}
