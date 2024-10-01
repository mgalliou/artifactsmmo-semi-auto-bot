use super::{
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    character::Character,
    config::Config,
    game::Game,
};
use crate::artifactsmmo_sdk::char_config::CharConfig;
use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration, my_characters_api::GetMyCharactersMyCharactersGetError, Error,
    },
    models::CharacterSchema,
};
use itertools::Itertools;
use std::sync::Arc;
use std::sync::RwLock;

pub struct Account {
    pub configuration: Configuration,
    pub character_api: CharactersApi,
    pub my_characters_api: MyCharacterApi,
    pub characters: Arc<Vec<Arc<Character>>>,
}

impl Account {
    pub fn new(config: &Config, game: &Arc<Game>) -> Account {
        let mut configuration = Configuration::new();
        configuration.base_path = config.base_url.to_owned();
        configuration.bearer_access_token = Some(config.base_url.to_owned());
        let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
        let bank = Arc::new(Bank::new(config, &game.items));
        let chars_conf = init_char_conf(&config.characters);
        let chars_schema = init_chars_schema(config);
        let characters = chars_conf
            .into_iter()
            .zip(chars_schema.iter())
            .map(|(conf, schema)| {
                Arc::new(Character::new(
                    config,
                    game,
                    &bank,
                    &conf,
                    schema,
                ))
            })
            .collect_vec();
        Account {
            configuration,
            character_api: CharactersApi::new(&config.base_url, &config.token),
            my_characters_api,
            characters: Arc::new(characters),
        }
    }

    pub fn get_character(
        &self,
        index: usize,
    ) -> Result<CharacterSchema, Error<GetMyCharactersMyCharactersGetError>> {
        let chars = match self.my_characters_api.all() {
            Ok(c) => Ok(c.data),
            Err(e) => Err(e),
        };
        match chars {
            Ok(c) => Ok(c[index - 1].clone()),
            Err(e) => Err(e),
        }
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<Character>> {
        self.characters.iter().find(|c| c.name == name).cloned()
    }

    pub fn in_inventories(&self, code: &str) -> i32 {
        self.characters
            .iter()
            .cloned()
            .map(|c| c.has_in_inventory(code))
            .sum()
    }
}

fn init_char_conf(confs: &[CharConfig]) -> Vec<Arc<RwLock<CharConfig>>> {
    confs
        .iter()
        .map(|c| Arc::new(RwLock::new(c.clone())))
        .collect_vec()
}

fn init_chars_schema(config: &Config) -> Vec<Arc<RwLock<CharacterSchema>>> {
    let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
    my_characters_api
        .characters()
        .unwrap()
        .data
        .into_iter()
        .map(|s| Arc::new(RwLock::new(s)))
        .collect_vec()
}
