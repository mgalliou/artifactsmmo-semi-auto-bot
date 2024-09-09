use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account, api::my_character::MyCharacterApi, bank::Bank, character::Character,
    config::Config, items::Items, maps::Maps, monsters::Monsters, resources::Resources,
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
    sync::{Arc, RwLock},
    thread::JoinHandle,
};

fn main() -> Result<()> {
    let _ = simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info);
    let config: Config = Figment::new()
        .merge(Toml::file_exact("ArtifactsMMO.toml"))
        .extract()
        .unwrap();
    let account = Account::new(&config.base_url, &config.token);
    let maps = Arc::new(Maps::new(&account));
    let resources = Arc::new(Resources::new(&account));
    let monsters = Arc::new(Monsters::new(&account));
    let items = Arc::new(Items::new(&account, resources.clone(), monsters.clone()));
    let bank = Arc::new(RwLock::new(Bank::new(&account, items.clone())));
    let my_characters_api = MyCharacterApi::new(&config.base_url, &config.token);
    let chars_schema = my_characters_api
        .characters()
        .unwrap()
        .data
        .into_iter()
        .map(|s| Arc::new(RwLock::new(s)))
        .collect_vec();
    let chars_conf = config
        .characters
        .into_iter()
        .map(|c| Arc::new(RwLock::new(c)))
        .collect_vec();
    let characters = chars_conf
        .into_iter()
        .zip(chars_schema.iter())
        .map(|(conf, schema)| {
            Character::new(
                &account,
                maps.clone(),
                resources.clone(),
                monsters.clone(),
                items.clone(),
                bank.clone(),
                conf.clone(),
                schema.clone(),
            )
        })
        .collect_vec();
    let mut handles: Vec<JoinHandle<()>> = vec![];
    for c in characters.into_iter() {
        handles.push(Character::run(c)?);
    }
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let args = line.split_whitespace().collect_vec();
                match args.first() {
                    Some(cmd) => match *cmd {
                        "info" => println!("{:?}", chars_schema[0].read().unwrap()),
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
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    for handle in handles {
        handle.join().unwrap();
    }
    Ok(())
}
