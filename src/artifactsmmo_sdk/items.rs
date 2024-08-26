use artifactsmmo_openapi::models::{craft_schema::Skill, CraftSchema, SimpleItemSchema};

use super::{account::Account, api::items::ItemsApi};


pub struct Items {
    api: ItemsApi,
}

impl Items {
    pub fn new(account: &Account) -> Items {
        Items {
            api: ItemsApi::new(
                &account.configuration.base_path,
                &account.configuration.bearer_access_token.clone().unwrap(),
            ),
        }
    }

    pub fn craft_schema(&self, code: &str) -> Option<CraftSchema> {
        if let Ok(info) = self.api.info(code) {
            if let Some(Some(craft)) = info.data.item.craft {
                return Some(*craft)
            }
        };
        None
    }

    pub fn mats_for(&self, code: &str) -> Option<Vec<SimpleItemSchema>> {
        match self.craft_schema(code) {
            Some(schema) => schema.items,
            None => None,
        }
    }

    pub fn skill_to_craft(&self, code: &str) -> Option<Skill> {
        match self.craft_schema(code) {
            Some(schema) => schema.skill,
            None => None,
        }
    }
}
