use artifactsmmo_openapi::{apis::{configuration::Configuration, resources_api::{get_all_resources_resources_get, get_resource_resources_code_get, GetAllResourcesResourcesGetError, GetResourceResourcesCodeGetError}, Error}, models::{DataPageResourceSchema, ResourceResponseSchema}};

pub struct ResourcesApi {
  configuration: Configuration
}

impl ResourcesApi{
    pub fn all(&self) -> Result<DataPageResourceSchema, Error<GetAllResourcesResourcesGetError>> {
        get_all_resources_resources_get(&self.configuration, None, None, None, None, None, None)
    }

    pub fn info(&self, code: &str) -> Result<ResourceResponseSchema, Error<GetResourceResourcesCodeGetError>> {
        get_resource_resources_code_get(&self.configuration, code)
    }
}
