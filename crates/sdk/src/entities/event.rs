use chrono::{DateTime, Utc};
use openapi::models::{ActiveEventSchema, EventContentSchema, EventSchema};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event(Arc<EventSchema>);

impl Event {
    pub(crate) fn new(schema: EventSchema) -> Self {
        Self(schema.into())
    }

    pub fn content(&self) -> &EventContentSchema {
        self.0.content.deref()
    }
}

pub trait EventSchemaExt {
    fn content_code(&self) -> &str;
    fn to_string(&self) -> String;
}

impl EventSchemaExt for Event {
    fn content_code(&self) -> &str {
        &self.0.content.code
    }

    fn to_string(&self) -> String {
        format!("{}: '{}'", self.0.name, self.content_code())
    }
}

impl EventSchemaExt for ActiveEventSchema {
    fn content_code(&self) -> &str {
        self.map
            .interactions
            .content
            .as_deref()
            .map(|c| &c.code)
            .expect("event to have content")
    }

    fn to_string(&self) -> String {
        let remaining = DateTime::parse_from_rfc3339(&self.expiration).map_or_else(
            |_| "?".to_string(),
            |expiration| (expiration.to_utc() - Utc::now()).num_seconds().to_string(),
        );
        format!(
            "{} ({},{}): '{}', duration: {}, created at {}, expires at {}, remaining: {}s",
            self.name,
            self.map.x,
            self.map.y,
            self.content_code(),
            self.duration,
            self.created_at,
            self.expiration,
            remaining
        )
    }
}
