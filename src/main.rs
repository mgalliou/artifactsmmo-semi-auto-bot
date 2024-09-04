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
use std::{
    sync::{Arc, RwLock},
    thread,
};

fn run() -> Option<()> {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    let account = Account::new(base_url, token);
    let maps = Arc::new(Maps::new(&account));
    let resources = Arc::new(Resources::new(&account));
    let monsters = Arc::new(Monsters::new(&account));
    let items = Arc::new(Items::new(&account, resources.clone(), monsters.clone()));
    let bank = Arc::new(RwLock::new(Bank::new(&account, items.clone())));
    let mut char1 = Character::new(
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
            resource: Some("copper_ore".to_string()),
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
    let mut char2 = Character::new(
        &account,
        "Eraly",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            resource: Some("coal".to_string()),
            ..Default::default()
        },
    );
    let mut char3 = Character::new(
        &account,
        "Nalgisk",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            process_gathered: true,
            resource: Some("iron_ore".to_string()),
            ..Default::default()
        },
    );
    let mut char4 = Character::new(
        &account,
        "Tieleja",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Woodcutter,
            resource: Some("birch_wood".to_string()),
            ..Default::default()
        },
    );
    let mut char5 = Character::new(
        &account,
        "Kvarask",
        maps.clone(),
        resources.clone(),
        monsters.clone(),
        items.clone(),
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            process_gathered: true,
            resource: Some("iron_ore".to_string()),
            ..Default::default()
        },
    );

    let t1 = thread::Builder::new()
        .name(char1.name.to_string())
        .spawn(move || {
            char1.run();
        })
        .unwrap();
    let t2 = thread::Builder::new()
        .name(char2.name.to_string())
        .spawn(move || {
            char2.run();
        })
        .unwrap();
    let t3 = thread::Builder::new()
        .name(char3.name.to_string())
        .spawn(move || char3.run())
        .unwrap();
    let t4 = thread::Builder::new()
        .name(char4.name.to_string())
        .spawn(move || char4.run())
        .unwrap();
    let t5 = thread::Builder::new()
        .name(char5.name.to_string())
        .spawn(move || char5.run())
        .unwrap();
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
