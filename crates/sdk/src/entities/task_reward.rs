use openapi::models::DropRateSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::Code;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TaskReward(Arc<DropRateSchema>);

impl TaskReward {
    pub(crate) fn new(schema: DropRateSchema) -> Self {
        Self(schema.into())
    }

    #[must_use] 
    pub fn max_quantity(&self) -> u32 {
        self.0.max_quantity
    }

    #[must_use] 
    pub fn min_quantity(&self) -> u32 {
        self.0.min_quantity
    }
}

impl Code for TaskReward {
    fn code(&self) -> &str {
        &self.0.code
    }
}
