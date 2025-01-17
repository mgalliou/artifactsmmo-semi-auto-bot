use crate::{
    api::{CharactersApi, MyCharacterApi},
    char::{Character, HasCharacterData, Skill},
    game_config::GAME_CONFIG,
    items::{ItemSource, ITEMS},
};
use artifactsmmo_openapi::{
    apis::configuration::Configuration,
    models::{CharacterSchema, SimpleItemSchema},
};
use itertools::Itertools;
use lazy_static::lazy_static;
use std::sync::{Arc, RwLock};

lazy_static! {
    pub static ref ACCOUNT: Arc<Account> = Arc::new(Account::new());
}

#[derive(Default)]
pub struct Account {
    pub configuration: Configuration,
    pub character_api: CharactersApi,
    pub my_characters_api: MyCharacterApi,
    pub characters: RwLock<Vec<Arc<Character>>>,
}

impl Account {
    fn new() -> Account {
        let mut configuration = Configuration::new();
        configuration.base_path = GAME_CONFIG.base_url.to_owned();
        configuration.bearer_access_token = Some(GAME_CONFIG.token.to_owned());
        let my_characters_api = MyCharacterApi::new(&GAME_CONFIG.base_url, &GAME_CONFIG.token);
        let account = Account {
            configuration,
            character_api: CharactersApi::new(&GAME_CONFIG.base_url, &GAME_CONFIG.token),
            my_characters_api,
            characters: RwLock::new(vec![]),
        };
        account.init_characters();
        account
    }

    fn init_characters(&self) {
        let Ok(mut chars) = self.characters.write() else {
            return;
        };
        *chars = self
            .get_characters_data()
            .iter()
            .enumerate()
            .map(|(id, data)| Arc::new(Character::new(id, data)))
            .collect_vec()
    }

    pub fn characters(&self) -> Vec<Arc<Character>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    pub fn get_character(&self, index: usize) -> Option<Arc<Character>> {
        self.characters.read().unwrap().get(index).cloned()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<Character>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .find(|c| c.name() == name)
            .cloned()
    }

    pub fn available_in_inventories(&self, item: &str) -> i32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .cloned()
            .map(|c| c.inventory.has_available(item))
            .sum()
    }

    pub fn can_craft(&self, item: &str) -> bool {
        self.characters
            .read()
            .unwrap()
            .iter()
            .any(|c| c.can_craft(item).is_ok())
    }

    pub fn max_skill_level(&self, skill: Skill) -> i32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .map(|c| c.skill_level(skill))
            .max()
            .unwrap_or(1)
    }

    pub fn fisher_max_items(&self) -> i32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .filter_map(|c| {
                if c.skill_enabled(Skill::Fishing) {
                    Some(c.inventory.max_items())
                } else {
                    None
                }
            })
            .min()
            .unwrap_or(0)
    }

    pub fn time_to_get(&self, item: &str) -> Option<i32> {
        ITEMS
            .best_source_of(item)
            .iter()
            .filter_map(|s| match s {
                ItemSource::Resource(r) => self
                    .characters
                    .read()
                    .unwrap()
                    .iter()
                    .filter_map(|c| c.time_to_gather(r))
                    .min(),
                ItemSource::Monster(m) => self
                    .characters
                    .read()
                    .unwrap()
                    .iter()
                    .filter_map(|c| c.time_to_kill(m))
                    .map(|time| time * ITEMS.drop_rate(item))
                    .min(),
                ItemSource::Craft => {
                    let mats_wit_ttg = ITEMS
                        .mats_of(item)
                        .into_iter()
                        .map(|m| (m.clone(), self.time_to_get(&m.code)))
                        .collect::<Vec<(SimpleItemSchema, Option<i32>)>>();
                    if mats_wit_ttg.iter().all(|(_, ttg)| ttg.is_some()) {
                        Some(
                            mats_wit_ttg
                                .iter()
                                .filter_map(|(m, ttg)| {
                                    ttg.as_ref()
                                        .map(|ttg| (ttg * m.quantity) + (5 * m.quantity))
                                })
                                .sum::<i32>(),
                        )
                    } else {
                        None
                    }
                }
                ItemSource::TaskReward => Some(20000),
                ItemSource::Task => Some(20000),
                ItemSource::Gift => Some(10000),
            })
            .min()
    }

    fn get_characters_data(&self) -> Vec<Arc<RwLock<CharacterSchema>>> {
        let my_characters_api = MyCharacterApi::new(
            &self.configuration.base_path,
            &self.configuration.bearer_access_token.clone().unwrap(),
        );
        my_characters_api
            .characters()
            .unwrap()
            .data
            .into_iter()
            .map(|s| Arc::new(RwLock::new(s)))
            .collect_vec()
    }
}
