use crate::{client::events::EventsClient, entities::Map, skill::Skill};
use api::ArtifactApi;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use openapi::models::{MapContentSchema, MapContentType, MapLayer, TaskType};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone)]
pub struct MapsClient(Arc<MapsClientInner>);

#[derive(Default, Debug)]
struct MapsClientInner {
    data: HashMap<(MapLayer, i32, i32), RwLock<Map>>,
    events: EventsClient,
}

impl MapsClient {
    pub(crate) fn new(api: &ArtifactApi, events: EventsClient) -> Self {
        Self(
            MapsClientInner {
                data: api
                    .maps
                    .get_all()
                    .unwrap()
                    .into_iter()
                    .map(|m| ((m.layer, m.x, m.y), RwLock::new(Map::new(m))))
                    .collect(),
                events,
            }
            .into(),
        )
    }

    fn events(&self) -> EventsClient {
        self.0.events.clone()
    }

    pub fn get(&self, position: &(MapLayer, i32, i32)) -> Option<Map> {
        Some(self.0.data.get(position)?.read().unwrap().clone())
    }

    pub fn all(&self) -> Vec<Map> {
        self.0
            .data
            .values()
            .map(|m| m.read().unwrap().clone())
            .collect_vec()
    }

    pub fn refresh_from_events(&self) {
        self.events().active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(e.expiration()).is_ok_and(|e| e < Utc::now())
                && let Some(map) = self.0.data.get(&(e.map().layer, e.map().x, e.map().y))
            {
                *map.write().unwrap() = Map::new(e.previous_map().clone());
            }
        });
        self.events().refresh_active();
        self.events().active().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(e.expiration()).is_ok_and(|e| e > Utc::now())
                && let Some(map) = self.0.data.get(&(e.map().layer, e.map().x, e.map().y))
            {
                *map.write().unwrap() = Map::new(e.map().clone());
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
        self.all()
            .into_iter()
            .filter_map(|m| m.content_type_is(r#type).then_some(m))
            .collect_vec()
    }

    pub fn with_content_code(&self, code: &str) -> Vec<Map> {
        self.all()
            .into_iter()
            .filter_map(|m| m.content_code_is(code).then_some(m))
            .collect()
    }

    pub fn with_content(&self, content: &MapContentSchema) -> Vec<Map> {
        self.all()
            .into_iter()
            .filter_map(|m| m.content_is(content).then_some(m))
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
            Skill::Combat | Skill::Fishing => None,
        }
    }

    pub fn closest_with_content_code_from(&self, map: &Map, code: &str) -> Option<Map> {
        let maps = self.with_content_code(code);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    fn closest_with_content_from(&self, map: &Map, content: &MapContentSchema) -> Option<Map> {
        let maps = self.with_content(content);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    pub fn closest_of_type_from(&self, map: &Map, r#type: MapContentType) -> Option<Map> {
        let maps = self.of_type(r#type);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    pub fn closest_tasksmaster_from(&self, map: &Map, r#type: Option<TaskType>) -> Option<Map> {
        r#type.map_or_else(
            || self.closest_of_type_from(map, MapContentType::TasksMaster),
            |r#type| {
                self.closest_with_content_from(
                    map,
                    &MapContentSchema {
                        r#type: MapContentType::TasksMaster,
                        code: r#type.to_string(),
                    },
                )
            },
        )
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
