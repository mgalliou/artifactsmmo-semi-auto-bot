use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account, character::Character, config::Config, game::Game, items::Items, skill::Skill,
};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use itertools::Itertools;
use log::LevelFilter;
use rustyline::Result;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{str::FromStr, sync::Arc};

fn main() -> Result<()> {
    let _ = simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Debug);
    let config: Config = Figment::new()
        .merge(Toml::file_exact("ArtifactsMMO.toml"))
        .extract()
        .unwrap();
    let game = Arc::new(Game::new(&config));
    let account = Account::new(&config, game.clone());
    let handles = account
        .characters
        .iter()
        .map(|c| Character::run(c.clone()).unwrap())
        .collect_vec();
    run_cli(game.clone(), &account)?;
    handles.into_iter().for_each(|h| {
        h.join().unwrap();
    });
    Ok(())
}

fn run_cli(game: Arc<Game>, account: &Account) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                handle_cmd_line(line, game.clone(), account);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:#?}", err);
                break;
            }
        }
    }
    Ok(())
}

fn handle_cmd_line(line: String, game: Arc<Game>, account: &Account) {
    let args = line.split_whitespace().collect_vec();
    if let Some(cmd) = args.first() {
        match *cmd {
            "items" => handle_items(&args[1..], &game.items),
            "char" => handle_char(&args[1..], account),
            _ => println!("error"),
        }
    }
}

fn handle_char(args: &[&str], account: &Account) {
    if let (Some(verb), Some(name)) = (args.first(), args.get(1)) {
        match account.get_character_by_name(name) {
            Some(char) => match *verb {
                "idle" => char.toggle_idle(),
                "fight" => {
                    char.action_fight();
                }
                "craft" => match (args.get(2), args.get(3)) {
                    (Some(code), Some(quantity)) => {
                        char.craft_from_bank(code, quantity.parse::<i32>().unwrap_or(0));
                    }
                    (Some(code), None) => {
                        char.craft_from_bank(code, 1);
                    }
                    _ => eprint!("missing args"),
                },
                _ => eprintln!("invalid verb"),
            },
            _ => eprintln!("character not found: {}", name),
        }
    }
}

fn handle_items(args: &[&str], items: &Arc<Items>) {
    if let Some(verb) = args.first() {
        match (*verb, args.get(1), args.get(2)) {
            ("bfl", Some(lvl), Some(skill)) => println!(
                "{:#?}",
                items.best_for_leveling(lvl.parse().unwrap(), Skill::from_str(skill).unwrap())
            ),
            _ => println!("error"),
        };
    }
}
