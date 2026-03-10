use crate::{DataEntity, Persist, TasksRewardsClient, entities::Task};
use api::ArtifactApi;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, CollectionClient)]
pub struct TasksClient(Arc<TasksClientInner>);

#[derive(Default, Debug)]
pub struct TasksClientInner {
    api: ArtifactApi,
    data: RwLock<HashMap<String, Task>>,
    rewards: TasksRewardsClient,
}

impl TasksClient {
    pub(crate) fn new(api: ArtifactApi, reward: TasksRewardsClient) -> Self {
        let tasks = Self(
            TasksClientInner {
                api,
                data: RwLock::default(),
                rewards: reward,
            }
            .into(),
        );
        *tasks.0.data.write().unwrap() = tasks.load();
        tasks
    }

    pub fn rewards(&self) -> TasksRewardsClient {
        self.0.rewards.clone()
    }
}

impl Persist<HashMap<String, Task>> for TasksClient {
    const PATH: &'static str = ".cache/tasks.json";

    fn load_from_api(&self) -> HashMap<String, Task> {
        self.0
            .api
            .tasks
            .get_all()
            .unwrap()
            .into_iter()
            .map(|task| (task.code.clone(), Task::new(task)))
            .collect()
    }

    fn refresh(&self) {
        *self.0.data.write().unwrap() = self.load_from_api();
    }
}

impl DataEntity for TasksClient {
    type Entity = Task;
}
