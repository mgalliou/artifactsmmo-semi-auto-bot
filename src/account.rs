use artifactsmmo_sdk::{
    char::{HasCharacterData, Skill},
    items::ItemSource,
    models::SimpleItemSchema,
    BASE_ACCOUNT, ITEMS,
};
use itertools::Itertools;
use std::sync::{Arc, LazyLock, RwLock};

use crate::character::Character;

pub static ACCOUNT: LazyLock<Account> = LazyLock::new(Account::new);

#[derive(Default)]
pub struct Account {
    pub characters: RwLock<Vec<Arc<Character>>>,
}

impl Account {
    fn new() -> Account {
        let account = Account {
            characters: RwLock::new(vec![]),
        };
        account.init_characters();
        account
    }

    fn init_characters(&self) {
        let Ok(mut chars) = self.characters.write() else {
            return;
        };
        *chars = BASE_ACCOUNT
            .characters()
            .iter()
            .map(|(id, data)| Arc::new(Character::new(*id, data.clone())))
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
                //ItemSource::Gift => Some(10000),
            })
            .min()
    }
}
