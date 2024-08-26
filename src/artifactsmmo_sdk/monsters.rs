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
}
