use crate::Code;
use openapi::models;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NpcItem(Arc<models::NpcItemSchema>);

impl NpcItem {
    #[must_use]
    pub(crate) fn new(schema: models::NpcItemSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn npc_code(&self) -> &str {
        &self.0.npc
    }

    #[must_use]
    pub fn currency(&self) -> &str {
        &self.0.currency
    }

    #[must_use]
    pub fn buy_price(&self) -> Option<u32> {
        self.0.buy_price.map(|p| p as u32)
    }

    #[must_use]
    pub fn sell_price(&self) -> Option<u32> {
        self.0.sell_price.map(|p| p as u32)
    }

    #[must_use]
    pub fn is_buyable(&self) -> bool {
        self.buy_price().is_some()
    }

    #[must_use]
    pub fn is_salable(&self) -> bool {
        self.sell_price().is_some()
    }
}

impl Code for NpcItem {
    fn code(&self) -> &str {
        &self.0.code
    }
}
