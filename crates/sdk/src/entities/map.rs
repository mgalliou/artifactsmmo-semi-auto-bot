use crate::MapsClient;
use core::fmt;
use openapi::models::{
    AccessSchema, InteractionSchema, MapAccessType, MapContentSchema, MapContentType, MapSchema,
    TaskType,
};
use serde::{Deserialize, Serialize};
use std::{ops::Deref, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map(Arc<MapSchema>);

impl Map {
    pub fn new(schema: MapSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn x(&self) -> i32 {
        self.0.x
    }

    pub fn y(&self) -> i32 {
        self.0.y
    }

    pub fn content(&self) -> Option<&MapContentSchema> {
        self.0.interactions.content.as_ref().map(|c| c.as_ref())
    }

    pub fn content_is(&self, content: &MapContentSchema) -> bool {
        self.content().is_some_and(|c| c == content)
    }

    pub fn content_code_is(&self, code: &str) -> bool {
        self.content().is_some_and(|c| c.code == code)
    }

    pub fn content_type_is(&self, r#type: MapContentType) -> bool {
        self.content().is_some_and(|c| c.r#type == r#type)
    }

    pub fn access(&self) -> &AccessSchema {
        self.0.access.deref()
    }

    pub fn interactions(&self) -> &InteractionSchema {
        self.0.interactions.deref()
    }

    pub fn is_blocked(&self) -> bool {
        self.0.access.r#type == MapAccessType::Blocked
    }

    pub fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    pub fn closest_among(&self, others: &[Map]) -> Option<Map> {
        MapsClient::closest_from_amoung(self.0.x, self.0.y, others)
    }

    pub fn is_tasksmaster(&self, task_type: Option<TaskType>) -> bool {
        self.content_type_is(MapContentType::TasksMaster)
            && task_type.is_none_or(|tt| self.content_code_is(&tt.to_string()))
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(content) = self.content() {
            write!(
                f,
                "{} ({},{} [{}])",
                self.0.name, self.0.x, self.0.y, content.code
            )
        } else {
            write!(f, "{} ({},{})", self.0.name, self.0.x, self.0.y)
        }
    }
}
