use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        maps_api::{
            get_all_maps_maps_get, get_map_maps_xy_get, GetAllMapsMapsGetError,
            GetMapMapsXyGetError,
        },
        Error,
    },
    models::{MapContentType, MapResponseSchema, MapSchema},
};

pub struct MapsApi {
    configuration: Configuration,
}

impl MapsApi {
    pub fn new(base_path: &str, token: &str) -> MapsApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        MapsApi { configuration }
    }

    pub fn all(
        &self,
        content_type: Option<MapContentType>,
        content_code: Option<&str>,
    ) -> Result<Vec<MapSchema>, Error<GetAllMapsMapsGetError>> {
        let mut maps: Vec<MapSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_maps_maps_get(
                &self.configuration,
                content_type,
                content_code,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    maps.extend(resp.data);
                    if let Some(Some(pages)) = resp.pages {
                        if current_page >= pages {
                            finished = true
                        }
                        current_page += 1;
                    } else {
                        // No pagination information, assume single page
                        finished = true
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(maps)
    }

    pub fn info(&self, x: i32, y: i32) -> Result<MapResponseSchema, Error<GetMapMapsXyGetError>> {
        get_map_maps_xy_get(&self.configuration, x, y)
    }
}
