use crate::{
    account::AccountController, bank::BankController, bot_config::BotConfig,
    gear_finder::GearFinder, leveling_helper::LevelingHelper, orderboard::OrderBoard,
};
use log::error;
use sdk::{Client, entities::Character};
use std::{
    thread::{Builder, sleep},
    time::Duration,
};

pub struct Bot {
    pub config: BotConfig,
    pub client: Client,
    pub order_board: OrderBoard,
    pub gear_finder: GearFinder,
    pub leveling_helper: LevelingHelper,
    pub account: AccountController,
    pub bank: BankController,
}

impl Bot {
    pub fn new(client: &Client) -> Self {
        let config = BotConfig::from_file();
        let bank = BankController::new(
            client.account.bank(),
            client.items.clone(),
        );
        let account = AccountController::new(
            config.clone(),
            client.account.clone(),
            client.items.clone(),
            client.npcs.clone(),
            bank.clone(),
        );
        Self {
            config,
            client: client.clone(),
            order_board: OrderBoard::new(client.items.clone(), account.clone()),
            gear_finder: GearFinder::new(client.items.clone(), account.clone()),
            leveling_helper: LevelingHelper::new(
                client.items.clone(),
                client.monsters.clone(),
                client.resources.clone(),
                client.maps.clone(),
                account.clone(),
                bank.clone(),
            ),
            account,
            bank,
        }
    }

    pub fn run(&self) {
        self.account.init_characters(
            &self.client,
            &self.account,
            &self.order_board,
            &self.gear_finder,
            &self.leveling_helper,
        );
        for char in self.account.characters() {
            sleep(Duration::from_millis(250));
            if let Err(e) = Builder::new().name(char.name().to_string()).spawn(move || {
                char.run_loop();
            }) {
                error!("failed to spawn character thread: {e}");
            }
        }
    }
}
