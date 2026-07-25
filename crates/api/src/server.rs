use openapi::{
    apis::{configuration::Configuration, server_details_api::get_server_details_get},
    models::StatusResponseSchema,
};
use std::sync::Arc;
use crate::RUNTIME;

#[derive(Default, Debug)]
pub struct ServerApi {
    configuration: Arc<Configuration>,
}

impl ServerApi {
    pub(crate) const fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    //TODO: return result
    #[must_use]
    pub fn status(&self) -> Option<StatusResponseSchema> {
        RUNTIME
            .block_on(get_server_details_get(&self.configuration))
            .ok()
    }
}
