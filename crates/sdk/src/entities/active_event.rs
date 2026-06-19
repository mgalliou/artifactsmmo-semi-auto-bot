use chrono::{Utc, prelude::{DateTime, FixedOffset}};
use openapi::models::ActiveEventSchema;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    ops::Deref,
    sync::Arc,
};

use crate::entities::{EventSchemaExt, RawMap};

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
    pub fn is_expired(&self) -> bool {
        self.0.expiration < Utc::now()
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

impl EventSchemaExt for ActiveEvent {
    fn content_code(&self) -> &str {
        self.0
            .map
            .interactions
            .content
            .as_deref()
            .map(|c| &c.code)
            .expect("event to have content")
    }

    fn pretty(&self) -> String {
        let remaining = self.0.expiration.to_utc() - Utc::now();
        format!(
            "{} ({},{}): '{}', duration: {}, created at {}, expires at {}, remaining: {}s",
            self.0.name,
            self.0.map.x,
            self.0.map.y,
            self.content_code(),
            self.0.duration,
            self.0.created_at,
            self.0.expiration,
            remaining
        )
    }
}

impl Display for ActiveEvent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}
