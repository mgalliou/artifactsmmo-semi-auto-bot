use crate::{DataPage, Paginate};
use openapi::{
    apis::{
        Error,
        configuration::Configuration,
        maps_api::{
            GetAllMapsMapsGetError, GetMapByIdMapsIdMapIdGetError, get_all_maps_maps_get,
            get_map_by_id_maps_id_map_id_get,
        },
    },
    models::{DataPageMapSchema, MapResponseSchema, MapSchema},
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct MapsApi {
    configuration: Arc<Configuration>,
}

impl MapsApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get_all(&self) -> Result<Vec<MapSchema>, Error<GetAllMapsMapsGetError>> {
        MapsRequest {
            configuration: &self.configuration,
        }
        .send()
    }

    pub fn get_by_id(
        &self,
        id: i32,
    ) -> Result<MapResponseSchema, Error<GetMapByIdMapsIdMapIdGetError>> {
        get_map_by_id_maps_id_map_id_get(&self.configuration, id)
    }
}

struct MapsRequest<'a> {
    configuration: &'a Configuration,
}

impl<'a> Paginate for MapsRequest<'a> {
    type Data = MapSchema;
    type Page = DataPageMapSchema;
    type Error = GetAllMapsMapsGetError;

    fn request_page(&self, page: u32) -> Result<Self::Page, Error<Self::Error>> {
        get_all_maps_maps_get(
            self.configuration,
            None,
            None,
            None,
            None,
            Some(page),
            Some(100),
        )
    }
}

impl DataPage<MapSchema> for DataPageMapSchema {
    fn data(self) -> Vec<MapSchema> {
        self.data
    }

    fn pages(&self) -> Option<u32> {
        self.pages
    }
}
