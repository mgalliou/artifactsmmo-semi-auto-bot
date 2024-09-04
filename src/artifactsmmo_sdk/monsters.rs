use super::{account::Account, api::monsters::MonstersApi};
use artifactsmmo_openapi::models::MonsterSchema;
use itertools::Itertools;

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

    pub fn dropping(&self, code: &str) -> Option<Vec<&MonsterSchema>> {
        let monsters = self
            .data
            .iter()
            .filter(|m| m.drops.iter().any(|d| d.code == code))
            .collect_vec();
        match monsters.is_empty() {
            true => Some(monsters),
            false => None,
        }
    }

    pub fn lowest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        let min = if level > 11 { level - 10 } else { 1 };
        self.data
            .iter()
            .filter(|m| m.level >= min && m.level <= level)
            .into_iter()
            .min_by_key(|m| m.level)
    }

    pub fn highest_providing_exp(&self, level: i32) -> Option<&MonsterSchema> {
        self.data
            .iter()
            .filter(|m| m.level <= level)
            .into_iter()
            .max_by_key(|m| m.level)
    }

    pub fn get(&self, code: &str) -> Option<&MonsterSchema> {
        self.data.iter().find(|m| m.code == code)
    }
}
