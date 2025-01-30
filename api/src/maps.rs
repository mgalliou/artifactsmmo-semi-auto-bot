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
use std::sync::Arc;

pub struct MapsApi {
    configuration: Arc<Configuration>,
}

impl MapsApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
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
