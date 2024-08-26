use artifactsmmo_openapi::{
    apis::{maps_api::GetAllMapsMapsGetError, Error},
    models::{DataPageMapSchema, MapSchema},
};

use super::{account::Account, api::maps::MapsApi};

pub struct Maps {
    account: Account,
    maps_api: MapsApi,
}

impl Maps {
    pub fn new(account: &Account) -> Maps {
        Maps {
            account: account.clone(),
            maps_api: MapsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn closest_from_amoung(&self, x: i32, y: i32, maps: Vec<MapSchema>) -> Option<MapSchema> {
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

    pub fn get_cordinate_for_resources(
        &self,
        code: &str,
    ) -> Result<DataPageMapSchema, Error<GetAllMapsMapsGetError>> {
        self.maps_api.all(None, Some(code), None, None)
    }
}
