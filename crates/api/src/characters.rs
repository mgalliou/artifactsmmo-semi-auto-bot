use openapi::{
    apis::{
        Error,
        characters_api::{GetCharacterCharactersNameGetError, get_character_characters_name_get},
        configuration::Configuration,
    },
    models::CharacterResponseSchema,
};
use std::sync::Arc;

#[derive(Default, Debug)]
pub struct CharactersApi {
    configuration: Arc<Configuration>,
}

impl CharactersApi {
    pub(crate) fn new(configuration: Arc<Configuration>) -> Self {
        Self { configuration }
    }

    pub fn get(
        &self,
        name: &str,
    ) -> Result<CharacterResponseSchema, Error<GetCharacterCharactersNameGetError>> {
        get_character_characters_name_get(&self.configuration, name)
    }
}
