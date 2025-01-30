use crate::{PersistedData, API};
use artifactsmmo_openapi::models::DropRateSchema;
use std::sync::LazyLock;

pub static TASKS_REWARDS: LazyLock<TasksRewards> = LazyLock::new(TasksRewards::new);

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
