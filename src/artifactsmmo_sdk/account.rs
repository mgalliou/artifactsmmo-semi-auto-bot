use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        items_api::{
            get_all_items_items_get, get_item_items_code_get, GetAllItemsItemsGetError,
            GetItemItemsCodeGetError,
        },
        my_characters_api::{
            get_my_characters_my_characters_get, GetMyCharactersMyCharactersGetError,
        },
        Error,
    },
    models::{CharacterSchema, DataPageItemSchema, ItemResponseSchema},
};

use super::character::Character;

#[derive(Clone)]
pub struct Account {
    pub configuration: Configuration,
}

impl Account {
    pub fn new(base_url: &str, token: &str) -> Account {
        let mut configuration = Configuration::new();
        configuration.base_path = base_url.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        Account { configuration }
    }

    pub fn get_character(
        &self,
        index: usize,
    ) -> Result<Character, Error<GetMyCharactersMyCharactersGetError>> {
        let chars = match get_my_characters_my_characters_get(&self.configuration) {
            Ok(c) => Ok(c.data),
            Err(e) => Err(e),
        };
        match chars {
            Ok(c) => Ok(Character::from_schema(c[index - 1].clone(), self.clone())),
            Err(e) => Err(e),
        }
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<CharacterSchema> {
        let chars = get_my_characters_my_characters_get(&self.configuration).unwrap();
        chars.data.iter().find(|c| c.name == name).cloned()
    }

    pub fn get_all_items(&self) -> Result<DataPageItemSchema, Error<GetAllItemsItemsGetError>> {
        get_all_items_items_get(
            &self.configuration,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    pub fn get_item_info(
        &self,
        code: &str,
    ) -> Result<ItemResponseSchema, Error<GetItemItemsCodeGetError>> {
        get_item_items_code_get(&self.configuration, code)
    }
}
