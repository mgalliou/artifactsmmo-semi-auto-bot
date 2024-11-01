use super::api::tasks::TasksApi;
use super::config::Config;
use artifactsmmo_openapi::models::{TaskFullSchema, TasksRewardFullSchema};

pub struct Tasks {
    pub api: TasksApi,
    pub list: Vec<TaskFullSchema>,
    pub rewards: Vec<TasksRewardFullSchema>,
}

impl Tasks {
    pub fn new(config: &Config) -> Self {
        let api = TasksApi::new(&config.base_url, &config.token);
        Self {
            list: api
                .all(None, None, None, None)
                .expect("tasks to be retrieved from API."),
            rewards: api.rewards().expect("tasks rewards to be retrieved from API."),
            api,
        }
    }
}
