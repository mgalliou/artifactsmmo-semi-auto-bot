use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        maps_api::{
            get_all_maps_maps_get, get_map_maps_xy_get, GetAllMapsMapsGetError,
            GetMapMapsXyGetError,
        },
        Error,
    },
    models::{DataPageMapSchema, MapResponseSchema},
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
        content_type: Option<&str>,
        content_code: Option<&str>,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Result<DataPageMapSchema, Error<GetAllMapsMapsGetError>> {
        get_all_maps_maps_get(&self.configuration, content_type, content_code, page, size)
    }

    pub fn info(&self, x: i32, y: i32) -> Result<MapResponseSchema, Error<GetMapMapsXyGetError>> {
        get_map_maps_xy_get(&self.configuration, x, y)
    }
}
