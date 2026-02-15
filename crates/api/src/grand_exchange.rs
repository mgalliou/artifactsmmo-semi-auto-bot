use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        grand_exchange_api::{
            GetGeSellHistoryGrandexchangeHistoryCodeGetError,
            GetGeSellOrderGrandexchangeOrdersIdGetError,
            GetGeSellOrdersGrandexchangeOrdersGetError,
            get_ge_sell_history_grandexchange_history_code_get,
            get_ge_sell_order_grandexchange_orders_id_get,
            get_ge_sell_orders_grandexchange_orders_get,
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
    ) -> Result<Vec<GeOrderHistorySchema>, Error<GetGeSellHistoryGrandexchangeHistoryCodeGetError>>
    {
        SellHistoryRequest {
            configuration: &self.configuration,
            code: item_code,
        }
        .send()
    }

    pub fn sell_orders(
        &self,
    ) -> Result<Vec<GeOrderSchema>, Error<GetGeSellOrdersGrandexchangeOrdersGetError>> {
        SellOrdersRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_sell_order(
        &self,
        id: &str,
    ) -> Result<GeOrderResponseSchema, Error<GetGeSellOrderGrandexchangeOrdersIdGetError>> {
        get_ge_sell_order_grandexchange_orders_id_get(&self.configuration, id)
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
    type Error = GetGeSellHistoryGrandexchangeHistoryCodeGetError;

    fn request_page(&self, current_page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_ge_sell_history_grandexchange_history_code_get(
            self.configuration,
            self.code,
            None,
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
    type Error = GetGeSellOrdersGrandexchangeOrdersGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_ge_sell_orders_grandexchange_orders_get(
            self.configuration,
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
