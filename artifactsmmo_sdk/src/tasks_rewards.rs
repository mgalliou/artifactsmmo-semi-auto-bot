use crate::{PersistedData, API};
use artifactsmmo_openapi::models::DropRateSchema;
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    pub static ref TASKS_REWARDS: Arc<TasksRewards> = Arc::new(TasksRewards::new());
}

pub struct TasksRewards(Vec<DropRateSchema>);

impl PersistedData<Vec<DropRateSchema>> for TasksRewards {
    fn data_from_api() -> Vec<DropRateSchema> {
        API.tasks.rewards().unwrap()
    }

    fn path() -> &'static str {
        ".cache/tasks_rewards.json"
    }
}

impl TasksRewards {
    fn new() -> Self {
        Self(Self::get_data())
    }

    pub fn all(&self) -> &Vec<DropRateSchema> {
        &self.0
    }
}
