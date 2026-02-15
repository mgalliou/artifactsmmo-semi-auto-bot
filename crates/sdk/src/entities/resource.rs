use crate::{CanProvideXp, Code, DropsItems, Level, Skill};
use openapi::models::{DropRateSchema, ResourceSchema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resource(Arc<ResourceSchema>);

impl Resource {
    pub fn new(schema: ResourceSchema) -> Self {
        Self(Arc::new(schema))
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn skill(&self) -> Skill {
        self.0.skill.into()
    }
}

impl DropsItems for Resource {
    fn drops(&self) -> &Vec<DropRateSchema> {
        &self.0.drops
    }
}

impl Code for Resource {
    fn code(&self) -> &str {
        &self.0.code
    }
}

impl Level for Resource {
    fn level(&self) -> u32 {
        self.0.level as u32
    }
}

impl CanProvideXp for Resource {}
