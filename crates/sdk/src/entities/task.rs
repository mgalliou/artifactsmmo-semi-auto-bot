use openapi::models::{RewardsSchema, TaskFullSchema, TaskType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::Code;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Task(Arc<TaskFullSchema>);

impl Task {
    pub fn new(schema: TaskFullSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn r#type(&self) -> TaskType {
        self.0.r#type
    }

    pub fn rewards_quantity(&self) -> u32 {
        self.rewards().items.iter().map(|i| i.quantity).sum()
    }

    pub fn rewards_slots(&self) -> u32 {
        self.rewards().items.len() as u32
    }

    pub fn rewards(&self) -> &RewardsSchema {
        self.0.rewards.as_ref()
    }
}

impl Code for Task {
    fn code(&self) -> &str {
        &self.0.code
    }
}
