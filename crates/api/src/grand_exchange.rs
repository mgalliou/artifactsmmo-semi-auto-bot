use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        grand_exchange_api::{
            GetGeHistoryGrandexchangeHistoryCodeGetError, GetGeOrderGrandexchangeOrdersIdGetError,
            GetGeOrdersGrandexchangeOrdersGetError, get_ge_history_grandexchange_history_code_get,
            get_ge_order_grandexchange_orders_id_get, get_ge_orders_grandexchange_orders_get,
        },
    },
    models::{
        DataPageGeOrderHistorySchema, DataPageGeOrderSchema, GeOrderHistorySchema,
        GeOrderResponseSchema, GeOrderSchema,
    },
};
use std::{result::Result, sync::Arc, vec::Vec};

#[derive(Default, Debug)]
pub struct GrandExchangeApi {
    configuration: Arc<Configuration>,
}

impl GrandExchangeApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn sell_history(
        &self,
        item_code: &str,
    ) -> Result<Vec<GeOrderHistorySchema>, Error<GetGeHistoryGrandexchangeHistoryCodeGetError>>
    {
        SellHistoryRequest {
            configuration: &self.configuration,
            code: item_code,
        }
        .send()
    }

    pub fn sell_orders(
        &self,
    ) -> Result<Vec<GeOrderSchema>, Error<GetGeOrdersGrandexchangeOrdersGetError>> {
        SellOrdersRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_sell_order(
        &self,
        id: &str,
    ) -> Result<GeOrderResponseSchema, Error<GetGeOrderGrandexchangeOrdersIdGetError>> {
        get_ge_order_grandexchange_orders_id_get(&self.configuration, id)
    }
}

struct SellHistoryRequest<'a> {
    configuration: &'a Configuration,
    code: &'a str,
}

struct SellOrdersRequest<'a> {
    configuration: &'a Configuration,
}
impl<'a> Paginate for SellHistoryRequest<'a> {
    type Data = GeOrderHistorySchema;
    type Page = DataPageGeOrderHistorySchema;
    type Error = GetGeHistoryGrandexchangeHistoryCodeGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_ge_history_grandexchange_history_code_get(
            self.configuration,
            self.code,
            None,
            Some(current_page),
            Some(100),
        )
    }
}

impl DataPage<GeOrderHistorySchema> for DataPageGeOrderHistorySchema {
    fn data(self) -> Vec<GeOrderHistorySchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}

impl<'a> Paginate for SellOrdersRequest<'a> {
    type Data = GeOrderSchema;
    type Page = DataPageGeOrderSchema;
    type Error = GetGeOrdersGrandexchangeOrdersGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_ge_orders_grandexchange_orders_get(
            self.configuration,
            None,
            None,
            None,
            Some(page),
            Some(100),
        )
    }
}

impl DataPage<GeOrderSchema> for DataPageGeOrderSchema {
    fn data(self) -> Vec<GeOrderSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
