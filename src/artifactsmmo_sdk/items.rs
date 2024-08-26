use artifactsmmo_openapi::models::{
    craft_schema::Skill, CraftSchema, ItemSchema, SimpleItemSchema,
};

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

    pub fn best_craftable_at_level(&self, level: i32, skill: &str) -> Option<Vec<ItemSchema>> {
        let mut highest_lvl = 0;
        let mut best_schemas: Vec<ItemSchema> = vec![];

        match self
            .api
            .all(None, Some(level), None, None, Some(skill), None, None, None)
        {
            Ok(schemas) => { 
                for schema in schemas.data {
                    if highest_lvl == 0 || highest_lvl <= schema.level {
                        highest_lvl = schema.level;
                        best_schemas.push(schema);
                    }
                };
                Some(best_schemas)
            },
            _ => None,
        }
    }

    pub fn craft_schema(&self, code: &str) -> Option<CraftSchema> {
        if let Ok(info) = self.api.info(code) {
            if let Some(Some(craft)) = info.data.item.craft {
                return Some(*craft);
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
