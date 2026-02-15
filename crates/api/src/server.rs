use openapi::{
    apis::{configuration::Configuration, server_details_api::get_server_details_get},
    models::StatusResponseSchema,
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct ServerApi {
    configuration: Arc<Configuration>,
}

impl ServerApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    //TODO: return result
    pub fn status(&self) -> Option<StatusResponseSchema> {
        get_server_details_get(&self.configuration).ok()
    }
}
