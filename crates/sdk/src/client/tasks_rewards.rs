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
    directory: Box<str>,
    data: ArcSwap<HashMap<String, TaskReward>>,
    fetch: Box<dyn Fn() -> HashMap<String, TaskReward> + Send + Sync>,
}

impl Default for TasksRewardsClientInner {
    fn default() -> Self {
        Self {
            directory: ".cache".into(),
            data: ArcSwap::default(),
            fetch: Box::new(|| panic!("TasksRewardsClient not initialized")),
        }
    }
}

impl TasksRewardsClient {
    pub(crate) fn new(
        directory: &str,
        fetch: Box<dyn Fn() -> HashMap<String, TaskReward> + Send + Sync>,
    ) -> Self {
        Self(
            TasksRewardsClientInner {
                directory: directory.into(),
                data: ArcSwap::default(),
                fetch,
            }
            .into(),
        )
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

    fn directory(&self) -> &str {
        &self.directory
    }

    fn fetch_from_source(&self) -> HashMap<String, TaskReward> {
        (self.fetch)()
    }

    fn refresh(&self) {
        self.data.store(Arc::new(self.fetch_from_source()));
    }

}
