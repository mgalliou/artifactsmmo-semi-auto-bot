use anyhow::Result;
use artifactsmmo_sdk::Client;
use artifactsmmo_semi_auto_bot::{bot::Bot, cli};
use log::LevelFilter;
use std::{env, sync::Arc};

fn main() -> Result<()> {
    simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info)?;
    let client = Client::new(
        "https://api.artifactsmmo.com".to_string(),
        "podJio".to_string(),
        env::var("ARTIFACTSMMO_TOKEN").unwrap_or("".to_string()),
    )?;
    let bot = Arc::new(Bot::new(Arc::new(client)));
    //game.orderboard
    //    .add(None, "lizard_skin", 1000, Purpose::Cli)?;
    //game.orderboard
    //    .add(None, "demon_horn", 1000, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "malefic_cloth", 200, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "rosenblood_elixir", 200, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "strange_ore", 6000, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "magic_wood", 6000, Purpose::Cli)?;
    bot.run_characters();
    cli::run(bot.clone())
}
