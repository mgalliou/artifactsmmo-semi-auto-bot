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
        if let Ok(schemas) = self.api.all(None, None, Some(code), None, None) {
            return Some(schemas.data);
        }
        None
    }

    pub fn below_or_equal(&self, level: i32) -> Option<MonsterSchema> {
        let mut highest_lvl = 0;
        let mut best_schema: Option<MonsterSchema> = None;

        match self.api.all(None, Some(level), None, None, None) {
            Ok(schemas) => { 
                for schema in schemas.data {
                    if highest_lvl == 0 || highest_lvl < schema.level {
                        highest_lvl = schema.level;
                        best_schema = Some(schema);
                    }
                };
                best_schema
            },
            _ => None,
        }
    }
}
