use artifactsmmo_openapi::{
    apis::{configuration::Configuration, default_api::get_status_get},
    models::StatusResponseSchema,
};
use std::sync::Arc;

pub struct ServerApi {
    configuration: Arc<Configuration>,
}

impl ServerApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn status(&self) -> Option<StatusResponseSchema> {
        get_status_get(&self.configuration).ok()
    }
}
