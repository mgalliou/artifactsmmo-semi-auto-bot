use std::path::Path;

use super::config::Config;
use super::retreive_data;
use super::{api::tasks::TasksApi, persist_data};
use artifactsmmo_openapi::models::{DropRateSchema, TaskFullSchema};
use log::error;

pub struct Tasks {
    pub api: TasksApi,
    pub list: Vec<TaskFullSchema>,
    pub rewards: Vec<DropRateSchema>,
}

impl Tasks {
    pub fn new(config: &Config) -> Self {
        let api = TasksApi::new(&config.base_url, &config.token);
        let tasks_path = Path::new(".cache/tasks.json");
        let list = if let Ok(data) = retreive_data::<Vec<TaskFullSchema>>(tasks_path) {
            data
        } else {
            let data = api
                .all(None, None, None, None)
                .expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, tasks_path) {
                error!("failed to persist tasks data: {}", e);
            }
            data
        };
        let rewards_path = Path::new(".cache/task_rewards.json");
        let rewards = if let Ok(data) = retreive_data::<Vec<DropRateSchema>>(rewards_path) {
            data
        } else {
            let data = api.rewards().expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, rewards_path) {
                error!("failed to persist tasks reward data: {}", e);
            }
            data
        };
        Self { list, rewards, api }
    }
}
