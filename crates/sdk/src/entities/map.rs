use crate::MapsClient;
use core::fmt;
use derive_more::Deref;
use openapi::models::{
    AccessSchema, InteractionSchema, MapAccessType, MapContentSchema, MapContentType, MapLayer,
    MapSchema, TaskType, TransitionSchema,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::AsRef,
    sync::{Arc, RwLock},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDataHandle(Arc<RwLock<Map>>);

impl MapDataHandle {
    pub fn read(&self) -> Map {
        self.0.read().unwrap().clone()
    }

    pub fn update(&self, data: Map) {
        *self.0.write().unwrap() = data;
    }
}

impl From<MapSchema> for MapDataHandle {
    fn from(value: MapSchema) -> Self {
        Self(RwLock::new(value.into()).into())
    }
}

impl From<&MapSchema> for MapDataHandle {
    fn from(value: &MapSchema) -> Self {
        value.clone().into()
    }
}

#[derive(Clone, Debug, PartialEq, Deref, Serialize, Deserialize)]
#[deref(forward)]
pub struct Map(Arc<MapSchema>);

impl Map {
    #[must_use]
    pub fn id(&self) -> i32 {
        self.map_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn position(&self) -> (MapLayer, i32, i32) {
        (self.layer(), self.x(), self.y())
    }

    #[must_use]
    pub fn layer(&self) -> MapLayer {
        self.layer
    }

    #[must_use]
    pub fn x(&self) -> i32 {
        self.x
    }

    #[must_use]
    pub fn y(&self) -> i32 {
        self.y
    }

    #[must_use]
    pub fn closest_among(&self, others: &[Self]) -> Option<Self> {
        MapsClient::closest_from_amoung(self.x(), self.y(), others)
    }

    #[must_use]
    pub fn content_code_is(&self, code: &str) -> bool {
        self.content_code().is_some_and(|c| c == code)
    }

    #[must_use]
    pub fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    #[must_use]
    pub fn is_tasksmaster(&self, task_type: Option<TaskType>) -> bool {
        self.content_type_is(MapContentType::TasksMaster)
            && task_type.is_none_or(|tt| self.content_code_is(&tt.to_string()))
    }

    #[must_use]
    pub fn content_type_is(&self, r#type: MapContentType) -> bool {
        self.content_type().is_some_and(|t| *t == r#type)
    }

    #[must_use]
    pub fn content_type(&self) -> Option<&MapContentType> {
        Some(&self.content()?.r#type)
    }

    #[must_use]
    pub fn content_is(&self, content: &MapContentSchema) -> bool {
        self.content().is_some_and(|c| c == content)
    }

    pub fn content(&self) -> Option<&MapContentSchema> {
        self.interactions.content.as_ref().map(AsRef::as_ref)
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.access().r#type == MapAccessType::Blocked
    }

    #[must_use]
    pub fn access(&self) -> &AccessSchema {
        &self.access
    }

    #[must_use]
    pub fn transition(&self) -> Option<&TransitionSchema> {
        self.interactions().transition.as_deref()
    }

    #[must_use]
    pub fn interactions(&self) -> &InteractionSchema {
        &self.interactions
    }
}

impl From<MapSchema> for Map {
    fn from(value: MapSchema) -> Self {
        Self(value.into())
    }
}

impl From<&MapSchema> for Map {
    fn from(value: &MapSchema) -> Self {
        value.clone().into()
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(content) = self.content() {
            write!(
                f,
                "{} ({}, {} [{}])",
                self.name(),
                self.x(),
                self.y(),
                content.code
            )
        } else {
            write!(f, "{} ({}, {})", self.name(), self.x(), self.y())
        }
    }
}
