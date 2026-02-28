use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        npcs_api::{
            GetAllNpcsItemsNpcsItemsGetError, GetAllNpcsNpcsDetailsGetError,
            get_all_npcs_items_npcs_items_get, get_all_npcs_npcs_details_get,
        },
    },
    models::{NpcItem, NpcSchema, StaticDataPageNpcItem, StaticDataPageNpcSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct NpcsApi {
    configuration: Arc<Configuration>,
}

impl NpcsApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<NpcSchema>, Error<GetAllNpcsNpcsDetailsGetError>> {
        NpcsRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_items(&self) -> Result<Vec<NpcItem>, Error<GetAllNpcsItemsNpcsItemsGetError>> {
        NpcsItemsRequest {
            configuration: &self.configuration,
        }
        .send()
    }
}

struct NpcsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for NpcsRequest<'a> {
    type Data = NpcSchema;
    type Page = StaticDataPageNpcSchema;
    type Error = GetAllNpcsNpcsDetailsGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_npcs_npcs_details_get(self.configuration, None, None, Some(page), Some(100))
    }
}

impl DataPage<NpcSchema> for StaticDataPageNpcSchema {
    fn data(self) -> Vec<NpcSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}

struct NpcsItemsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for NpcsItemsRequest<'a> {
    type Data = NpcItem;
    type Page = StaticDataPageNpcItem;
    type Error = GetAllNpcsItemsNpcsItemsGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_npcs_items_npcs_items_get(
            self.configuration,
            None,
            None,
            None,
            Some(page),
            Some(100),
        )
    }
}

impl DataPage<NpcItem> for StaticDataPageNpcItem {
    fn data(self) -> Vec<NpcItem> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
