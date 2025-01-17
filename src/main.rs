use anyhow::Result;
use artifactsmmo_sdk::{Game, orderboard::Purpose};
use log::LevelFilter;

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
    game.run_characters();
    artifactsmmo_playground::cli::run_cli(&game)
}
