use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        items_api::{GetAllItemsItemsGetError, get_all_items_items_get},
    },
    models::{ItemSchema, StaticDataPageItemSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct ItemsApi {
    configuration: Arc<Configuration>,
}

impl ItemsApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<ItemSchema>, Error<GetAllItemsItemsGetError>> {
        ItemsRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct ItemsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for ItemsRequest<'a> {
    type Data = ItemSchema;
    type Page = StaticDataPageItemSchema;
    type Error = GetAllItemsItemsGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_items_items_get(
            self.configuration,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(current_page),
            Some(100),
        )
    }
}

impl DataPage<ItemSchema> for StaticDataPageItemSchema {
    fn data(self) -> Vec<ItemSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
