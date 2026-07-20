use crate::entities::EventSchemaExt;
use openapi::models::{EventContentSchema, EventMapSchema, EventSchema};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Event(Arc<EventSchema>);

impl Event {
    #[must_use]
    pub(crate) fn new(schema: EventSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn content(&self) -> Option<&EventContentSchema> {
        self.0.content.as_deref()
    }

    #[must_use]
    pub fn maps(&self) -> &Vec<EventMapSchema> {
        &self.0.maps
    }
}

impl EventSchemaExt for Event {
    fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    fn pretty(&self) -> String {
        format!("{}: '{:?}'", self.0.name, self.content_code())
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}
