use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account, api::my_character::MyCharacterApi, bank::Bank, character::Character, config::Config, items::Items, maps::Maps, monsters::Monsters, resources::Resources
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
    let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
    let chars_schema = my_characters_api.characters().unwrap();
    let mut handles: Vec<JoinHandle<()>> = vec![];
    for char_conf in config.characters {
        handles.push(Character::run(Character::new(
            &account,
            maps.clone(),
            resources.clone(),
            monsters.clone(),
            items.clone(),
            bank.clone(),
            &char_conf,
            chars_schema.data.iter().find(|c| c.name == char_conf.name).unwrap()
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
