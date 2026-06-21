use crate::{CollectionClient, Persist, entities::TaskReward};
use api::ArtifactApi;
use arc_swap::ArcSwap;
use derive_more::Deref;
use log::info;
use std::{collections::HashMap, sync::Arc};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
#[element(TaskReward)]
pub struct TasksRewardsClient(Arc<TasksRewardsClientInner>);

#[derive(Default, Debug)]
pub struct TasksRewardsClientInner {
    api: ArtifactApi,
    data: ArcSwap<HashMap<String, TaskReward>>,
}

impl TasksRewardsClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            TasksRewardsClientInner {
                api,
                data: ArcSwap::default(),
            }
            .into(),
        )
    }

    pub fn init(&self) {
        self.0.data.store(Arc::new(self.load()));
        info!("Tasks rewards client initilized");
    }

    #[must_use]
    pub fn max_quantity(&self) -> u32 {
        self.max_by_key(TaskReward::max_quantity)
            .map_or(0, |r| r.max_quantity())
    }
}

impl Persist<HashMap<String, TaskReward>> for TasksRewardsClient {
    const PATH: &'static str = ".cache/tasks_rewards.json";

    fn load_from_api(&self) -> HashMap<String, TaskReward> {
        self.api
            .tasks
            .get_rewards()
            .unwrap()
            .into_iter()
            .map(|tr| (tr.code.clone(), TaskReward::new(tr)))
            .collect()
    }

    fn refresh(&self) {
        self.0.data.store(Arc::new(self.load_from_api()));
    }
}
