use crate::Code;
use openapi::models::{NpcSchema, NpcType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Npc(Arc<NpcSchema>);

impl Npc {
    #[must_use]
    pub(crate) fn new(schema: NpcSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    #[must_use]
    pub fn r#type(&self) -> NpcType {
        self.0.r#type
    }

    #[must_use]
    pub fn is_merchant(&self) -> bool {
        self.r#type() == NpcType::Merchant
    }

    #[must_use]
    pub fn is_trader(&self) -> bool {
        self.r#type() == NpcType::Trader
    }
}

impl Code for Npc {
    fn code(&self) -> &str {
        &self.0.code
    }
}
