use artifactsmmo_openapi::models::ResourceSchema;

use super::{account::Account, api::resources::ResourcesApi, skill::Skill};

pub struct Resources {
    api: ResourcesApi,
}

impl Resources {
    pub fn new(account: &Account) -> Resources {
        Resources {
            api: ResourcesApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn dropping(&self, code: &str) -> Option<Vec<ResourceSchema>> {
        self.api.all(None, None, None, Some(code), None, None)
            .ok()
            .map(|schemas| schemas.data)
    }

    pub fn lowest_providing_exp(&self, level: i32, skill: Skill) -> Option<ResourceSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.api
            .all(Some(min), Some(level), Some(&skill.to_string()), None, None, None)
            .ok()?
            .data
            .into_iter()
            .min_by(|a, b| a.level.cmp(&b.level))
    }

    pub fn highest_providing_exp(&self, level: i32, skill: Skill) -> Option<ResourceSchema> {
        self.api
            .all(
                None,
                Some(level),
                Some(&skill.to_string()),
                None,
                None,
                None,
            )
            .ok()?
            .data
            .into_iter()
            .max_by(|a, b| a.level.cmp(&b.level))
    }
}
