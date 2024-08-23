use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        resources_api::{
            get_all_resources_resources_get, get_resource_resources_code_get,
            GetAllResourcesResourcesGetError, GetResourceResourcesCodeGetError,
        },
        Error,
    },
    models::{DataPageResourceSchema, ResourceResponseSchema},
};

pub struct ResourcesApi {
    configuration: Configuration,
}

impl ResourcesApi {
    pub fn new(base_path: &str, token: &str) -> ResourcesApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        ResourcesApi { configuration }
    }

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        skill: Option<&str>,
        drop: Option<&str>,
        page: Option<i32>,
        size: Option<i32>,
    ) -> Result<DataPageResourceSchema, Error<GetAllResourcesResourcesGetError>> {
        get_all_resources_resources_get(
            &self.configuration,
            min_level,
            max_level,
            skill,
            drop,
            page,
            size,
        )
    }

    pub fn info(
        &self,
        code: &str,
    ) -> Result<ResourceResponseSchema, Error<GetResourceResourcesCodeGetError>> {
        get_resource_resources_code_get(&self.configuration, code)
    }
}
