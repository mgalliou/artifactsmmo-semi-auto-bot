use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account,
    bank::Bank,
    char_config::CharConfig,
    character::{Character, Role},
    items::Items,
    maps::Maps,
    monsters::Monsters,
    resources::Resources,
};
use std::sync::{Arc, RwLock};

fn run() -> Option<()> {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    env_logger::init();
    let account = Account::new(base_url, token);
    let maps = Arc::new(Maps::new(&account));
    let resources = Arc::new(Resources::new(&account));
    let monsters = Arc::new(Monsters::new(&account));
    let items = Arc::new(Items::new(&account, resources.clone(), monsters.clone()));
    let bank = Arc::new(RwLock::new(Bank::new(&account, items.clone())));
    let char1 = Character::new(
        &account,
        "Jio",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Fighter,
            fight_target: Some("mushmush".to_string()),
            do_tasks: false,
            target_item: Some("copper_ore".to_string()),
            craft_from_bank: false,
            weaponcraft: true,
            level_weaponcraft: false,
            gearcraft: true,
            level_gearcraft: false,
            jewelcraft: true,
            level_jewelcraft: false,
            ..Default::default()
        },
    );
    let char2 = Character::new(
        &account,
        "Eraly",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            target_item: Some("gold_ore".to_string()),
            ..Default::default()
        },
    );
    let char3 = Character::new(
        &account,
        "Nalgisk",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            target_item: Some("gold_ore".to_string()),
            ..Default::default()
        },
    );
    let char4 = Character::new(
        &account,
        "Tieleja",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Woodcutter,
            target_item: Some("dead_wood".to_string()),
            ..Default::default()
        },
    );
    let char5 = Character::new(
        &account,
        "Kvarask",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            target_item: Some("coal".to_string()),
            ..Default::default()
        },
    );

    let t1 = Character::run(char1).unwrap();
    let t2 = Character::run(char2).unwrap();
    let t3 = Character::run(char3).unwrap();
    let t4 = Character::run(char4).unwrap();
    let t5 = Character::run(char5).unwrap();
    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();
    t4.join().unwrap();
    t5.join().unwrap();
    Some(())
}

fn main() {
    let _ = run().is_some();
}
