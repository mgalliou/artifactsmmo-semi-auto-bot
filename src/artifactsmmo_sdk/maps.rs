use super::{account::Account, api::maps::MapsApi, skill::Skill, MapSchemaExt};
use artifactsmmo_openapi::models::{MapContentSchema, MapSchema, ResourceSchema};
use itertools::Itertools;

pub struct Maps {
    pub data: Vec<MapSchema>,
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
}

impl Maps {
    pub fn new(account: &Account) -> Maps {
        let api = MapsApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Maps {
            data: api.all(None, None).unwrap().clone(),
        }
    }

    pub fn closest_from_amoung(x: i32, y: i32, maps: Vec<&MapSchema>) -> Option<&MapSchema> {
        let mut delta_total;
        let mut min_delta = -1;
        let mut target_map = None;

        for map in maps {
            delta_total = i32::abs(map.x - x) + i32::abs(map.y - y);
            if min_delta == -1 || delta_total < min_delta {
                min_delta = delta_total;
                target_map = Some(map);
            }
        }
        target_map
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

    pub fn with_content(&self, code: &str) -> Option<&MapSchema> {
         self.data.iter().find(|m| m.content_is(code))
    }

    pub fn to_craft(&self, skill: Skill) -> Option<&MapSchema> {
        match skill {
            Skill::Weaponcrafting => self.with_content("weaponcrafting"),
            Skill::Gearcrafting => self.with_content("gearcrafting"),
            Skill::Jewelrycrafting => self.with_content("jewelrycrafting"),
            Skill::Cooking => self.with_content("cooking"),
            Skill::Woodcutting => self.with_content("woodcutting"),
            Skill::Mining => self.with_content("mining"),
            _ => None,
        }
    }
}
