use super::{account::Account, api::monsters::MonstersApi};
use artifactsmmo_openapi::models::MonsterSchema;

pub struct Monsters {
    api: MonstersApi,
}

impl Monsters {
    pub fn new(account: &Account) -> Monsters {
        Monsters {
            api: MonstersApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn dropping(&self, code: &str) -> Option<Vec<MonsterSchema>> {
        self.api
            .all(None, None, Some(code), None, None)
            .ok()
            .map(|schemas| schemas.data)
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.api
            .all(Some(min), Some(level), None, None, None)
            .ok()?
            .data
            .into_iter()
            .min_by(|a, b| a.level.cmp(&b.level))
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<MonsterSchema> {
        self.api
            .all(None, Some(level), None, None, None)
            .ok()?
            .data
            .into_iter()
            .max_by(|a, b| a.level.cmp(&b.level))
    }
}
