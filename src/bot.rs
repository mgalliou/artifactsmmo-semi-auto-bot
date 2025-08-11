use crate::{
    account::AccountController,
    bank::Bank,
    gear_finder::GearFinder,
    leveling_helper::LevelingHelper,
    orderboard::{Order, OrderBoard},
};
use artifactsmmo_sdk::Client;
use log::error;
use std::{
    sync::Arc,
    thread::{sleep, Builder},
    time::Duration,
};

pub struct Bot {
    pub client: Arc<Client>,
    pub order_board: Arc<OrderBoard>,
    pub gear_finder: Arc<GearFinder>,
    pub leveling_helper: Arc<LevelingHelper>,
    pub account: Arc<AccountController>,
    pub bank: Arc<Bank>,
}

impl Bot {
    pub fn new(client: Arc<Client>) -> Self {
        let account = Arc::new(AccountController::new(client.account.clone(), client.items.clone()));
        let bank = Arc::new(Bank::new(client.account.bank.clone(), client.items.clone()));
        Self {
            client: client.clone(),
            order_board: Arc::new(OrderBoard::new(client.items.clone(), account.clone())),
            gear_finder: Arc::new(GearFinder::new(
                client.items.clone(),
                client.resources.clone(),
                account.clone(),
            )),
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

    pub fn run_characters(self) {
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
