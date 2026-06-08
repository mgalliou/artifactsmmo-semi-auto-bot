use crate::{
    client::events::EventsClient,
    entities::{Map, MapDataHandle},
    skill::Skill,
};
use api::ArtifactApi;
use chrono::Utc;
use itertools::Itertools;
use log::info;
use openapi::models::{MapContentSchema, MapContentType, MapLayer, TaskType};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone)]
pub struct MapsClient(Arc<MapsClientInner>);

#[derive(Default, Debug)]
struct MapsClientInner {
    data: RwLock<HashMap<(MapLayer, i32, i32), MapDataHandle>>,
    events: EventsClient,
    api: ArtifactApi,
}

impl MapsClient {
    pub(crate) fn new(api: ArtifactApi, events: EventsClient) -> Self {
        Self(
            MapsClientInner {
                data: RwLock::default(),
                events,
                api,
            }
            .into(),
        )
    }

    pub(crate) fn init(&self) {
        *self.0.data.write().unwrap() = self
            .0
            .api
            .maps
            .get_all()
            .unwrap()
            .into_iter()
            .map(|m| ((m.layer, m.x, m.y), m.into()))
            .collect();
        info!("Maps client initilized");
    }

    fn events(&self) -> EventsClient {
        self.0.events.clone()
    }

    #[must_use]
    pub fn get(&self, position: &(MapLayer, i32, i32)) -> Option<Map> {
        Some(self.0.data.read().unwrap().get(position)?.read())
    }

    pub fn all(&self) -> Vec<Map> {
        self.0
            .data
            .read()
            .unwrap()
            .values()
            .map(MapDataHandle::read)
            .collect_vec()
    }

    pub fn refresh_from_events(&self) {
        self.events().active().iter().for_each(|e| {
            if e.expiration() < Utc::now()
                && let Some(map) = self.0.data.read().unwrap().get(&e.map().position())
            {
                map.update(e.previous_map());
            }
        });
        self.events().refresh_active();
        self.events().active().iter().for_each(|e| {
            if e.expiration() > Utc::now()
                && let Some(map) = self.0.data.read().unwrap().get(&e.map().position())
            {
                map.update(e.map());
            }
        });
    }

    //TODO: handle layer
    #[must_use]
    pub fn closest_from_amoung(x: i32, y: i32, maps: &[Map]) -> Option<Map> {
        maps.iter()
            .min_by_key(|m| i32::abs(x - m.x()) + i32::abs(y - m.y()))
            .cloned()
    }

    #[must_use]
    pub fn of_type(&self, r#type: MapContentType) -> Vec<Map> {
        self.all()
            .into_iter()
            .filter_map(|m| m.content_type_is(r#type).then_some(m))
            .collect_vec()
    }

    #[must_use]
    pub fn with_content_code(&self, code: &str) -> Vec<Map> {
        self.all()
            .into_iter()
            .filter_map(|m| m.content_code_is(code).then_some(m))
            .collect()
    }

    #[must_use]
    pub fn with_content(&self, content: &MapContentSchema) -> Vec<Map> {
        self.all()
            .into_iter()
            .filter_map(|m| m.content_is(content).then_some(m))
            .collect()
    }

    #[must_use]
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

    #[must_use]
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

    #[must_use]
    pub fn closest_of_type_from(&self, map: &Map, r#type: MapContentType) -> Option<Map> {
        let maps = self.of_type(r#type);
        if maps.is_empty() {
            return None;
        }
        map.closest_among(&maps)
    }

    #[must_use]
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
