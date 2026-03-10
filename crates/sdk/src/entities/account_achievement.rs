use chrono::{DateTime, FixedOffset};
use openapi::models::AccountAchievementSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::Code;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountAchievement(Arc<AccountAchievementSchema>);

impl AccountAchievement {
    pub(crate) fn new(schema: AccountAchievementSchema) -> Self {
        Self(schema.into())
    }

    pub fn is_completed(self) -> bool {
        self.completed_at().is_some()
    }

    pub fn completed_at(&self) -> Option<DateTime<FixedOffset>> {
        let date_string = self.0.completed_at.as_ref()?;
        DateTime::parse_from_rfc3339(date_string).ok()
    }
}

impl Code for AccountAchievement {
    fn code(&self) -> &str {
        &self.0.code
    }
}
