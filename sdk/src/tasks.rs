use crate::{PersistedData, API};
use artifactsmmo_openapi::models::TaskFullSchema;
use std::sync::LazyLock;

pub static TASKS: LazyLock<Tasks> = LazyLock::new(Tasks::new);

pub struct Tasks(Vec<TaskFullSchema>);

impl PersistedData<Vec<TaskFullSchema>> for Tasks {
    fn data_from_api() -> Vec<TaskFullSchema> {
        API.tasks.all(None, None, None, None).unwrap()
    }

    fn path() -> &'static str {
        ".cache/tasks.json"
    }
}

impl Tasks {
    fn new() -> Self {
        Self(Self::get_data())
    }

    pub fn all(&self) -> &Vec<TaskFullSchema> {
        &self.0
    }
}
