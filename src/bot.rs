use crate::account::ACCOUNT;
use log::error;
use std::{
    thread::{sleep, Builder},
    time::Duration,
};

pub struct Bot {}

impl Bot {
    pub fn run_characters() {
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
