use crate::MapsClient;
use core::fmt;
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
        Self(Arc::new(RwLock::new(value.into())))
    }
}

impl From<&MapSchema> for MapDataHandle {
    fn from(value: &MapSchema) -> Self {
        value.clone().into()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map(Arc<MapSchema>);

impl Map {
    pub fn id(&self) -> i32 {
        self.0.map_id
    }

    pub fn position(&self) -> (MapLayer, i32, i32) {
        (self.layer(), self.x(), self.y())
    }

    pub fn layer(&self) -> MapLayer {
        self.0.layer
    }

    pub fn x(&self) -> i32 {
        self.0.x
    }

    pub fn y(&self) -> i32 {
        self.0.y
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn content(&self) -> Option<&MapContentSchema> {
        self.0.interactions.content.as_ref().map(AsRef::as_ref)
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
        &self.0.access
    }

    pub fn interactions(&self) -> &InteractionSchema {
        &self.0.interactions
    }

    pub fn transition(&self) -> Option<&TransitionSchema> {
        self.interactions().transition.as_deref()
    }

    pub fn is_blocked(&self) -> bool {
        self.0.access.r#type == MapAccessType::Blocked
    }

    pub fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    pub fn closest_among(&self, others: &[Self]) -> Option<Self> {
        MapsClient::closest_from_amoung(self.0.x, self.0.y, others)
    }

    pub fn is_tasksmaster(&self, task_type: Option<TaskType>) -> bool {
        self.content_type_is(MapContentType::TasksMaster)
            && task_type.is_none_or(|tt| self.content_code_is(&tt.to_string()))
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
