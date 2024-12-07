use super::{
    api::{characters::CharactersApi, my_character::MyCharacterApi},
    bank::Bank,
    character::Character,
    game::Game,
    game_config::GameConfig,
    items::{ItemSource, Items},
    skill::Skill,
};
use crate::artifactsmmo_sdk::char_config::CharConfig;
use artifactsmmo_openapi::{
    apis::configuration::Configuration,
    models::{CharacterSchema, SimpleItemSchema},
};
use itertools::Itertools;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Default)]
pub struct Account {
    pub configuration: Configuration,
    pub config: Arc<GameConfig>,
    pub character_api: CharactersApi,
    pub my_characters_api: MyCharacterApi,
    pub items: Arc<Items>,
    pub bank: Arc<Bank>,
    pub characters: RwLock<Vec<Arc<Character>>>,
}

impl Account {
    pub fn new(config: &Arc<GameConfig>, items: &Arc<Items>) -> Arc<Account> {
        let mut configuration = Configuration::new();
        configuration.base_path = config.base_url.to_owned();
        configuration.bearer_access_token = Some(config.base_url.to_owned());
        let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
        Arc::new(Account {
            configuration,
            config: config.clone(),
            character_api: CharactersApi::new(&config.base_url, &config.token),
            my_characters_api,
            items: items.clone(),
            bank: Arc::new(Bank::from_api(config, items)),
            characters: RwLock::new(vec![]),
        })
    }

    pub fn init_characters(&self, game: &Game) {
        let chars_conf = init_char_conf(&self.config.characters);
        let chars_schema = init_chars_schema(&self.config);
        if let Ok(mut chars) = self.characters.write() {
            *chars = chars_conf
                .into_iter()
                .zip(chars_schema.iter())
                .map(|(conf, schema)| {
                    Arc::new(Character::new(
                        &self.config,
                        &game.account,
                        game,
                        &self.bank,
                        &conf,
                        schema,
                    ))
                })
                .collect_vec()
        }
    }

    pub fn get_character(&self, index: usize) -> Option<Arc<Character>> {
        self.characters.read().unwrap().get(index).cloned()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<Character>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .find(|c| c.name == name)
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

    pub fn fisher_inventory_space(&self) -> i32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .filter_map(|c| {
                if c.skill_enabled(Skill::Fishing) {
                    Some(c.inventory.free_space())
                } else {
                    None
                }
            })
            .min()
            .unwrap_or(0)
    }

    pub fn time_to_get(&self, item: &str) -> Option<i32> {
        self.items
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
                    .map(|time| time * self.items.drop_rate(item))
                    .min(),
                ItemSource::Craft => {
                    let mats_wit_ttg = self
                        .items
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
}

fn init_char_conf(confs: &[CharConfig]) -> Vec<Arc<RwLock<CharConfig>>> {
    confs
        .iter()
        .map(|c| Arc::new(RwLock::new(c.clone())))
        .collect_vec()
}

fn init_chars_schema(config: &GameConfig) -> Vec<Arc<RwLock<CharacterSchema>>> {
    let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
    my_characters_api
        .characters()
        .unwrap()
        .data
        .into_iter()
        .map(|s| Arc::new(RwLock::new(s)))
        .collect_vec()
}
