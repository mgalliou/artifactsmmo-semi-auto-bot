use crate::{char::Skill, events::EVENTS, resources::RESOURCES, API, MONSTERS};
use artifactsmmo_openapi::models::{MapContentSchema, MapSchema, MonsterSchema, ResourceSchema};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, RwLock},
};
use strum_macros::{AsRefStr, Display};

pub static MAPS: LazyLock<Maps> = LazyLock::new(Maps::new);

pub struct Maps(HashMap<(i32, i32), RwLock<Arc<MapSchema>>>);

impl Maps {
    fn new() -> Self {
        Self(
            API.maps
                .all(None, None)
                .unwrap()
                .into_iter()
                .map(|m| ((m.x, m.y), RwLock::new(Arc::new(m))))
                .collect(),
        )
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Arc<MapSchema>> {
        Some(self.0.get(&(x, y))?.read().unwrap().clone())
    }

    pub fn refresh_from_events(&self) {
        EVENTS.active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(&e.expiration).unwrap() < Utc::now() {
                if let Some(map) = self.0.get(&(e.map.x, e.map.y)) {
                    let mut new_map = (*map.read().unwrap().clone()).clone();
                    new_map.content = None;
                    new_map.skin = e.previous_skin.clone();
                    *map.write().unwrap() = Arc::new(new_map);
                }
            }
        });
        EVENTS.refresh_active();
        EVENTS.active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(&e.expiration).unwrap() > Utc::now() {
                if let Some(map) = self.0.get(&(e.map.x, e.map.y)) {
                    let mut new_map = (*map.read().unwrap().clone()).clone();
                    new_map.content = e.map.content.clone();
                    new_map.skin = e.map.skin.clone();
                    *map.write().unwrap() = Arc::new(new_map);
                }
            }
        });
    }

    pub fn closest_from_amoung(
        x: i32,
        y: i32,
        maps: Vec<Arc<MapSchema>>,
    ) -> Option<Arc<MapSchema>> {
        maps.into_iter()
            .min_by_key(|m| i32::abs(x - m.x) + i32::abs(y - m.y))
    }

    pub fn of_type(&self, r#type: ContentType) -> Vec<Arc<MapSchema>> {
        self.0
            .values()
            .filter(|m| {
                m.read()
                    .unwrap()
                    .content
                    .as_ref()
                    .is_some_and(|c| c.r#type == r#type)
            })
            .map(|m| m.read().unwrap().clone())
            .collect()
    }

    pub fn with_content_code(&self, code: &str) -> Vec<Arc<MapSchema>> {
        self.0
            .values()
            .filter(|m| m.read().unwrap().content_code_is(code))
            .map(|m| m.read().unwrap().clone())
            .collect()
    }

    pub fn with_content_schema(&self, schema: &MapContentSchema) -> Vec<Arc<MapSchema>> {
        self.0
            .values()
            .filter(|m| m.read().unwrap().content().is_some_and(|c| c == schema))
            .map(|m| m.read().unwrap().clone())
            .collect()
    }

    pub fn with_workshop_for(&self, skill: Skill) -> Option<Arc<MapSchema>> {
        match skill {
            Skill::Weaponcrafting
            | Skill::Gearcrafting
            | Skill::Jewelrycrafting
            | Skill::Cooking
            | Skill::Woodcutting
            | Skill::Mining
            | Skill::Alchemy => self.with_content_code(skill.as_ref()).first().cloned(),
            Skill::Combat => None,
            Skill::Fishing => None,
        }
    }
}

pub trait MapSchemaExt {
    fn content(&self) -> Option<&MapContentSchema>;
    fn content_code_is(&self, code: &str) -> bool;
    fn content_type_is(&self, r#type: ContentType) -> bool;
    fn pretty(&self) -> String;
    fn monster(&self) -> Option<Arc<MonsterSchema>>;
    fn resource(&self) -> Option<Arc<ResourceSchema>>;
}

impl MapSchemaExt for MapSchema {
    fn content(&self) -> Option<&MapContentSchema> {
        self.content.as_ref().map(|c| c.as_ref())
    }

    fn content_code_is(&self, code: &str) -> bool {
        self.content.as_ref().is_some_and(|c| c.code == code)
    }

    fn content_type_is(&self, r#type: ContentType) -> bool {
        self.content.as_ref().is_some_and(|c| c.r#type == r#type)
    }

    fn pretty(&self) -> String {
        if let Some(content) = self.content() {
            format!("{} ({},{} [{}])", self.name, self.x, self.y, content.code)
        } else {
            format!("{} ({},{})", self.name, self.x, self.y)
        }
    }

    fn monster(&self) -> Option<Arc<MonsterSchema>> {
        MONSTERS.get(&self.content()?.code)
    }

    fn resource(&self) -> Option<Arc<ResourceSchema>> {
        RESOURCES.get(&self.content()?.code)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, AsRefStr, Display)]
#[strum(serialize_all = "snake_case")]
pub enum ContentType {
    Monster,
    Resource,
    Workshop,
    Bank,
    GrandExchange,
    TasksMaster,
    SantaClaus,
}

impl PartialEq<ContentType> for String {
    fn eq(&self, other: &ContentType) -> bool {
        other.as_ref() == *self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_content_type_as_string() {
        assert_eq!(ContentType::Monster.to_string(), "monster");
        assert_eq!(ContentType::Monster.as_ref(), "monster");
    }
}
