use log::error;
use std::{
    sync::LazyLock,
    thread::{sleep, Builder},
    time::Duration,
};

use crate::account::ACCOUNT;

pub static GAME: LazyLock<Game> = LazyLock::new(Game::new);

pub struct Game {}

impl Game {
    fn new() -> Self {
        Game {}
    }

    pub fn run_characters(&self) {
        for c in ACCOUNT.characters() {
            sleep(Duration::from_millis(250));
            if let Err(e) = Builder::new().spawn(move || {
                c.run_loop();
            }) {
                error!("failed to spawn character thread: {}", e);
            }
        }
    }
}
