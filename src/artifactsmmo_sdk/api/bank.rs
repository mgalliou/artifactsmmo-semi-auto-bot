use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        my_account_api::{
            get_bank_details_my_bank_get, get_bank_items_my_bank_items_get,
            GetBankDetailsMyBankGetError, GetBankItemsMyBankItemsGetError,
        },
        Error,
    },
    models::{BankResponseSchema, DataPageSimpleItemSchema},
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
        page: Option<i32>,
        size: Option<i32>,
    ) -> Result<DataPageSimpleItemSchema, Error<GetBankItemsMyBankItemsGetError>> {
        get_bank_items_my_bank_items_get(&self.configuration, code, page, size)
    }
}
