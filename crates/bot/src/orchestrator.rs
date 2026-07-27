use crate::{
    CharacterCommand, account::AccountController, bank::BankController,
    character::CharacterController,
};
use itertools::Itertools;
use log::{debug, info, warn};
use sdk::{
    CanProvideXp, Code, ItemsClient, Level, SdkEvent, Skill,
    entities::{Character, Item},
    models::SimpleItemSchema,
};
use std::cmp::{Reverse, min};

pub struct Orchestrator {
    event_rx: bus::BusReader<SdkEvent>,
    account: AccountController,
    bank: BankController,
    items: ItemsClient,
}

impl Orchestrator {
    #[must_use]
    pub const fn new(
        event_rx: bus::BusReader<SdkEvent>,
        account: AccountController,
        bank: BankController,
        items: ItemsClient,
    ) -> Self {
        Self {
            event_rx,
            account,
            bank,
            items,
        }
    }

    pub fn run(&mut self) {
        info!("orchestrator started");
        while let Ok(event) = self.event_rx.recv() {
            match event {
                SdkEvent::ItemDeposited { character, items } => {
                    // self.handle_item_deposited(&character, &items);
                }
                SdkEvent::ItemWithdrawn { .. }
                | SdkEvent::GoldDeposited { .. }
                | SdkEvent::GoldWithdrawn { .. } => {}
            }
        }
        info!("orchestrator stopped");
    }

    fn handle_item_deposited(&self, character: &str, deposited_items: &[SimpleItemSchema]) {
        debug!("orchestrator: processing deposit from {character}");
        for deposited in deposited_items {
            let cookable = self
                .items
                .crafted_with(deposited.code())
                .into_iter()
                .filter(|i| i.skill_to_craft() == Some(Skill::Cooking))
                .collect_vec();

            for item in cookable {
                let Some(char) = self.best_cook_for(&item) else {
                    debug!(
                        "orchestrator: no cook available for '{}', skipping",
                        item.code()
                    );
                    continue;
                };

                let Some(quantity) = self.max_craft_batch(&item, &char) else {
                    continue;
                };

                info!(
                    "orchestrator: sending cook command to {}: craft '{}'x{quantity}",
                    char.name(),
                    item.code(),
                );
                if let Err(e) = char.send_cmd(CharacterCommand::Craft {
                    item: item.code().to_owned(),
                    quantity,
                }) {
                    warn!(
                        "orchestrator: failed to send craft command to {}: {e}",
                        char.name()
                    );
                }
            }
        }
    }

    fn best_cook_for(&self, item: &Item) -> Option<CharacterController> {
        let mut xp_cooks = self
            .account
            .characters()
            .into_iter()
            .filter(|c| c.skill_enabled(Skill::Cooking))
            .filter(|c| item.provides_xp_at(c.skill_level(Skill::Cooking)))
            .collect_vec();

        if !xp_cooks.is_empty() {
            xp_cooks.sort_by_key(|c| Reverse(c.skill_level(Skill::Cooking)));
            return xp_cooks.first().cloned();
        }

        let mut any_cooks = self
            .account
            .characters()
            .into_iter()
            .filter(|c| c.skill_enabled(Skill::Cooking))
            .filter(|c| c.skill_level(Skill::Cooking) >= item.level())
            .collect_vec();

        any_cooks.sort_by_key(|c| Reverse(c.skill_level(Skill::Cooking)));
        any_cooks.first().cloned()
    }

    fn max_craft_batch(&self, item: &Item, char: &CharacterController) -> Option<u32> {
        let mats = item.mats();
        if mats.is_empty() {
            return None;
        }
        let max_from_bank = mats
            .iter()
            .map(|m| self.bank.has_available((m.code(), char.name())) / m.quantity)
            .min()?;
        if max_from_bank == 0 {
            return None;
        }
        let max_from_inventory = char.max_craftable_items(item.code());
        let quantity = min(max_from_bank, max_from_inventory);
        if quantity == 0 {
            return None;
        }

        Some(quantity)
    }
}
