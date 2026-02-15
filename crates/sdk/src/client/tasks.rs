use crate::{DataEntity, Persist, TasksRewardsClient, entities::Task};
use api::ArtifactApi;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, CollectionClient)]
pub struct TasksClient {
    data: RwLock<HashMap<String, Task>>,
    pub reward: Arc<TasksRewardsClient>,
    api: Arc<ArtifactApi>,
}

impl TasksClient {
    pub(crate) fn new(api: Arc<ArtifactApi>, reward: Arc<TasksRewardsClient>) -> Self {
        let tasks = Self {
            data: Default::default(),
            reward,
            api,
        };
        *tasks.data.write().unwrap() = tasks.load();
        tasks
    }
}

impl Persist<HashMap<String, Task>> for TasksClient {
    const PATH: &'static str = ".cache/tasks.json";

    fn load_from_api(&self) -> HashMap<String, Task> {
        self.api
            .tasks
            .get_all()
            .unwrap()
            .into_iter()
            .map(|task| (task.code.clone(), Task::new(task)))
            .collect()
    }

    fn refresh(&self) {
        *self.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for TasksClient {
    type Entity = Task;
}
