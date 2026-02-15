use crate::{client::events::EventsClient, entities::Map, skill::Skill};
use api::ArtifactApi;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use openapi::models::{MapContentSchema, MapContentType, MapLayer, TaskType};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug)]
pub struct MapsClient {
    data: HashMap<(MapLayer, i32, i32), RwLock<Map>>,
    events: Arc<EventsClient>,
}

impl MapsClient {
    pub(crate) fn new(api: &ArtifactApi, events: Arc<EventsClient>) -> Self {
        Self {
            data: api
                .maps
                .get_all()
                .unwrap()
                .into_iter()
                .map(|m| ((m.layer, m.x, m.y), RwLock::new(Map::new(m))))
                .collect(),
            events,
        }
    }

    pub fn get(&self, layer: MapLayer, x: i32, y: i32) -> Option<Map> {
        Some(self.data.get(&(layer, x, y))?.read().unwrap().clone())
    }

    pub fn refresh_from_events(&self) {
        self.events.active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(e.expiration()).is_ok_and(|e| e < Utc::now())
                && let Some(map) = self.data.get(&(e.map().layer, e.map().x, e.map().y))
            {
                *map.write().unwrap() = Map::new(e.previous_map().clone())
            }
        });
        self.events.refresh_active();
        self.events.active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(e.expiration()).is_ok_and(|e| e > Utc::now())
                && let Some(map) = self.data.get(&(e.map().layer, e.map().x, e.map().y))
            {
                *map.write().unwrap() = Map::new(e.map().clone())
            }
        });
    }

    //TODO: handle layer
    pub fn closest_from_amoung(x: i32, y: i32, maps: &[Map]) -> Option<Map> {
        maps.iter()
            .min_by_key(|m| i32::abs(x - m.x()) + i32::abs(y - m.y()))
            .cloned()
    }

    pub fn of_type(&self, r#type: MapContentType) -> Vec<Map> {
        self.data
            .values()
            .filter_map(|m| {
                let map = m.read().unwrap().clone();
                map.content_type_is(r#type).then_some(map)
            })
            .collect_vec()
    }

    pub fn with_content_code(&self, code: &str) -> Vec<Map> {
        self.data
            .values()
            .filter_map(|m| {
                let map = m.read().unwrap().clone();
                map.content_code_is(code).then_some(map)
            })
            .collect()
    }

    pub fn with_content(&self, content: &MapContentSchema) -> Vec<Map> {
        self.data
            .values()
            .filter_map(|m| {
                let map = m.read().unwrap().clone();
                map.content_is(content).then_some(map)
            })
            .collect()
    }

    pub fn with_workshop_for(&self, skill: Skill) -> Option<Map> {
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

    pub fn closest_with_content_code_from(&self, map: Map, code: &str) -> Option<Map> {
        let maps = self.with_content_code(code);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    fn closest_with_content_from(&self, map: Map, content: &MapContentSchema) -> Option<Map> {
        let maps = self.with_content(content);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    pub fn closest_of_type_from(&self, map: Map, r#type: MapContentType) -> Option<Map> {
        let maps = self.of_type(r#type);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    pub fn closest_tasksmaster_from(&self, map: Map, r#type: Option<TaskType>) -> Option<Map> {
        if let Some(r#type) = r#type {
            self.closest_with_content_from(
                map,
                &MapContentSchema {
                    r#type: MapContentType::TasksMaster,
                    code: r#type.to_string(),
                },
            )
        } else {
            self.closest_of_type_from(map, MapContentType::TasksMaster)
        }
    }
}

#[cfg(test)]
mod tests {
    //use super::*;

    // #[test]
    // fn check_content_type_as_string() {
    //     assert_eq!(ContentType::Monster.to_string(), "monster");
    //     assert_eq!(ContentType::Monster.as_ref(), "monster");
    // }
}
