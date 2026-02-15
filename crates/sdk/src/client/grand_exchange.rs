use api::ArtifactApi;
use openapi::models::{GeOrderHistorySchema, GeOrderSchema};
use itertools::Itertools;
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct GrandExchangeClient {
    api: Arc<ArtifactApi>,
}

impl GrandExchangeClient {
    pub(crate) fn new(api: Arc<ArtifactApi>) -> Self {
        Self { api }
    }

    pub fn sell_history(&self, item_code: &str) -> Option<Vec<GeOrderHistorySchema>> {
        self.api.grand_exchange.sell_history(item_code).ok()
    }

    pub fn sell_orders(&self) -> Vec<GeOrderSchema> {
        self.api
            .grand_exchange
            .sell_orders()
            .into_iter()
            .flatten()
            .collect_vec()
    }

    pub fn get_order_by_id(&self, id: &str) -> Option<GeOrderSchema> {
        self.api
            .grand_exchange
            .get_sell_order(id)
            .map(|r| *r.data)
            .ok()
    }

    // pub fn refresh_orders(&self) {
    //     *self.sell_orders.write().unwrap() = self
    //         .api
    //         .grand_exchange
    //         .sell_orders()
    //         .unwrap()
    //         .into_iter()
    //         .map(Arc::new)
    //         .collect_vec()
    // }
}
