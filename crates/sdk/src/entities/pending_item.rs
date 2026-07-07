use openapi::models::{PendingItemSchema, SimpleItemSchema};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingItemDataHandle(Arc<RwLock<PendingItem>>);

impl PendingItemDataHandle {
    #[must_use]
    pub fn read(&self) -> PendingItem {
        self.0.read().unwrap().clone()
    }

    pub fn update(&self, data: PendingItem) {
        *self.0.write().unwrap() = data;
    }
}

impl From<PendingItemSchema> for PendingItemDataHandle {
    fn from(value: PendingItemSchema) -> Self {
        Self(Arc::new(RwLock::new(PendingItem(Arc::new(value)))))
    }
}

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

    #[must_use]
    pub fn is_claimed(&self) -> bool {
        self.0.claimed_at.is_some()
    }
}
