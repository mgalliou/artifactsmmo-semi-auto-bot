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

    pub fn has_mats_for(&self, code: &str) -> bool {
        let mats = self.items.mats_for(code).unwrap();
        for mat in mats {
            let schema = self.has_item(&mat.code);
            if !schema.is_some_and(|s| s.quantity >= mat.quantity) {
                return false;
            }
        }
        true
    }
}
