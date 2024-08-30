use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account,
    bank::Bank,
    char_config::CharConfig,
    character::{Character, Role},
};
use std::{
    sync::{Arc, RwLock},
    thread,
};

fn run() -> Option<()> {
    let base_url = "https://api.artifactsmmo.com";
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ1c2VybmFtZSI6InBvZEppbyIsInBhc3N3b3JkX2NoYW5nZWQiOiIifQ.Qy1Hm2-QYm84O_9aLP076TczjYDCpSuZ75dKkh9toUY";
    let account = Account::new(base_url, token);
    let bank = Arc::new(RwLock::new(Bank::new(&account)));
    let mut char1 = Character::new(
        &account,
        &account.get_character_by_name("Jio")?.name,
        bank.clone(),
        CharConfig {
            role: Role::Fighter,
            fight: true,
            fight_target: Some("chicken".to_string()),
            weaponcraft: true,
            level_weaponcraft: true,
            gearcraft: true,
            level_gearcraft: true,
            jewelcraft: true,
            level_jewelcraft: true,
            cook: true,
            level_cook: true,
            ..Default::default()
        },
    );
    let mut char2 = Character::new(
        &account,
        &account.get_character_by_name("Eraly")?.name,
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            mine: true,
            mine_resource: Some("copper_ore".to_string()),
            ..Default::default()
        },
    );
    let mut char3 = Character::new(
        &account,
        &account.get_character_by_name("Nalgisk")?.name,
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            mine: true,
            mine_craft: true,
            mine_resource: Some("copper_ore".to_string()),
            ..Default::default()
        },
    );
    let mut char4 = Character::new(
        &account,
        &account.get_character_by_name("Tieleja")?.name,
        bank.clone(),
        CharConfig {
            role: Role::Woodcutter,
            lumber: true,
            lumber_resource: Some("ash_wood".to_string()),
            ..Default::default()
        },
    );
    let mut char5 = Character::new(
        &account,
        &account.get_character_by_name("Kvarask")?.name,
        bank.clone(),
        CharConfig {
            role: Role::Miner,
            mine: true,
            mine_resource: Some("copper_ore".to_string()),
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
