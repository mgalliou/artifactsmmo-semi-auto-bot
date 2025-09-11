use crate::{
    bank::BankController, bot_config::BotConfig, character::CharacterController,
    gear_finder::GearFinder, leveling_helper::LevelingHelper, orderboard::OrderBoard,
};
use artifactsmmo_sdk::{
    Client, ItemContainer, Items, SpaceLimited,
    account::Account as AccountClient,
    char::{HasCharacterData, Skill},
    items::ItemSource,
    models::{ItemSchema, SimpleItemSchema},
};
use itertools::Itertools;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct AccountController {
    config: Arc<BotConfig>,
    client: Arc<AccountClient>,
    items: Arc<Items>,
    pub bank: Arc<BankController>,
    pub characters: RwLock<Vec<Arc<CharacterController>>>,
}

impl AccountController {
    pub fn new(
        config: Arc<BotConfig>,
        client: Arc<AccountClient>,
        items: Arc<Items>,
        bank: Arc<BankController>,
    ) -> Self {
        Self {
            config,
            client,
            items,
            bank,
            characters: RwLock::new(vec![]),
        }
    }

    pub fn init_characters(
        &self,
        client: Arc<Client>,
        account: Arc<AccountController>,
        order_board: Arc<OrderBoard>,
        gear_finder: Arc<GearFinder>,
        leveling_helper: Arc<LevelingHelper>,
    ) {
        let Ok(mut chars) = self.characters.write() else {
            return;
        };
        *chars = self
            .client
            .characters
            .iter()
            .map(|char_client| {
                Arc::new(CharacterController::new(
                    char_client.clone(),
                    self.config.clone(),
                    &client,
                    account.clone(),
                    order_board.clone(),
                    gear_finder.clone(),
                    leveling_helper.clone(),
                ))
            })
            .collect_vec()
    }

    pub fn characters(&self) -> Vec<Arc<CharacterController>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .cloned()
            .collect_vec()
    }

    pub fn get_character(&self, index: usize) -> Option<Arc<CharacterController>> {
        self.characters.read().unwrap().get(index).cloned()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<CharacterController>> {
        self.characters
            .read()
            .unwrap()
            .iter()
            .find(|c| c.name() == name)
            .cloned()
    }

    pub fn available_in_inventories(&self, item: &str) -> u32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .cloned()
            .map(|c| c.inventory.has_available(item))
            .sum()
    }

    pub fn total_of(&self, item: &str) -> u32 {
        self.bank.total_of(item)
            + self
                .characters()
                .iter()
                .map(|c| c.has_equiped(item) + c.inventory.total_of(item))
                .sum::<u32>()
    }

    pub fn meets_conditions(&self, item: &ItemSchema) -> u32 {
        self.characters()
            .iter()
            .filter(|c| c.meets_conditions_for(item))
            .count() as u32
    }

    pub fn can_craft(&self, item: &str) -> bool {
        self.characters
            .read()
            .unwrap()
            .iter()
            .any(|c| c.can_craft(item).is_ok())
    }

    pub fn max_skill_level(&self, skill: Skill) -> u32 {
        self.characters
            .read()
            .unwrap()
            .iter()
            .map(|c| c.skill_level(skill))
            .max()
            .unwrap_or(1)
    }

    pub fn fisher_max_items(&self) -> u32 {
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

    pub fn time_to_get(&self, item: &str) -> Option<u32> {
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
                        .collect::<Vec<(SimpleItemSchema, Option<u32>)>>();
                    if mats_wit_ttg.iter().all(|(_, ttg)| ttg.is_some()) {
                        Some(
                            mats_wit_ttg
                                .iter()
                                .filter_map(|(m, ttg)| {
                                    ttg.as_ref()
                                        .map(|ttg| (ttg * m.quantity) + (5 * m.quantity))
                                })
                                .sum::<u32>(),
                        )
                    } else {
                        None
                    }
                }
                ItemSource::TaskReward => Some(20000),
                ItemSource::Task => Some(20000),
                ItemSource::Npc(_) => Some(60),
            })
            .min()
    }
}
