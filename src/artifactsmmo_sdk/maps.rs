use std::sync::Arc;

use super::{api::maps::MapsApi, game_config::GameConfig, events::Events, skill::Skill, MapSchemaExt};
use artifactsmmo_openapi::models::{MapContentSchema, MapSchema, ResourceSchema};
use itertools::Itertools;

pub struct Maps {
    pub data: Vec<Arc<MapSchema>>,
    pub events: Arc<Events>,
}

impl Maps {
    pub fn new(config: &GameConfig, events: &Arc<Events>) -> Maps {
        let api = MapsApi::new(&config.base_url);
        Maps {
            data: api
                .all(None, None)
                .expect("maps to be retrieved from API.")
                .into_iter()
                .map(Arc::new)
                .collect_vec(),
            events: events.clone(),
        }
    }

    pub fn get(&self, x: i32, y: i32) -> Option<Arc<MapSchema>> {
        self.events
            .maps()
            .into_iter()
            .find(|m| m.x == x && m.y == y)
            .or_else(|| self.data.iter().find(|m| m.x == x && m.y == y).cloned())
    }

    pub fn closest_from_amoung(
        x: i32,
        y: i32,
        maps: Vec<Arc<MapSchema>>,
    ) -> Option<Arc<MapSchema>> {
        maps.into_iter()
            .min_by_key(|m| i32::abs(m.x - x) + i32::abs(m.y - y))
    }

    pub fn of_type(&self, r#type: &str) -> Vec<Arc<MapSchema>> {
        self.data
            .iter()
            .chain(self.events.maps().iter())
            .filter(|m| m.content.as_ref().is_some_and(|c| c.r#type == r#type))
            .cloned()
            .collect_vec()
    }

    pub fn with_ressource(&self, code: &str) -> Vec<Arc<MapSchema>> {
        self.data
            .iter()
            .chain(self.events.maps().iter())
            .filter(|m| m.content.as_ref().is_some_and(|c| c.code == code))
            .cloned()
            .collect_vec()
    }

    pub fn with_monster(&self, code: &str) -> Vec<Arc<MapSchema>> {
        self.data
            .iter()
            .chain(self.events.maps().iter())
            .filter(|m| m.content.as_ref().is_some_and(|c| c.code == code))
            .cloned()
            .collect_vec()
    }

    pub fn with_content_code(&self, code: &str) -> Option<Arc<MapSchema>> {
        self.data
            .iter()
            .chain(self.events.maps().iter())
            .find(|m| m.content_is(code))
            .cloned()
    }

    pub fn with_content_schema(&self, schema: &MapContentSchema) -> Vec<Arc<MapSchema>> {
        self.data
            .iter()
            .chain(self.events.maps().iter())
            .filter(|m| m.content().is_some_and(|c| c == *schema))
            .cloned()
            .collect_vec()
    }

    pub fn to_craft(&self, skill: Skill) -> Option<Arc<MapSchema>> {
        match skill {
            Skill::Weaponcrafting => self.with_content_code("weaponcrafting"),
            Skill::Gearcrafting => self.with_content_code("gearcrafting"),
            Skill::Jewelrycrafting => self.with_content_code("jewelrycrafting"),
            Skill::Cooking => self.with_content_code("cooking"),
            Skill::Woodcutting => self.with_content_code("woodcutting"),
            Skill::Mining => self.with_content_code("mining"),
            Skill::Alchemy => self.with_content_code("alchemy"),
            Skill::Combat => None,
            Skill::Fishing => None,
        }
    }
}

impl MapSchemaExt for MapSchema {
    fn has_one_of_resource(&self, resources: &[&ResourceSchema]) -> bool {
        self.content
            .as_ref()
            .is_some_and(|c| resources.iter().any(|r| r.code == c.code))
    }

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
