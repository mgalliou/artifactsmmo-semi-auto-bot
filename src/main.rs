use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account, bank::Bank, character::Character, config::Config, items::Items, maps::Maps,
    monsters::Monsters, resources::Resources,
};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use std::{
    sync::{Arc, RwLock},
    thread::JoinHandle,
};

fn run() -> Option<()> {
    env_logger::init();
    let config: Config = Figment::new()
        .merge(Toml::file_exact("ArtifactsMMO.toml"))
        .extract()
        .unwrap();
    let account = Account::new(&config.base_url, &config.token);
    let maps = Arc::new(Maps::new(&account));
    let resources = Arc::new(Resources::new(&account));
    let monsters = Arc::new(Monsters::new(&account));
    let items = Arc::new(Items::new(&account, resources.clone(), monsters.clone()));
    let bank = Arc::new(RwLock::new(Bank::new(&account, items.clone())));
    let mut handles: Vec<JoinHandle<()>> = vec![];
    for char in config.characters {
        handles.push(Character::run(Character::new(
            &char,
            &account,
            maps.clone(),
            resources.clone(),
            monsters.clone(),
            items.clone(),
            bank.clone(),
        )).unwrap());
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Some(())
}

fn main() {
    let _ = run().is_some();
}
