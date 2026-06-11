use chrono::prelude::{DateTime, FixedOffset};
use openapi::models::ActiveEventSchema;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

use crate::entities::RawMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEvent(Arc<ActiveEventSchema>);

impl ActiveEvent {
    pub(crate) fn new(schema: ActiveEventSchema) -> Self {
        Self(schema.into())
    }
    #[must_use]
    pub fn expiration(&self) -> DateTime<FixedOffset> {
        self.0.expiration
    }

    #[must_use]
    pub fn map(&self) -> RawMap {
        self.0.map.deref().into()
    }

    #[must_use]
    pub fn previous_map(&self) -> RawMap {
        self.0.map.deref().into()
    }
}
