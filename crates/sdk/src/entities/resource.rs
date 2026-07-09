use crate::{CanProvideXp, Code, DropRateSchemaExt, HasDropTable, Level, Skill};
use openapi::models::ResourceSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resource(Arc<ResourceSchema>);

impl Resource {
    pub(crate) fn new(schema: ResourceSchema) -> Self {
        Self(schema.into())
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    #[must_use]
    pub fn skill(&self) -> Skill {
        self.0.skill.into()
    }
}

impl HasDropTable for Resource {
    fn drops(&self) -> &[impl DropRateSchemaExt] {
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
