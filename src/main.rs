use artifactsmmo_playground::artifactsmmo_sdk::{
    account::Account,
    bank::Bank,
    character::Character,
    game::Game,
    items::Items,
    orderboard::{Order, OrderBoard, Purpose},
    skill::Skill,
};
use figment::{
    providers::{Format, Toml},
    Figment,
};
use itertools::Itertools;
use log::LevelFilter;
use rustyline::Result;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{str::FromStr, sync::Arc, thread::sleep, time::Duration};

fn main() -> Result<()> {
    let _ = simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info);
    let config = Arc::new(
        Figment::new()
            .merge(Toml::file_exact("ArtifactsMMO.toml"))
            .extract()
            .unwrap(),
    );
    let game = Arc::new(Game::new(&config));
    let account = Account::new(&config, &game);
    let handles = account
        .characters
        .read()
        .unwrap()
        .iter()
        .map(|c| {
            sleep(Duration::from_millis(250));
            Character::run(c.clone()).unwrap()
        })
        .collect_vec();
    run_cli(&game, &account)?;
    handles.into_iter().for_each(|h| {
        h.join().unwrap();
    });
    Ok(())
}

fn run_cli(game: &Arc<Game>, account: &Account) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                handle_cmd_line(line, game, account);
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

fn handle_cmd_line(line: String, game: &Arc<Game>, account: &Account) {
    let args = line.split_whitespace().collect_vec();
    if let Some(cmd) = args.first() {
        match *cmd {
            "items" => handle_items(&args[1..], &game.items),
            "char" => handle_char(&args[1..], account),
            "orderboard" => handle_orderboard(&args[1..], &game.orderboard),
            "bank" => handle_bank(&args[1..], &account.bank),
            _ => println!("error"),
        }
    }
}

fn handle_bank(args: &[&str], bank: &Bank) {
    match args.first() {
        Some(verb) => match *verb {
            "res" => {
                println!("reservations:");
                bank.reservations().iter().for_each(|r| println!("{}", r));
            }
            _ => println!("invalid verb"),
        },
        None => eprint!("missing verb"),
    }
}

fn handle_char(args: &[&str], account: &Account) {
    if let (Some(verb), Some(name)) = (args.first(), args.get(1)) {
        match account.get_character_by_name(name) {
            Some(char) => match *verb {
                "idle" => char.toggle_idle(),
                "fight" => {
                    let _ = char.action_fight();
                }
                "craft" => match (args.get(2), args.get(3)) {
                    (Some(code), Some(quantity)) => {
                        char.craft_items(code, quantity.parse::<i32>().unwrap_or(0));
                    }
                    (Some(code), None) => {
                        char.craft_items(code, 1);
                    }
                    _ => eprint!("missing args"),
                },
                "recycle" => match (args.get(2), args.get(3)) {
                    (Some(code), Some(quantity)) => {
                        char.recycle_from_bank(code, quantity.parse::<i32>().unwrap_or(1));
                    }
                    (Some(code), None) => {
                        char.recycle_from_bank(code, 1);
                    }
                    _ => eprint!("missing args"),
                },
                "unequip_all" => char.unequip_and_deposit_all(),
                "deposit_all" => char.deposit_all(),
                "empty_bank" => char.empty_bank(),
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

fn handle_orderboard(args: &[&str], orderboard: &Arc<OrderBoard>) {
    match args.first() {
        Some(verb) => match *verb {
            "request" => match (args.get(1), args.get(2)) {
                (Some(item), Some(quantity)) => {
                    orderboard.add(Order::new(
                        None,
                        item,
                        quantity.parse::<i32>().unwrap_or(0),
                        Purpose::Cli,
                    ));
                }
                _ => eprintln!("missings args"),
            },
            "orders" => {
                println!("orders:");
                orderboard
                    .orders_by_priority()
                    .iter()
                    .for_each(|o| println!("{}", o));
            }
            "prio" => {
                println!("orders:");
                orderboard
                    .orders_by_priority()
                    .iter()
                    .for_each(|o| println!("{}", o));
            }
            _ => println!("invalid verb"),
        },
        None => eprint!("missing verb"),
    }
}
