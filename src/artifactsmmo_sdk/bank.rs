use artifactsmmo_openapi::models::SimpleItemSchema;

use super::{account::Account, api::bank::BankApi, items::Items};

pub struct Bank {
    api: BankApi,
    items: Items,
}

impl Bank {
    pub fn new(account: &Account) -> Bank {
        Bank {
            api: BankApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
            items: Items::new(account),
        }
    }

    pub fn has_item(&self, code: &str) -> Option<SimpleItemSchema> {
        self.api
            .items(Some(code), None, None)
            .ok()?
            .data
            .first()
            .cloned()
    }

    ///. return the number of time the item `code` can be crafted with the mats available in bank
    pub fn has_mats_for(&self, code: &str) -> i32 {
        self.items
            .mats_for(code)
            .map(|mats| {
                mats.iter()
                    .map(|mat| {
                        self.has_item(&mat.code)
                            .map_or(0, |schema| schema.quantity / mat.quantity)
                    })
                    .min()
                    .unwrap_or(0)
            })
            .unwrap_or(0)
    }
}
