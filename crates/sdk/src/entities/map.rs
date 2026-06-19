use core::fmt::{self, Display, Formatter};
use openapi::models::{
    AccessSchema, InteractionSchema, MapAccessType, MapContentSchema, MapContentType, MapLayer,
    MapSchema, TaskType, TransitionSchema,
};
use serde::{Deserialize, Serialize};
use std::{
    convert::AsRef,
    sync::{Arc, RwLock},
};

pub trait Map {
    fn position(&self) -> (MapLayer, i32, i32) {
        (self.layer(), self.x(), self.y())
    }

    fn closest_among(&self, others: &[Self]) -> Option<Self>
    where
        Self: std::marker::Sized + std::clone::Clone,
    {
        others
            .iter()
            .min_by_key(|m| i32::abs(self.x() - m.x()) + i32::abs(self.y() - m.y()))
            .cloned()
    }

    fn content_code_is(&self, code: &str) -> bool {
        self.content_code().is_some_and(|c| c == code)
    }

    fn content_code(&self) -> Option<&str> {
        Some(&self.content()?.code)
    }

    fn is_bank(&self) -> bool {
        self.content_type_is(MapContentType::Bank)
    }

    fn is_tasksmaster(&self, task_type: impl Into<Option<TaskType>>) -> bool {
        self.content_type_is(MapContentType::TasksMaster)
            && task_type
                .into()
                // TODO remove to_string
                .is_none_or(|tt| self.content_code_is(&tt.to_string()))
    }

    fn content_type_is(&self, r#type: MapContentType) -> bool {
        self.content_type().is_some_and(|t| *t == r#type)
    }

    fn content_type(&self) -> Option<&MapContentType> {
        Some(&self.content()?.r#type)
    }

    fn content_is(&self, content: &MapContentSchema) -> bool {
        self.content().is_some_and(|c| c == content)
    }

    fn content(&self) -> Option<&MapContentSchema> {
        self.interactions().content.as_ref().map(AsRef::as_ref)
    }

    fn transition(&self) -> Option<&TransitionSchema> {
        self.interactions().transition.as_deref()
    }

    fn is_blocked(&self) -> bool {
        self.access().r#type == MapAccessType::Blocked
    }

    fn id(&self) -> i32;
    fn name(&self) -> &str;
    fn layer(&self) -> MapLayer;
    fn x(&self) -> i32;
    fn y(&self) -> i32;
    fn interactions(&self) -> &InteractionSchema;
    fn access(&self) -> &AccessSchema;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapDataHandle(Arc<RwLock<RawMap>>);

impl MapDataHandle {
    pub fn read(&self) -> RawMap {
        self.0.read().unwrap().clone()
    }

    pub fn update(&self, data: RawMap) {
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawMap(Arc<MapSchema>);

impl Map for RawMap {
    fn id(&self) -> i32 {
        self.0.map_id
    }

    fn name(&self) -> &str {
        &self.0.name
    }

    fn layer(&self) -> MapLayer {
        self.0.layer
    }

    fn x(&self) -> i32 {
        self.0.x
    }

    fn y(&self) -> i32 {
        self.0.y
    }

    fn interactions(&self) -> &InteractionSchema {
        &self.0.interactions
    }

    fn access(&self) -> &AccessSchema {
        &self.0.access
    }
}

impl From<MapSchema> for RawMap {
    fn from(value: MapSchema) -> Self {
        Self(value.into())
    }
}

impl From<&MapSchema> for RawMap {
    fn from(value: &MapSchema) -> Self {
        value.clone().into()
    }
}

impl Display for RawMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {} [{}])",
            self.name(),
            self.x(),
            self.y(),
            self.layer()
        )
    }
}
