use artifactsmmo_openapi::{apis::{characters_api::{get_character_characters_name_get, GetCharacterCharactersNameGetError}, configuration::Configuration, Error}, models::CharacterResponseSchema};

#[derive(Clone)]
pub struct CharactersApi {
    pub configuration: Configuration,
}

impl CharactersApi {
    pub fn new(base_path: &str, token: &str) -> CharactersApi {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        CharactersApi { configuration }
    }

    pub fn get(&self, name: &str) -> Result<CharacterResponseSchema, Error<GetCharacterCharactersNameGetError>> {
        get_character_characters_name_get(&self.configuration, name)
    }
}
