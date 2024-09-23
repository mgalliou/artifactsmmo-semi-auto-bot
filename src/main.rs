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
    run_command_line(account.characters.clone(), game.items.clone())?;
    handles.into_iter().for_each(|h| {
        h.join().unwrap();
    });
    Ok(())
}

fn run_command_line(_characters: Arc<Vec<Arc<Character>>>, items: Arc<Items>) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let args = line.split_whitespace().collect_vec();
                match args.first() {
                    Some(cmd) => match *cmd {
                        "items" => match args.get(1) {
                            Some(verb) => match (*verb, args.get(2), args.get(3)) {
                                ("bfl", Some(lvl), Some(skill)) => println!(
                                    "{:#?}",
                                    items.best_for_leveling(
                                        lvl.parse().unwrap(),
                                        Skill::from_str(skill).unwrap()
                                    )
                                ),
                                _ => println!("error"),
                            },

                            None => println!("error"),
                        },
                        _ => println!("error"),
                    },
                    None => todo!(),
                }
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
