use artifactsmmo_semi_auto_bot::{bot::Bot, bot_config::BOT_CONFIG, cli, orderboard::{Purpose, ORDER_BOARD}};
use anyhow::Result;
use log::LevelFilter;

fn main() -> Result<()> {
    simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info)?;
    artifactsmmo_sdk::init(&BOT_CONFIG.base_url, &BOT_CONFIG.token);
    //game.orderboard
    //    .add(None, "lizard_skin", 1000, Purpose::Cli)?;
    //game.orderboard
    //    .add(None, "demon_horn", 1000, Purpose::Cli)?;
    ORDER_BOARD.add(None, "malefic_cloth", 200, Purpose::Cli)?;
    ORDER_BOARD.add(None, "rosenblood_elixir", 200, Purpose::Cli)?;
    ORDER_BOARD.add(None, "strange_ore", 6000, Purpose::Cli)?;
    ORDER_BOARD.add(None, "magic_wood", 6000, Purpose::Cli)?;
    //game.orderboard.add(None, "carrot", 1000, Purpose::Cli);
    //game.orderboard.add(None, "frozen_pickaxe", 5, Purpose::Cli)?;
    Bot::run_characters();
    cli::run()
}
