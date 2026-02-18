use crate::{
    bank::BankController, bot_config::BotConfig, character::CharacterController,
    gear_finder::GearFinder, leveling_helper::LevelingHelper, orderboard::OrderBoard,
};
use itertools::Itertools;
use sdk::{
    AccountClient, Client, Code, CollectionClient, ItemContainer, ItemsClient, NpcsClient, Skill,
    SpaceLimited, character::{HasCharacterData, MeetsConditionsFor}, entities::Item, items::ItemSource,
};
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct AccountController {
    config: Arc<BotConfig>,
    client: Arc<AccountClient>,
    items: Arc<ItemsClient>,
    npcs: Arc<NpcsClient>,
    pub bank: Arc<BankController>,
    pub characters: RwLock<Vec<Arc<CharacterController>>>,
}

impl AccountController {
    pub fn new(
        config: Arc<BotConfig>,
        client: Arc<AccountClient>,
        items: Arc<ItemsClient>,
        npcs: Arc<NpcsClient>,
        bank: Arc<BankController>,
    ) -> Self {
        Self {
            config,
            client,
            items,
            bank,
            npcs,
            characters: RwLock::new(vec![]),
        }
    }

    pub fn client(&self) -> Arc<AccountClient> {
        self.client.clone()
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

    pub fn meets_conditions(&self, item: &Item) -> u32 {
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

    pub fn time_to_get(&self, code: &str) -> Option<u32> {
        let item = self.items.get(code)?;
        let (source, mut time) = self
            .characters()
            .iter()
            .filter_map(|c| c.time_to_get(item.code()))
            .min_by_key(|(_, t)| *t)?;

        match source {
            ItemSource::Npc(npc) => {
                if let Some(npc_item) = self.npcs.items.get(item.code())
                    && npc_item.npc_code() == npc.code()
                {
                    time += self.time_to_get(npc_item.currency())? * npc_item.buy_price()?
                }
            }
            ItemSource::Craft => {
                for mat in item.mats().iter() {
                    time += self.time_to_get(&mat.code)? * mat.quantity;
                }
            }
            _ => (),
        }
        Some(time)
    }
}
