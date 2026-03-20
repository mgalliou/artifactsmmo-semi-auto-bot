use openapi::models::ActiveEventSchema;
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

use crate::entities::Map;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActiveEvent(Arc<ActiveEventSchema>);

impl ActiveEvent {
    pub(crate) fn new(schema: ActiveEventSchema) -> Self {
        Self(schema.into())
    }
    pub fn expiration(&self) -> &str {
        &self.0.expiration
    }

    pub fn map(&self) -> Map {
        self.0.map.deref().into()
    }

    pub fn previous_map(&self) -> Map {
        self.0.map.deref().into()
    }
}
