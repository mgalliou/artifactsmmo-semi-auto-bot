use crate::{Cached, CollectionClient, entities::TaskReward};
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Deref, Default, CollectionClient)]
#[deref(forward)]
#[element(TaskReward)]
pub struct TasksRewardsClient(Arc<TasksRewardsClientInner>);

pub struct TasksRewardsClientInner {
    path: Box<str>,
    data: ArcSwap<HashMap<String, TaskReward>>,
    fetch: Box<dyn Fn() -> HashMap<String, TaskReward> + Send + Sync>,
}

impl Default for TasksRewardsClientInner {
    fn default() -> Self {
        Self {
            path: Box::from(".cache/tasks_rewards.json"),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("TasksRewardsClient not initialized")),
        }
    }
}

impl TasksRewardsClient {
    pub(crate) fn new(
        path: &str,
        fetch: Box<dyn Fn() -> HashMap<String, TaskReward> + Send + Sync>,
    ) -> Self {
        Self(
            TasksRewardsClientInner {
                path: path.into(),
                fetch,
                data: ArcSwap::default(),
            }
            .into(),
        )
    }

    #[must_use]
    pub fn from_cache(path: &str) -> Self {
        let client = Self::new(
            path,
            Box::new(|| unreachable!("TasksRewardsClient::from_cache has no API fallback")),
        );
        client.init();
        client
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.fetch()));
        info!("Tasks rewards client initilized");
    }

    #[must_use]
    pub fn max_quantity(&self) -> u32 {
        self.max_by_key(TaskReward::max_quantity)
            .map_or(0, |r| r.max_quantity())
    }
}

impl Cached<HashMap<String, TaskReward>> for TasksRewardsClient {
    fn path(&self) -> &str {
        &self.path
    }

    fn fetch_from_source(&self) -> HashMap<String, TaskReward> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.fetch_from_source()));
    }
}
