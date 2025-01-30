use artifactsmmo_openapi::{
    apis::{
        characters_api::{get_character_characters_name_get, GetCharacterCharactersNameGetError},
        configuration::Configuration,
        Error,
    },
    models::CharacterResponseSchema,
};
use std::sync::Arc;

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
