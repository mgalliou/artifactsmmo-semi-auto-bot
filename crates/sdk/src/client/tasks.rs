use crate::{Data, DataEntity, Persist, TasksRewardsClient, entities::Task};
use api::ArtifactApi;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    thread,
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
pub struct TasksClient(Arc<TasksClientInner>);

#[derive(Default, Debug)]
pub struct TasksClientInner {
    api: ArtifactApi,
    data: RwLock<HashMap<String, Task>>,
    rewards: TasksRewardsClient,
}

impl TasksClient {
    pub(crate) fn new(api: ArtifactApi, reward: TasksRewardsClient) -> Self {
        Self(
            TasksClientInner {
                api,
                data: RwLock::default(),
                rewards: reward,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        let () = thread::scope(|s| {
            let _ = s.spawn(|| *self.data_mut() = self.load());
            let _ = s.spawn(|| self.rewards().init());
        });
        info!("Tasks client initilized");
    }

    #[must_use]
    pub fn rewards(&self) -> TasksRewardsClient {
        self.rewards.clone()
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
        *self.data_mut() = self.load_from_api();
    }
}

impl DataEntity for TasksClient {
    type Entity = Task;
}
