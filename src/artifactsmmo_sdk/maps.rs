use super::{
    api::maps::MapsApi, events::Events, game_config::GameConfig, skill::Skill, MapSchemaExt,
};
use artifactsmmo_openapi::models::{ActiveEventSchema, MapContentSchema, MapSchema};
use chrono::{DateTime, Utc};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct Maps {
    api: MapsApi,
    data: HashMap<(i32, i32), RwLock<MapSchema>>,
    events: Arc<Events>,
    active_events: Arc<RwLock<Vec<ActiveEventSchema>>>,
}

impl Maps {
    pub fn new(config: &GameConfig, events: &Arc<Events>) -> Maps {
        let api = MapsApi::new(&config.base_url);
        Maps {
            data: api
                .all(None, None)
                .expect("maps to be retrieved from API.")
                .into_iter()
                .map(|m| ((m.x, m.y), RwLock::new(m)))
                .collect(),
            api,
            events: events.clone(),
            active_events: events.active.clone(),
        }
    }

    pub fn refresh(&self) {
        self.active_events.read().unwrap().iter().for_each(|e| {
            if DateTime::parse_from_rfc3339(&e.expiration).unwrap() < Utc::now() {
                if let Some(map) = self.data.get(&(e.map.x, e.map.y)) {
                    map.write().unwrap().content = None;
                    map.write().unwrap().skin = e.previous_skin.clone();
                }
            }
        });
        self.events.refresh();
        self.active_events.read().unwrap().iter().for_each(|e| {
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
            .min_by_key(|m| i32::abs(m.x - x) + i32::abs(m.y - y))
    }

    pub fn of_type(&self, r#type: &str) -> Vec<MapSchema> {
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

    pub fn to_craft(&self, skill: Skill) -> Option<MapSchema> {
        match skill {
            Skill::Weaponcrafting => self.with_content_code("weaponcrafting").first().cloned(),
            Skill::Gearcrafting => self.with_content_code("gearcrafting").first().cloned(),
            Skill::Jewelrycrafting => self.with_content_code("jewelrycrafting").first().cloned(),
            Skill::Cooking => self.with_content_code("cooking").first().cloned(),
            Skill::Woodcutting => self.with_content_code("woodcutting").first().cloned(),
            Skill::Mining => self.with_content_code("mining").first().cloned(),
            Skill::Alchemy => self.with_content_code("alchemy").first().cloned(),
            Skill::Combat => None,
            Skill::Fishing => None,
        }
    }
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
