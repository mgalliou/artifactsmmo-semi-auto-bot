use crate::{CollectionClient, Data, DataEntity, Persist, entities::TaskReward};
use api::ArtifactApi;
use derive_more::Deref;
use log::info;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default, Debug, Clone, Deref, CollectionClient)]
#[deref(forward)]
pub struct TasksRewardsClient(Arc<TasksRewardsClientInner>);

#[derive(Default, Debug)]
pub struct TasksRewardsClientInner {
    api: ArtifactApi,
    data: RwLock<Arc<HashMap<String, TaskReward>>>,
}

impl TasksRewardsClient {
    pub(crate) fn new(api: ArtifactApi) -> Self {
        Self(
            TasksRewardsClientInner {
                api,
                data: RwLock::default(),
            }
            .into(),
        )
    }

    pub fn init(&self) {
        *self.data_mut() = Arc::new(self.load());
        info!("Tasks rewards client initilized");
    }

    pub fn max_quantity(&self) -> u32 {
        self.all()
            .iter()
            .max_by_key(|i| i.max_quantity())
            .map_or(0, TaskReward::max_quantity)
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
        *self.data_mut() = Arc::new(self.load_from_api());
    }
}

impl DataEntity for TasksRewardsClient {
    type Entity = TaskReward;
}
