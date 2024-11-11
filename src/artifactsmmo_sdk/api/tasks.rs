use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        tasks_api::{
            get_all_tasks_rewards_tasks_rewards_get, get_all_tasks_tasks_list_get,
            GetAllTasksRewardsTasksRewardsGetError, GetAllTasksTasksListGetError,
        },
        Error,
    },
    models::{DropRateSchema, Skill, TaskFullSchema, TaskType},
};

pub struct TasksApi {
    pub configuration: Configuration,
}

impl TasksApi {
    pub fn new(base_path: &str, token: &str) -> Self {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        Self { configuration }
    }

    pub fn all(
        &self,
        min_level: Option<i32>,
        max_level: Option<i32>,
        skill: Option<Skill>,
        r#type: Option<TaskType>,
    ) -> Result<Vec<TaskFullSchema>, Error<GetAllTasksTasksListGetError>> {
        let mut tasks: Vec<TaskFullSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_tasks_tasks_list_get(
                &self.configuration,
                min_level,
                max_level,
                skill,
                r#type,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    tasks.extend(resp.data);
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
        Ok(tasks)
    }

    pub fn rewards(
        &self,
    ) -> Result<Vec<DropRateSchema>, Error<GetAllTasksRewardsTasksRewardsGetError>> {
        let mut drops: Vec<DropRateSchema> = vec![];
        let mut current_page = 1;
        let mut finished = false;
        while !finished {
            let resp = get_all_tasks_rewards_tasks_rewards_get(
                &self.configuration,
                Some(current_page),
                Some(100),
            );
            match resp {
                Ok(resp) => {
                    drops.extend(resp.data);
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
        Ok(drops)
    }
}
