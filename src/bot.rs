use crate::{
    account::AccountController,
    bank::BankController,
    bot_config::BotConfig,
    gear_finder::GearFinder,
    leveling_helper::LevelingHelper,
    orderboard::OrderBoard,
};
use artifactsmmo_sdk::Client;
use log::error;
use std::{
    sync::Arc,
    thread::{Builder, sleep},
    time::Duration,
};

pub struct Bot {
    pub config: Arc<BotConfig>,
    pub client: Arc<Client>,
    pub order_board: Arc<OrderBoard>,
    pub gear_finder: Arc<GearFinder>,
    pub leveling_helper: Arc<LevelingHelper>,
    pub account: Arc<AccountController>,
    pub bank: Arc<BankController>,
}

impl Bot {
    pub fn new(client: Arc<Client>) -> Self {
        let config = Arc::new(BotConfig::from_file());
        let bank = Arc::new(BankController::new(
            client.account.bank.clone(),
            client.items.clone(),
        ));
        let account = Arc::new(AccountController::new(
            config.clone(),
            client.account.clone(),
            client.items.clone(),
            bank.clone(),
        ));
        Self {
            config,
            client: client.clone(),
            order_board: Arc::new(OrderBoard::new(client.items.clone(), account.clone())),
            gear_finder: Arc::new(GearFinder::new(client.items.clone(), account.clone())),
            leveling_helper: Arc::new(LevelingHelper::new(
                client.items.clone(),
                client.monsters.clone(),
                client.resources.clone(),
                client.maps.clone(),
                account.clone(),
                bank.clone(),
            )),
            account,
            bank,
        }
    }

    pub fn run_characters(&self) {
        self.account.init_characters(
            self.client.clone(),
            self.account.clone(),
            self.order_board.clone(),
            self.gear_finder.clone(),
            self.leveling_helper.clone(),
        );
        for c in self.account.characters() {
            sleep(Duration::from_millis(250));
            if let Err(e) = Builder::new().spawn(move || {
                c.run_loop();
            }) {
                error!("failed to spawn character thread: {}", e);
            }
        }
    }
}
