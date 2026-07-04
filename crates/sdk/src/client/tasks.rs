use crate::{Cached, TasksRewardsClient, entities::Task};
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use sdk_derive::CollectionClient;
use std::{collections::HashMap, sync::Arc, thread};

#[derive(Clone, Default, Deref, CollectionClient)]
#[deref(forward)]
#[element(Task)]
pub struct TasksClient(Arc<TasksClientInner>);

pub struct TasksClientInner {
    path: Box<str>,
    data: ArcSwap<HashMap<String, Task>>,
    fetch: Box<dyn Fn() -> HashMap<String, Task> + Send + Sync>,
    rewards: TasksRewardsClient,
}

impl Default for TasksClientInner {
    fn default() -> Self {
        Self {
            path: Box::from(".cache/tasks.ron"),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("TasksClient not initialized")),
            rewards: TasksRewardsClient::default(),
        }
    }
}

impl TasksClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, Task> + Send + Sync>,
        reward: TasksRewardsClient,
    ) -> Self {
        Self(
            TasksClientInner {
                path: path.into(),
                fetch,
                data: ArcSwap::default(),
                rewards: reward,
            }
            .into(),
        )
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("TasksClient::from_cache has no API fallback")),
            TasksRewardsClient::default(),
        );
        client.init();
        client
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
    fn path(&self) -> &str {
        &self.path
    }

    fn fetch_from_source(&self) -> HashMap<String, Task> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
