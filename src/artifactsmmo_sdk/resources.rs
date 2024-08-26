use artifactsmmo_openapi::models::ResourceSchema;

use super::{account::Account, api::resources::ResourcesApi};

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

    pub fn dropping(&self, code: &str) -> Option<Vec<String>> {
        let mut codes: Vec<String> = vec![];

        if let Ok(resources) = self.api.all(None, None, None, Some(code), None, None) {
            for r in resources.data {
                codes.push(r.code)
            }
            return Some(codes);
        }
        None
    }

    pub fn below_or_equal(&self, level: i32, skill: &str) -> Option<ResourceSchema> {
        let mut highest_lvl = 0;
        let mut best_schema: Option<ResourceSchema> = None;

        match self.api.all(None, Some(level), Some(skill), None, None, None) {
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
