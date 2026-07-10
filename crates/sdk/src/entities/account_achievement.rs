use crate::Code;
use chrono::{DateTime, FixedOffset};
use openapi::models::AccountAchievementSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountAchievement(Arc<AccountAchievementSchema>);

impl AccountAchievement {
    #[must_use]
    pub(crate) fn new(schema: AccountAchievementSchema) -> Self {
        Self(Arc::new(schema))
    }

    #[must_use]
    pub fn is_completed(self) -> bool {
        self.completed_at().is_some()
    }

    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<FixedOffset>> {
        self.0.completed_at
    }
}

impl Code for AccountAchievement {
    fn code(&self) -> &str {
        &self.0.code
    }
}
