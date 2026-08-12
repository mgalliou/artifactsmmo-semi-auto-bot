use bot::Bot;
use color_eyre::eyre::eyre;
use sdk::Client;
use std::env;
use tui::App;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let logs = tui::init_logger().map_err(|e| eyre!(e.to_string()))?;
    let client = Client::new(
        "https://api.artifactsmmo.com".into(),
        env::var("ARTIFACTSMMO_TOKEN").unwrap_or_default(),
        ".cache",
    );
    client.init();
    let bot = Bot::new(client.clone());
    let order_board = bot.order_board.clone();
    bot.run();
    let mut terminal = ratatui::init();
    let result = App::new(client, logs, order_board).run(&mut terminal);
    ratatui::restore();
    result
}
