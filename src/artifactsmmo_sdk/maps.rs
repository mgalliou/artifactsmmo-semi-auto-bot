use super::{account::Account, api::maps::MapsApi};
use artifactsmmo_openapi::models::{MapSchema, ResourceSchema};
use itertools::Itertools;

pub struct Maps {
    pub data: Vec<MapSchema>,
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

    pub fn has_one_of_resource(map: &MapSchema, resources: Vec<&ResourceSchema>) -> bool {
        map.content
            .as_ref()
            .is_some_and(|c| resources.iter().any(|r| r.code == c.code))
    }
}
