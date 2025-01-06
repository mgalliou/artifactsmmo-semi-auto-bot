use anyhow::Result;
use artifactsmmo_playground::{
    artifactsmmo_sdk::{character::Character, game::Game, orderboard::Purpose},
    cli::run_cli,
};
use log::LevelFilter;
use std::{thread::sleep, time::Duration};

fn main() -> Result<()> {
    simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info)?;
    let game = Game::new();
    game.init();
    //game.orderboard
    //    .add(None, "lizard_skin", 1000, Purpose::Cli)?;
    //game.orderboard
    //    .add(None, "demon_horn", 1000, Purpose::Cli)?;
    game.orderboard
        .add(None, "malefic_cloth", 200, Purpose::Cli)?;
    game.orderboard
        .add(None, "rosenblood_elixir", 200, Purpose::Cli)?;
    game.orderboard
        .add(None, "strange_ore", 6000, Purpose::Cli)?;
    game.orderboard
        .add(None, "magic_wood", 6000, Purpose::Cli)?;
    //game.orderboard.add(None, "carrot", 1000, Purpose::Cli);
    //game.orderboard.add(None, "frozen_pickaxe", 5, Purpose::Cli)?;
    for c in game.account.characters.read().unwrap().iter() {
        sleep(Duration::from_millis(250));
        Character::run(c.clone())?;
    }
    run_cli(&game)
}
