use super::{api::maps::MapsApi, config::Config, skill::Skill, MapSchemaExt};
use artifactsmmo_openapi::models::{MapContentSchema, MapSchema, ResourceSchema};
use itertools::Itertools;

pub struct Maps {
    pub data: Vec<MapSchema>,
}

impl Maps {
    pub fn new(config: &Config) -> Maps {
        let api = MapsApi::new(&config.base_url, &config.token);
        Maps {
            data: api.all(None, None).expect("maps to be retrieved from API."),
        }
    }

    pub fn closest_from_amoung(x: i32, y: i32, maps: Vec<&MapSchema>) -> Option<&MapSchema> {
        maps.into_iter()
            .min_by_key(|m| i32::abs(m.x - x) + i32::abs(m.y - y))
    }

    pub fn of_type(&self, r#type: &str) -> Vec<&MapSchema> {
        self.data
            .iter()
            .filter(|m| m.content.as_ref().is_some_and(|c| c.r#type == r#type))
            .collect_vec()
    }

    pub fn with_ressource(&self, code: &str) -> Vec<&MapSchema> {
        self.data
            .iter()
            .filter(|m| m.content.as_ref().is_some_and(|c| c.code == code))
            .collect_vec()
    }

    pub fn with_monster(&self, code: &str) -> Vec<&MapSchema> {
        self.data
            .iter()
            .filter(|m| m.content.as_ref().is_some_and(|c| c.code == code))
            .collect_vec()
    }

    pub fn with_content_code(&self, code: &str) -> Option<&MapSchema> {
        self.data.iter().find(|m| m.content_is(code))
    }

    pub fn with_content_schema(&self, schema: &MapContentSchema) -> Vec<&MapSchema> {
        self.data
            .iter()
            .filter(|m| m.content().is_some_and(|c| c == *schema))
            .collect_vec()
    }

    pub fn to_craft(&self, skill: Skill) -> Option<&MapSchema> {
        match skill {
            Skill::Weaponcrafting => self.with_content_code("weaponcrafting"),
            Skill::Gearcrafting => self.with_content_code("gearcrafting"),
            Skill::Jewelrycrafting => self.with_content_code("jewelrycrafting"),
            Skill::Cooking => self.with_content_code("cooking"),
            Skill::Woodcutting => self.with_content_code("woodcutting"),
            Skill::Mining => self.with_content_code("mining"),
            _ => None,
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
