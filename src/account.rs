use crate::{
    bank::BankController, bot_config::BotConfig, character::CharacterController,
    gear_finder::GearFinder, leveling_helper::LevelingHelper, orderboard::OrderBoard,
};
use artifactsmmo_sdk::{
    AccountClient, Client, CollectionClient, ItemContainer, ItemsClient, SpaceLimited,
    character::HasCharacterData, items::ItemSchemaExt, models::ItemSchema, skill::Skill,
};
use itertools::Itertools;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct AccountController {
    config: Arc<BotConfig>,
    client: Arc<AccountClient>,
    items: Arc<ItemsClient>,
    pub bank: Arc<BankController>,
    pub characters: RwLock<Vec<Arc<CharacterController>>>,
}

impl AccountController {
    pub fn new(
        config: Arc<BotConfig>,
        client: Arc<AccountClient>,
        items: Arc<ItemsClient>,
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
            .characters()
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
        self.characters().get(index).cloned()
    }

    pub fn get_character_by_name(&self, name: &str) -> Option<Arc<CharacterController>> {
        self.characters().iter().find(|c| c.name() == name).cloned()
    }

    pub fn available_in_inventories(&self, item: &str) -> u32 {
        self.characters()
            .iter()
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
        self.characters().iter().any(|c| c.can_craft(item).is_ok())
    }

    pub fn max_skill_level(&self, skill: Skill) -> u32 {
        self.characters()
            .iter()
            .map(|c| c.skill_level(skill))
            .max()
            .unwrap_or(0)
    }

    pub fn fisher_max_items(&self) -> u32 {
        self.characters()
            .iter()
            .filter_map(|c| {
                c.skill_enabled(Skill::Fishing)
                    .then_some(c.inventory.max_items())
            })
            .min()
            .unwrap_or(0)
    }

    pub fn time_to_get(&self, item: &str) -> Option<u32> {
        let item = self.items.get(item)?;
        let mut time = self
            .characters()
            .iter()
            .filter_map(|c| c.time_to_get(&item.code))
            .min()?;

        for mat in item.mats().iter() {
            time += self.time_to_get(&mat.code)? * mat.quantity;
        }
        Some(time)
    }
}
