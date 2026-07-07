use openapi::models::{PendingItemSchema, SimpleItemSchema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingItem(Arc<PendingItemSchema>);

impl PendingItem {
    #[must_use]
    pub fn new(pending_item_schema: PendingItemSchema) -> Self {
        Self(Arc::new(pending_item_schema))
    }

    #[must_use]
    pub fn id(&self) -> &String {
        &self.0.id
    }

    #[must_use]
    pub fn items(&self) -> &[SimpleItemSchema] {
        self.0.items.as_deref().unwrap_or_default()
    }
}
