use chrono::Utc;
use openapi::models::{ActiveEventSchema, EventContentSchema, EventSchema};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event(Arc<EventSchema>);

impl Event {
    pub(crate) fn new(schema: EventSchema) -> Self {
        Self(schema.into())
    }

    #[must_use]
    pub fn content(&self) -> Option<&EventContentSchema> {
        Some(self.0.content.as_ref()?)
    }
}

pub trait EventSchemaExt {
    fn content_code(&self) -> Option<&str>;
    fn pretty(&self) -> String;
}

impl EventSchemaExt for Event {
    fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    fn pretty(&self) -> String {
        format!("{}: '{:?}'", self.0.name, self.content_code())
    }
}

impl EventSchemaExt for ActiveEventSchema {
    fn content_code(&self) -> Option<&str> {
        Some(&self.map.interactions.content.as_deref()?.code)
    }

    fn pretty(&self) -> String {
        let remaining = self.expiration.to_utc() - Utc::now();
        format!(
            "{} ({},{}): '{:?}', duration: {}, created at {}, expires at {}, remaining: {}s",
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

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}
