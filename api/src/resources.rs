use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        resources_api::{
            get_all_resources_resources_get, get_resource_resources_code_get,
            GetAllResourcesResourcesGetError, GetResourceResourcesCodeGetError,
        },
        Error,
    },
    models::{GatheringSkill, ResourceResponseSchema, ResourceSchema},
};
use std::sync::Arc;

pub struct ResourcesApi {
    configuration: Arc<Configuration>,
}

impl ResourcesApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        skill: Option<GatheringSkill>,
        drop: Option<&str>,
    ) -> Result<Vec<ResourceSchema>, Error<GetAllResourcesResourcesGetError>> {
        let mut resources: Vec<ResourceSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_resources_resources_get(
                &self.configuration,
                min_level,
                max_level,
                skill,
                drop,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    resources.extend(resp.data);
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
        Ok(resources)
    }

    pub fn info(
        &self,
        code: &str,
    ) -> Result<ResourceResponseSchema, Error<GetResourceResourcesCodeGetError>> {
        get_resource_resources_code_get(&self.configuration, code)
    }
}
