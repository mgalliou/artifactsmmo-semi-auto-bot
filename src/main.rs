use artifactsmmo_playground::artifactsmmo_sdk::{
    character::Character,
    fight_simulator::FightSimulator,
    game::Game,
    gear_finder::{Filter, GearFinder},
    orderboard::Purpose,
    skill::Skill,
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
    let _ = game.orderboard.add(None, "snowman_hat", 10, Purpose::Cli);
    let _ = game.orderboard.add(None, "lizard_skin", 1000, Purpose::Cli);
    let _ = game.orderboard.add(None, "demon_horn", 1000, Purpose::Cli);
    let _ = game
        .orderboard
        .add(None, "malefic_cloth", 200, Purpose::Cli);
    let _ = game.orderboard.add(None, "strange_ore", 6000, Purpose::Cli);
    let _ = game.orderboard.add(None, "magic_wood", 6000, Purpose::Cli);
    //let _ = game.orderboard.add(None, "frozen_pickaxe", 5, Purpose::Cli);
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
    let mut character: Option<Arc<Character>> = None;
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => match respond(&line, &mut character, game) {
                Ok(_) => {
                    if let Err(e) = rl.add_history_entry(line.as_str()) {
                        eprintln!("failed to add history entry: {}", e);
                    }
                }
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

fn respond(
    line: &str,
    character: &mut Option<Arc<Character>>,
    game: &Game,
) -> Result<bool, String> {
    let args = line.split_whitespace().collect_vec();
    let gear_finder = GearFinder::new(&game.items);
    let cli = Cli::try_parse_from(args).map_err(|err| format!("{}", err))?;
    match cli.command {
        Commands::Orderboard { action } => match action {
            OrderboardAction::Add { item, quantity } => {
                if let Err(e) = game.orderboard.add(None, &item, quantity, Purpose::Cli) {
                    println!("failed to add order: {:?}", e);
                }
            }
            OrderboardAction::Remove { item } => {
                if let Some(o) = game.orderboard.get(None, &item, &Purpose::Cli) {
                    if let Err(e) = game.orderboard.remove(&o) {
                        println!("failed to remove order: {:?}", e);
                    }
                }
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
            ItemsAction::Sources { item } => game
                .items
                .sources_of(&item)
                .iter()
                .for_each(|s| println!("{:?}", s)),
        },
        Commands::Char { i } => {
            character.clone_from(&game.account.get_character(i as usize));
            if let Some(char) = character.clone() {
                println!("character '{}' selected", char.name);
            } else {
                println!("character not found");
            }
        }
        Commands::Status => todo!(),
        Commands::Idle => {
            if let Some(char) = character {
                char.toggle_idle();
            } else {
                println!("no character selected");
            }
        }
        Commands::Craft { item, quantity } => {
            if let Some(char) = character {
                char.craft_items(&item, quantity);
            } else {
                println!("no character selected");
            }
        }
        Commands::Recycle { item, quantity } => {
            if let Some(char) = character {
                char.recycle_item(&item, quantity)
                    .map_err(|e| e.to_string())?;
            } else {
                println!("no character selected");
            }
        }
        Commands::Delete { item, quantity } => {
            if let Some(char) = character {
                char.delete_item(&item, quantity)
                    .map_err(|e| e.to_string())?;
            } else {
                println!("no character selected");
            }
        }
        Commands::Gear { filter, monster } => {
            if let Some(char) = character {
                if let Some(monster) = game.monsters.get(&monster) {
                    println!(
                        "{}",
                        gear_finder.best_against(char, monster, Filter::from_str(&filter).unwrap())
                    );
                } else {
                    println!("monster not found");
                }
            } else {
                println!("no character selected");
            }
        }
        Commands::Simulate { monster } => {
            if let Some(char) = character {
                if let Some(monster) = game.monsters.get(&monster) {
                    let gear = gear_finder.best_against(char, monster, Filter::Available);
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
        Commands::Skill { action } => match action {
            SkillAction::Add { skill } => {
                if let Some(char) = character {
                    char.conf
                        .write()
                        .unwrap()
                        .skills
                        .insert(Skill::from_str(&skill).unwrap());
                }
            }
            SkillAction::Remove { skill } => {
                if let Some(char) = character {
                    char.conf
                        .write()
                        .unwrap()
                        .skills
                        .remove(&Skill::from_str(&skill).unwrap());
                }
            }
            SkillAction::List => {
                if let Some(char) = character {
                    char.conf.read().unwrap().skills.iter().for_each(|s| {
                        println!(
                            "{}({}): {}/{} ({}%)",
                            s,
                            char.skill_level(*s),
                            char.skill_xp(*s),
                            char.skill_max_xp(*s),
                            (f64::from(char.skill_xp(*s)) / f64::from(char.skill_max_xp(*s))
                                * 100.0)
                                .round() as i32
                        )
                    });
                }
            }
        },
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
        #[arg(value_parser = value_parser!(i32), default_value = "0")]
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
    Delete {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: i32,
    },
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    Gear {
        #[arg(short = 'f', long = "filter", value_parser = value_parser!(String), default_value = "all")]
        filter: String,
        monster: String,
    },
    #[command(alias = "sim")]
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
    #[command(alias = "rm")]
    Remove { item: String },
    #[command(alias = "l")]
    List,
}

#[derive(Subcommand)]
enum BankAction {
    #[command(alias = "res")]
    Reservations,
    Empty,
    List,
}

#[derive(Subcommand)]
enum ItemsAction {
    #[command(alias = "ttg")]
    TimeToGet {
        item: String,
    },
    Sources {
        item: String,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    Add {
        skill: String,
    },
    #[command(alias = "rm")]
    Remove {
        skill: String,
    },
    List,
}
