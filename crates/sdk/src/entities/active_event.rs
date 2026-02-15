use openapi::models::{ActiveEventSchema, MapSchema};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEvent(Arc<ActiveEventSchema>);

impl ActiveEvent {
    pub fn new(schema: ActiveEventSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn expiration(&self) -> &str {
        &self.0.expiration
    }

    pub fn map(&self) -> &MapSchema {
        self.0.map.deref()
    }

    pub fn previous_map(&self) -> &MapSchema {
        self.0.previous_map.deref()
    }
}
