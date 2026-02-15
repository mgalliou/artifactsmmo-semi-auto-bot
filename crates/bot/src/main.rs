use anyhow::Result;
use bot::{bot::Bot, cli, orderboard::Purpose};
use log::LevelFilter;
use sdk::Client;
use std::{env, sync::Arc};

fn main() -> Result<()> {
    simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info)?;
    let client = Client::new(
        "https://api.artifactsmmo.com".to_string(),
        "podJio".to_string(),
        env::var("ARTIFACTSMMO_TOKEN").unwrap_or("".to_string()),
    )?;
    let bot = Arc::new(Bot::new(Arc::new(client)));
    bot.order_board
        .add("lizard_skin", 1000, None, Purpose::Cli)?;
    bot.order_board
        .add("demon_horn", 1000, None, Purpose::Cli)?;
    bot.order_board
        .add("corrupted_gem", 1000, None, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "malefic_cloth", 200, Purpose::Cli)?;
    // bot.order_board
    //     .add(None, "rosenblood_elixir", 200, Purpose::Cli)?;
    // bot.order_board
    //     .add("magic_wood", 6000, None, Purpose::Cli)?;
    // bot.order_board
    //     .add("strange_ore", 6000, None, Purpose::Cli)?;
    bot.run_characters();
    cli::run(bot.clone())
}
