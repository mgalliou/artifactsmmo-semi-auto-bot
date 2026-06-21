use crate::{Persist, TasksRewardsClient, entities::Task};
use api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc, thread};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(Task)]
pub struct TasksClient(Arc<TasksClientInner>);

#[derive(Default, Debug)]
pub struct TasksClientInner {
    api: ArtifactApi,
    data: ArcSwap<HashMap<String, Task>>,
    rewards: TasksRewardsClient,
}

impl TasksClient {
    pub(crate) fn new(api: ArtifactApi, reward: TasksRewardsClient) -> Self {
        Self(
            TasksClientInner {
                api,
                data: ArcSwap::default(),
                rewards: reward,
            }
            .into(),
        )
    }

    pub fn init(&self) {
        let () = thread::scope(|s| {
            let _ = s.spawn(|| self.0.data.store(Arc::new(self.load())));
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
        self.0.data.store(Arc::new(self.load_from_api()));
    }
}
