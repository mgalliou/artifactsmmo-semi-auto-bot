use crate::{PersistedData, API};
use artifactsmmo_openapi::models::TaskFullSchema;
use itertools::Itertools;
use std::sync::{Arc, LazyLock, RwLock};

pub static TASKS: LazyLock<Tasks> = LazyLock::new(Tasks::new);

pub struct Tasks(RwLock<Vec<Arc<TaskFullSchema>>>);

impl PersistedData<Vec<Arc<TaskFullSchema>>> for Tasks {
    const PATH: &'static str = ".cache/tasks.json";

    fn data_from_api() -> Vec<Arc<TaskFullSchema>> {
        API.tasks
            .all(None, None, None, None)
            .unwrap()
            .into_iter()
            .map(Arc::new)
            .collect_vec()
    }

    fn refresh_data(&self) {
        *self.0.write().unwrap() = Self::data_from_api();
    }
}

impl Tasks {
    fn new() -> Self {
        Self(RwLock::new(Self::retrieve_data()))
    }

    pub fn all(&self) -> Vec<Arc<TaskFullSchema>> {
        self.0.read().unwrap().iter().cloned().collect_vec()
    }
}
