use crate::{PersistedData, API};
use artifactsmmo_openapi::models::DropRateSchema;
use itertools::Itertools;
use std::sync::{Arc, LazyLock, RwLock};

pub static TASKS_REWARDS: LazyLock<TasksRewards> = LazyLock::new(TasksRewards::new);

pub struct TasksRewards(RwLock<Vec<Arc<DropRateSchema>>>);

impl PersistedData<Vec<Arc<DropRateSchema>>> for TasksRewards {
    const PATH: &'static str = ".cache/tasks_rewards.json";

    fn data_from_api() -> Vec<Arc<DropRateSchema>> {
        API.tasks
            .rewards()
            .unwrap()
            .into_iter()
            .map(Arc::new)
            .collect_vec()
    }

    fn refresh_data(&self) {
        *self.0.write().unwrap() = Self::data_from_api();
    }
}

impl TasksRewards {
    fn new() -> Self {
        Self(RwLock::new(Self::retrieve_data()))
    }

    pub fn all(&self) -> Vec<Arc<DropRateSchema>> {
        self.0.read().unwrap().iter().cloned().collect_vec()
    }
}
