use artifactsmmo_playground::artifactsmmo_sdk::{
    character::Character,
    fight_simulator::FightSimulator,
    game::Game,
    gear_finder::{Filter, GearFinder},
    orderboard::{Order, Purpose},
};
use clap::{value_parser, Parser, Subcommand};
use itertools::Itertools;
use log::LevelFilter;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{str::FromStr, sync::Arc, thread::sleep, time::Duration};

fn main() -> rustyline::Result<()> {
    let _ = simple_logging::log_to_file("artifactsmmo.log", LevelFilter::Info);
    let game = Game::new();
    game.init();
    let handles = game
        .account
        .characters
        .read()
        .unwrap()
        .iter()
        .map(|c| {
            sleep(Duration::from_millis(250));
            Character::run(c.clone()).unwrap()
        })
        .collect_vec();
    run_cli(&game)?;
    handles.into_iter().for_each(|h| {
        h.join().unwrap();
    });
    Ok(())
}

fn run_cli(game: &Game) -> rustyline::Result<()> {
    let mut rl = DefaultEditor::new()?;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => match respond(line, game) {
                Ok(_) => {}
                Err(e) => eprintln!("{}", e),
            },
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

static mut CHAR: Option<Arc<Character>> = None;

fn respond(line: String, game: &Game) -> Result<bool, String> {
    let args = line.split_whitespace().collect_vec();
    let gear_finder = GearFinder::new(&game.items);
    let cli = Cli::try_parse_from(args).map_err(|err| format!("{}", err))?;
    match cli.command {
        Commands::Orderboard { action } => match action {
            OrderboardAction::Add { item, quantity } => {
                game.orderboard
                    .add(Order::new(None, &item, quantity, Purpose::Cli));
            }
            OrderboardAction::Remove { item, quantity } => {
                game.orderboard
                    .remove(&Order::new(None, &item, quantity, Purpose::Cli));
            }
            OrderboardAction::List => {
                println!("orders (by priority):");
                game.orderboard.orders_by_priority().iter().for_each(|o| {
                    println!(
                        "{}, in inventory: {}",
                        o,
                        game.account.available_in_inventories(&o.item)
                    )
                });
            }
        },
        Commands::Bank { action } => match action {
            BankAction::Reservations => {
                println!("reservations:");
                game.account
                    .bank
                    .reservations()
                    .iter()
                    .for_each(|r| println!("{}", r));
            }
            BankAction::List => {
                game.account
                    .bank
                    .content
                    .read()
                    .unwrap()
                    .iter()
                    .for_each(|i| println!("{}: {}", i.code, i.quantity));
            }
            BankAction::Empty => {
                println!("not yet implemented");
            }
        },
        Commands::Items { action } => match action {
            ItemsAction::TimeToGet { item } => println!("{:?}", game.account.time_to_get(&item)),
        },
        Commands::Char { i } => {
            unsafe { CHAR = game.account.get_character(i as usize) };
            if let Some(char) = unsafe { CHAR.clone() } {
                println!("character '{}' selected", char.name);
            } else {
                println!("character not found");
            }
        }
        Commands::Status => todo!(),
        Commands::Idle => {
            if let Some(char) = unsafe { CHAR.clone() } {
                char.toggle_idle();
            } else {
                println!("no character selected");
            }
        }
        Commands::Craft { item, quantity } => {
            if let Some(char) = unsafe { CHAR.clone() } {
                char.craft_items(&item, quantity);
            } else {
                println!("no character selected");
            }
        }
        Commands::Recycle { item, quantity } => {
            if let Some(char) = unsafe { CHAR.clone() } {
                char.recycle_item(&item, quantity)
                    .map_err(|e| e.to_string())?;
            } else {
                println!("no character selected");
            }
        }
        Commands::Gear { filter, monster } => {
            if let Some(char) = unsafe { CHAR.clone() } {
                if let Some(monster) = game.monsters.get(&monster) {
                    println!(
                        "{}",
                        gear_finder.best_against(
                            &char,
                            monster,
                            Filter::from_str(&filter).unwrap()
                        )
                    );
                } else {
                    println!("monster not found");
                }
            } else {
                println!("no character selected");
            }
        }
        Commands::Simulate { monster } => {
            if let Some(char) = unsafe { CHAR.clone() } {
                if let Some(monster) = game.monsters.get(&monster) {
                    let gear = gear_finder.best_against(&char, monster, Filter::Available);
                    let fight = FightSimulator::new().simulate(char.level(), 0, &gear, monster);
                    println!("{:?}", fight)
                } else {
                    println!("monster not found");
                }
            } else {
                println!("no character selected");
            }
        }
        Commands::Deposit {
            item: _,
            quantity: _,
        } => println!("not yet implemented"),
        Commands::Unequip {
            slot: _,
            quantity: _,
        } => println!("not yet implemented"),
    }
    Ok(true)
}

#[derive(Parser)]
#[command(
    version,
    subcommand_required = true,
    subcommand_value_name = "ARTIFACTS_MMO",
    multicall = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Orderboard {
        #[command(subcommand)]
        action: OrderboardAction,
    },
    Bank {
        #[command(subcommand)]
        action: BankAction,
    },
    Items {
        #[command(subcommand)]
        action: ItemsAction,
    },
    Char {
        #[arg(value_parser = value_parser!(i32), default_value = "1")]
        i: i32,
    },
    Status,
    Idle,
    Craft {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    Recycle {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    Gear {
        #[arg(short = 'f', long = "filter", value_parser = value_parser!(String), default_value = "all")]
        filter: String,
        monster: String,
    },
    Simulate {
        monster: String,
    },
    Deposit {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    Unequip {
        slot: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
}

#[derive(Subcommand)]
#[command(alias = "ob")]
enum OrderboardAction {
    Add {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    Remove {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    List,
}

#[derive(Subcommand)]
enum BankAction {
    Reservations,
    Empty,
    List,
}

#[derive(Subcommand)]
enum ItemsAction {
    #[command(alias = "ttg")]
    TimeToGet { item: String },
}
