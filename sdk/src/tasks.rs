use crate::{PersistedData, API};
use artifactsmmo_api_wrapper::ArtifactApi;
use artifactsmmo_openapi::models::TaskFullSchema;
use itertools::Itertools;
use std::sync::{Arc, LazyLock, RwLock};

pub static TASKS: LazyLock<Tasks> = LazyLock::new(|| Tasks::new(API.clone()));

pub struct Tasks {
    data: RwLock<Vec<Arc<TaskFullSchema>>>,
    api: Arc<ArtifactApi>,
}

impl PersistedData<Vec<Arc<TaskFullSchema>>> for Tasks {
    const PATH: &'static str = ".cache/tasks.json";

    fn data_from_api(&self) -> Vec<Arc<TaskFullSchema>> {
        self.api
            .tasks
            .all(None, None, None, None)
            .unwrap()
            .into_iter()
            .map(Arc::new)
            .collect_vec()
    }

    fn refresh_data(&self) {
        *self.data.write().unwrap() = self.data_from_api();
    }
}

impl Tasks {
    fn new(api: Arc<ArtifactApi>) -> Self {
        let tasks = Self {
            data: Default::default(),
            api,
        };
        *tasks.data.write().unwrap() = tasks.retrieve_data();
        tasks
    }

    pub fn all(&self) -> Vec<Arc<TaskFullSchema>> {
        self.data.read().unwrap().iter().cloned().collect_vec()
    }
}
