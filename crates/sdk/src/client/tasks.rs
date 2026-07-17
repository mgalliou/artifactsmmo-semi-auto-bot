use crate::{Cached, TasksRewardsClient, entities::Task};
type TasksSource = Box<dyn Fn() -> HashMap<String, Task> + Send + Sync + 'static>;

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
    cache_dir: Box<str>,
    data: ArcSwap<HashMap<String, Task>>,
    fetch: TasksSource,
    rewards: TasksRewardsClient,
}

impl TasksClient {
    #[must_use]
    pub(crate) fn new(cache_dir: &str, fetch: TasksSource, rewards: TasksRewardsClient) -> Self {
        Self(Arc::new(TasksClientInner {
            cache_dir: cache_dir.into(),
            data: ArcSwap::default(),
            fetch,
            rewards,
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

    fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    fn fetch_from_source(&self) -> HashMap<String, Task> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
