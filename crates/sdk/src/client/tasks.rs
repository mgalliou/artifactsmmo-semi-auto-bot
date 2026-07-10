use crate::{Cached, TasksRewardsClient, entities::Task};
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc, thread};

#[derive(Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(Task)]
pub struct TasksClient(Arc<TasksClientInner>);

pub struct TasksClientInner {
    directory: Box<str>,
    data: ArcSwap<HashMap<String, Task>>,
    fetch: Box<dyn Fn() -> HashMap<String, Task> + Send + Sync>,
    rewards: TasksRewardsClient,
}

impl TasksClient {
    #[must_use]
    pub(crate) fn new(
        directory: &str,
        fetch: Box<dyn Fn() -> HashMap<String, Task> + Send + Sync>,
        reward: TasksRewardsClient,
    ) -> Self {
        Self(Arc::new(TasksClientInner {
            directory: directory.into(),
            fetch,
            data: ArcSwap::default(),
            rewards: reward,
        }))
    }

    pub fn init(&self) {
        let () = thread::scope(|s| {
            let _ = s.spawn(|| self.data.store(Arc::new(self.fetch())));
            let _ = s.spawn(|| self.rewards().init());
        });
        info!("Tasks client initilized");
    }

    #[must_use]
    pub fn rewards(&self) -> TasksRewardsClient {
        self.rewards.clone()
    }
}

impl Cached<HashMap<String, Task>> for TasksClient {
    const FILE: &'static str = "tasks";

    fn directory(&self) -> &str {
        &self.directory
    }

    fn fetch_from_source(&self) -> HashMap<String, Task> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
