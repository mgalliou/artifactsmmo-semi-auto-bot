use openapi::models::{PendingItemSchema, SimpleItemSchema};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingItemHandle(Arc<RwLock<RawPendingItem>>);

impl PendingItemHandle {
    #[must_use]
    pub fn read(&self) -> RawPendingItem {
        self.0.read().unwrap().clone()
    }

    pub fn update(&self, data: RawPendingItem) {
        *self.0.write().unwrap() = data;
    }
}

impl From<PendingItemSchema> for PendingItemHandle {
    fn from(value: PendingItemSchema) -> Self {
        Self(Arc::new(RwLock::new(value.into())))
    }
}

impl From<&PendingItemSchema> for PendingItemHandle {
    fn from(value: &PendingItemSchema) -> Self {
        value.clone().into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawPendingItem(Arc<PendingItemSchema>);

impl From<PendingItemSchema> for RawPendingItem {
    fn from(value: PendingItemSchema) -> Self {
        Self(Arc::new(value))
    }
}

impl RawPendingItem {
    #[must_use]
    pub(crate) fn new(pending_item_schema: PendingItemSchema) -> Self {
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
