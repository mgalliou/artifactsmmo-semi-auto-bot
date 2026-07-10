use crate::{Cached, CollectionClient, entities::TaskReward};
type TasksRewardsSource = Box<dyn Fn() -> HashMap<String, TaskReward> + Send + Sync + 'static>;

use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(TaskReward)]
pub struct TasksRewardsClient(Arc<TasksRewardsClientInner>);

pub struct TasksRewardsClientInner {
    cache_dir: Box<str>,
    data: ArcSwap<HashMap<String, TaskReward>>,
    fetch: TasksRewardsSource,
}

impl TasksRewardsClient {
    #[must_use]
    pub(crate) fn new(
        cache_dir: &str,
        fetch: TasksRewardsSource,
    ) -> Self {
        Self(Arc::new(TasksRewardsClientInner {
            cache_dir: cache_dir.into(),
            data: ArcSwap::default(),
            fetch,
        }))
    }

    pub fn init(&self) {
        self.data.store(Arc::new(self.fetch()));
        info!("Tasks rewards client initilized");
    }

    #[must_use]
    pub fn max_quantity(&self) -> u32 {
        self.max_by_key(TaskReward::max_quantity)
            .map_or(0, |r| r.max_quantity())
    }
}

impl Cached<HashMap<String, TaskReward>> for TasksRewardsClient {
    const FILE: &'static str = "tasks_rewards";

    fn cache_dir(&self) -> &str {
        &self.cache_dir
    }

    fn fetch_from_source(&self) -> HashMap<String, TaskReward> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }
}
