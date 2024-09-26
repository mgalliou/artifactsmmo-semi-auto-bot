use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        my_account_api::{
            get_bank_details_my_bank_get, get_bank_items_my_bank_items_get,
            GetBankDetailsMyBankGetError, GetBankItemsMyBankItemsGetError,
        },
        Error,
    },
    models::{BankResponseSchema, SimpleItemSchema},
};

pub struct BankApi {
    configuration: Configuration,
}

impl BankApi {
    pub fn new(base_path: &str, token: &str) -> BankApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        BankApi { configuration }
    }

    pub fn details(&self) -> Result<BankResponseSchema, Error<GetBankDetailsMyBankGetError>> {
        get_bank_details_my_bank_get(&self.configuration)
    }

    pub fn items(
        &self,
        code: Option<&str>,
    ) -> Result<Vec<SimpleItemSchema>, Error<GetBankItemsMyBankItemsGetError>> {
        let mut items: Vec<SimpleItemSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_bank_items_my_bank_items_get(
                &self.configuration,
                code,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    items.extend(resp.data);
                    if let Some(Some(pages)) = resp.pages {
                        if current_page >= pages {
                            finished = true
                        }
                        current_page += 1;
                    } else {
                        // No pagination information, assume single page
                        finished = true
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(items)
    }
}
