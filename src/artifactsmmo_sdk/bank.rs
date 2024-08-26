use super::{account::Account, api::bank::BankApi};

pub struct Bank {
    api: BankApi
}

impl Bank {
    pub fn new(account: &Account) -> Bank {
        Bank {
            api: BankApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn has_item(&self, code: &str) -> bool {
        self.api.items(Some(code), None, None).is_ok()
    }
}
