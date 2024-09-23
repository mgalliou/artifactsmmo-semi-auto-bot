use artifactsmmo_openapi::models::CharacterSchema;
use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account, api::my_character::MyCharacterApi, bank::Bank, char_config::CharConfig,
    character::Character, config::Config, game::Game, items::Items, skill::Skill,
};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use itertools::Itertools;
use log::LevelFilter;
use rustyline::Result;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{
    str::FromStr,
    sync::{Arc, RwLock},
};

fn main() -> Result<()> {
    let _ = simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Debug);
    let config: Config = Figment::new()
        .merge(Toml::file_exact("ArtifactsMMO.toml"))
        .extract()
        .unwrap();
    let account = Arc::new(Account::new(&config));
    let game = Arc::new(Game::new(&config));
    let bank = Arc::new(Bank::new(&config, game.items.clone()));
    let chars_conf = init_char_conf(&config.characters);
    let chars_schema = init_chars_schema(config);
    let characters = chars_conf
        .into_iter()
        .zip(chars_schema.iter())
        .map(|(conf, schema)| {
            Character::new(
                account.clone(),
                game.clone(),
                bank.clone(),
                conf.clone(),
                schema.clone(),
            )
        })
        .collect_vec();
    let handles = characters
        .into_iter()
        .map(|c| Character::run(c).unwrap())
        .collect_vec();
    run_command_line(chars_schema, game.items.clone())?;
    handles.into_iter().for_each(|h| {
        h.join().unwrap();
    });
    Ok(())
}

fn run_command_line(
    chars_schema: Vec<Arc<RwLock<CharacterSchema>>>,
    items: Arc<Items>,
) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let args = line.split_whitespace().collect_vec();
                match args.first() {
                    Some(cmd) => match *cmd {
                        "info" => println!("{:#?}", chars_schema[0].read().unwrap()),
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

fn init_char_conf(confs: &[CharConfig]) -> Vec<Arc<RwLock<CharConfig>>> {
    confs
        .iter()
        .map(|c| Arc::new(RwLock::new(c.clone())))
        .collect_vec()
}

fn init_chars_schema(config: Config) -> Vec<Arc<RwLock<CharacterSchema>>> {
    let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
    my_characters_api
        .characters()
        .unwrap()
        .data
        .into_iter()
        .map(|s| Arc::new(RwLock::new(s)))
        .collect_vec()
}
