use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        my_account_api::{
            GetBankDetailsMyBankGetError, GetBankItemsMyBankItemsGetError,
            get_bank_details_my_bank_get, get_bank_items_my_bank_items_get,
        },
    },
    models::{BankResponseSchema, DataPageSimpleItemSchema, SimpleItemSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct BankApi {
    configuration: Arc<Configuration>,
}

impl BankApi {
    pub fn new(configuration: Arc<Configuration>) -> Self {
        BankApi { configuration }
    }

    pub fn get_items(
        &self,
    ) -> Result<Vec<SimpleItemSchema>, Error<GetBankItemsMyBankItemsGetError>> {
        BankItemsRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_details(&self) -> Result<BankResponseSchema, Error<GetBankDetailsMyBankGetError>> {
        get_bank_details_my_bank_get(&self.configuration)
    }
}

struct BankItemsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for BankItemsRequest<'a> {
    type Data = SimpleItemSchema;
    type Page = DataPageSimpleItemSchema;
    type Error = GetBankItemsMyBankItemsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_bank_items_my_bank_items_get(self.configuration, None, Some(current_page), Some(100))
    }
}

impl DataPage<SimpleItemSchema> for DataPageSimpleItemSchema {
    fn data(self) -> Vec<SimpleItemSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
