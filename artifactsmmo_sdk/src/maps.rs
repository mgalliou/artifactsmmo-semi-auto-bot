use crate::{char::Skill, events::EVENTS, API};
use artifactsmmo_openapi::models::{MapContentSchema, MapSchema};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};
use strum_macros::{AsRefStr, Display};

pub static MAPS: LazyLock<Maps> = LazyLock::new(Maps::new);

pub struct Maps {
    data: HashMap<(i32, i32), RwLock<MapSchema>>,
}

impl Maps {
    pub fn new() -> Self {
        Self {
            data: API
                .maps
                .all(None, None)
                .expect("maps to be retrieved from API.")
                .into_iter()
                .map(|m| ((m.x, m.y), RwLock::new(m)))
                .collect(),
        }
    }

    pub fn refresh(&self) {
        EVENTS.active.read().unwrap().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(&e.expiration).unwrap() < Utc::now() {
                if let Some(map) = self.data.get(&(e.map.x, e.map.y)) {
                    map.write().unwrap().content = None;
                    map.write().unwrap().skin = e.previous_skin.clone();
                }
            }
        });
        EVENTS.refresh_active();
        EVENTS.active.read().unwrap().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(&e.expiration).unwrap() > Utc::now() {
                if let Some(map) = self.data.get(&(e.map.x, e.map.y)) {
                    map.write().unwrap().content = e.map.content.clone();
                    map.write().unwrap().skin = e.map.skin.clone();
                }
            }
        });
    }

    pub fn get(&self, x: i32, y: i32) -> Option<MapSchema> {
        Some(self.data.get(&(x, y))?.read().unwrap().clone())
    }

    pub fn closest_from_amoung(x: i32, y: i32, maps: Vec<MapSchema>) -> Option<MapSchema> {
        maps.into_iter()
            .min_by_key(|m| i32::abs(x - m.x) + i32::abs(y - m.y))
    }

    pub fn of_type(&self, r#type: ContentType) -> Vec<MapSchema> {
        self.data
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

    pub fn with_content_code(&self, code: &str) -> Vec<MapSchema> {
        self.data
            .values()
            .filter(|m| m.read().unwrap().content_is(code))
            .map(|m| m.read().unwrap().clone())
            .collect()
    }

    pub fn with_content_schema(&self, schema: &MapContentSchema) -> Vec<MapSchema> {
        self.data
            .values()
            .filter(|m| m.read().unwrap().content().is_some_and(|c| c == *schema))
            .map(|m| m.read().unwrap().clone())
            .collect()
    }

    pub fn workshop(&self, skill: Skill) -> Option<MapSchema> {
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
    fn content(&self) -> Option<MapContentSchema>;
    fn content_is(&self, code: &str) -> bool;
    fn pretty(&self) -> String;
}

impl MapSchemaExt for MapSchema {
    fn content(&self) -> Option<MapContentSchema> {
        self.content.clone().map(|c| *c)
    }

    fn content_is(&self, code: &str) -> bool {
        self.content().is_some_and(|c| c.code == code)
    }

    fn pretty(&self) -> String {
        if let Some(content) = self.content() {
            format!("{} ({},{} [{}])", self.name, self.x, self.y, content.code)
        } else {
            format!("{} ({},{})", self.name, self.x, self.y)
        }
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
