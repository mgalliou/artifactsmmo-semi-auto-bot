use crate::{Code, Quantity};
use openapi::models::{RewardsSchema, TaskFullSchema, TaskType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task(Arc<TaskFullSchema>);

impl Task {
    #[must_use]
    pub(crate) fn new(schema: TaskFullSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn r#type(&self) -> TaskType {
        self.0.r#type
    }

    #[must_use]
    pub fn rewards_quantity(&self) -> u32 {
        self.rewards().items.iter().map(Quantity::quantity).sum()
    }

    #[must_use]
    pub fn rewards_slots(&self) -> u32 {
        self.rewards().items.len() as u32
    }

    #[must_use]
    pub fn rewards(&self) -> &RewardsSchema {
        &self.0.rewards
    }
}

impl Code for Task {
    fn code(&self) -> &str {
        &self.0.code
    }
}
