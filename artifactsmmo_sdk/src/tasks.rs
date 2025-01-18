use crate::{persist_data, retreive_data, API};
use artifactsmmo_openapi::models::{DropRateSchema, TaskFullSchema};
use lazy_static::lazy_static;
use log::error;
use std::{path::Path, sync::Arc};

lazy_static! {
    pub static ref TASKS: Arc<Tasks> = Arc::new(Tasks::new());
}

#[derive(Default)]
pub struct Tasks {
    pub list: Vec<TaskFullSchema>,
    pub rewards: Vec<DropRateSchema>,
}

impl Tasks {
    fn new() -> Self {
        let tasks_path = Path::new(".cache/tasks.json");
        let list = if let Ok(data) = retreive_data::<Vec<TaskFullSchema>>(tasks_path) {
            data
        } else {
            let data = API
                .tasks
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
            let data = API
                .tasks
                .rewards()
                .expect("items to be retrieved from API.");
            if let Err(e) = persist_data(&data, rewards_path) {
                error!("failed to persist tasks reward data: {}", e);
            }
            data
        };
        Self { list, rewards }
    }
}
