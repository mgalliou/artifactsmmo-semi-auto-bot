use std::sync::RwLock;

use super::api::{characters::CharactersApi, my_character::MyCharacterApi};
use artifactsmmo_openapi::{
    apis::{
        configuration::Configuration,
        default_api::{get_status_get, GetStatusGetError},
        my_characters_api::GetMyCharactersMyCharactersGetError,
        Error,
    },
    models::{CharacterSchema, StatusResponseSchema},
};
use chrono::{DateTime, TimeDelta, Utc};
use log::debug;

pub struct Account {
    pub configuration: Configuration,
    pub character_api: CharactersApi,
    pub my_characters_api: MyCharacterApi,
    pub server_offset: RwLock<TimeDelta>,
}

impl Account {
    pub fn new(base_path: &str, token: &str) -> Account {
        let mut configuration = Configuration::new();
        configuration.base_path = base_path.to_owned();
        configuration.bearer_access_token = Some(token.to_owned());
        let account = Account {
            configuration,
            character_api: CharactersApi::new(base_path, token),
            my_characters_api: MyCharacterApi::new(base_path, token),
            server_offset: RwLock::new(TimeDelta::default()),
        };
        account.update_offset();
        account
    }

    pub fn server_status(&self) -> Result<StatusResponseSchema, Error<GetStatusGetError>> {
        get_status_get(&self.configuration)
    }

    pub fn server_time(&self) -> Option<DateTime<Utc>> {
        match get_status_get(&self.configuration) {
            Ok(s) => match DateTime::parse_from_rfc3339(&s.data.server_time) {
                Ok(t) => Some(t.to_utc()),
                Err(_) => None,
            },
            Err(_) => None,
        }
    }

    pub fn update_offset(&self) {
        let server_time = self.server_time().unwrap();
        let now = Utc::now();
        let _ = self.server_offset.write().map(|mut so| *so = now - server_time);
        debug!("system time: {}", now);
        debug!("server time: {}", self.server_time().unwrap());
        debug!(
            "time offset: {}s and {}ms",
            self.server_offset.read().unwrap().num_seconds(),
            self.server_offset.read().unwrap().subsec_nanos() / 1000000
        );
        debug!("synced time: {}", now - *self.server_offset.read().unwrap());
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

    pub fn get_character_by_name(&self, name: &str) -> Option<CharacterSchema> {
        if let Ok(c) = self.my_characters_api.all() {
            c.data.iter().find(|c| c.name == name).cloned()
        } else {
            None
        }
    }
}
