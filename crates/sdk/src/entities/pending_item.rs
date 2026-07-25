use chrono::Utc;
use openapi::models::{PendingItemSchema, SimpleItemSchema};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    sync::{Arc, RwLock},
};

pub trait PendingItem {
    fn id(&self) -> Cow<'_, str>;
    fn items(&self) -> Cow<'_, [SimpleItemSchema]>;
    fn is_claimed(&self) -> bool;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingItemHandle(Arc<RwLock<RawPendingItem>>);

impl PendingItemHandle {
    #[must_use]
    pub(crate) fn new(schema: PendingItemSchema) -> Self {
        Self(Arc::new(RwLock::new(RawPendingItem::new(schema))))
    }

    #[must_use]
    pub fn load(&self) -> RawPendingItem {
        self.0.read().unwrap().clone()
    }

    pub fn store(&self, raw: RawPendingItem) {
        *self.0.write().unwrap() = raw;
    }
}

impl PendingItem for PendingItemHandle {
    fn id(&self) -> Cow<'_, str> {
        Cow::Owned(self.0.read().unwrap().id().into_owned())
    }

    fn items(&self) -> Cow<'_, [SimpleItemSchema]> {
        Cow::Owned(self.0.read().unwrap().items().into_owned())
    }

    fn is_claimed(&self) -> bool {
        self.0.read().unwrap().is_claimed()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawPendingItem(Arc<PendingItemSchema>);

impl RawPendingItem {
    #[must_use]
    pub(crate) fn new(schema: PendingItemSchema) -> Self {
        Self(Arc::new(schema))
    }
}

impl PendingItem for RawPendingItem {
    fn id(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0.id)
    }

    fn items(&self) -> Cow<'_, [SimpleItemSchema]> {
        Cow::Borrowed(self.0.items.as_deref().unwrap_or_default())
    }

    fn is_claimed(&self) -> bool {
        self.0.claimed_at.is_some_and(|t| t < Utc::now())
    }
}
