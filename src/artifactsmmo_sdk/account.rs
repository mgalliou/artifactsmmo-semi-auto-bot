use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        my_characters_api::{
            get_my_characters_my_characters_get, GetMyCharactersMyCharactersGetError,
        },
        Error,
    },
    models::CharacterSchema,
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
}
