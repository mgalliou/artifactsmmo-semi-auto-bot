use super::{account::Account, api::monsters::MonstersApi};
use artifactsmmo_openapi::models::MonsterSchema;

pub struct Monsters {
    pub data: Vec<MonsterSchema>,
}

impl Monsters {
    pub fn new(account: &Account) -> Monsters {
        let api = MonstersApi::new(
            &account.configuration.base_path,
            &account.configuration.bearer_access_token.clone().unwrap(),
        );
        Monsters {
            data: api.all(None, None, None).unwrap().clone(),
        }
    }

    pub fn dropping(&self, code: &str) -> Vec<&MonsterSchema> {
        self.data
            .iter()
            .filter(|m| m.drops.iter().any(|d| d.code == code))
            .collect::<Vec<_>>()
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|m| m.level >= min && m.level <= level)
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        self.data
            .iter()
            .filter(|m| m.level <= level)
            .max_by_key(|m| m.level)
    }

    pub fn get(&self, code: &str) -> Option<&MonsterSchema> {
        self.data.iter().find(|m| m.code == code)
    }
}
