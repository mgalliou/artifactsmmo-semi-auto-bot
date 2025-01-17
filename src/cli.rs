use anyhow::{bail, Result};
use artifactsmmo_sdk::{
    self,
    char::{character::PostCraftAction, Character, HasCharacterData, Skill},
    events::EventSchemaExt,
    fight_simulator::FightSimulator,
    game::Game,
    gear_finder::Filter,
    orderboard::Purpose,
};
use clap::{value_parser, Parser, Subcommand};
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{process::exit, str::FromStr, sync::Arc};

pub fn run_cli(game: &Game) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut chars: Option<Arc<Character>> = None;
    loop {
        let readline = rl.readline(
            format!(
                "{} >> ",
                chars
                    .as_ref()
                    .map(|c| c.name().to_string())
                    .unwrap_or("none".to_string())
            )
            .as_str(),
        );
        match readline {
            Ok(line) => match respond(&line, &mut chars, game) {
                Ok(_) => {
                    if let Err(e) = rl.add_history_entry(line.as_str()) {
                        eprintln!("failed to add history entry: {}", e);
                    }
                }
                Err(e) => eprintln!("{}", e),
            },
            Err(ReadlineError::Interrupted) => {
                println!("Quit");
            }
            Err(ReadlineError::Eof) => {
                println!("quit");
                exit(0);
            }
            Err(err) => {
                println!("Error: {:#?}", err);
            }
        }
    }
}

fn respond(line: &str, character: &mut Option<Arc<Character>>, game: &Game) -> Result<()> {
    match Cli::try_parse_from(line.split_whitespace())?.command {
        Commands::Orderboard { action } => match action {
            OrderboardAction::Add { item, quantity } => {
                game.orderboard.add(None, &item, quantity, Purpose::Cli)?;
            }
            OrderboardAction::Remove { item } => {
                let Some(o) = game.orderboard.get(None, &item, &Purpose::Cli) else {
                    bail!("order not found");
                };
                game.orderboard.remove(&o)?
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
                bail!("not yet implemented");
            }
        },
        Commands::Items { action } => match action {
            ItemsAction::TimeToGet { item } => println!("{:?}", game.account.time_to_get(&item)),
            ItemsAction::Sources { item } => game
                .items
                .sources_of(&item)
                .iter()
                .for_each(|s| println!("{:?}", s)),
            ItemsAction::BestCraft { skill } => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                println!(
                    "best {} craft: {:?}",
                    skill,
                    game.leveling_helper
                        .best_craft(char.skill_level(skill), skill, char)
                        .map(|i| i.name.clone())
                        .unwrap_or("none".to_string())
                );
            }
            ItemsAction::BestCrafts { skill } => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                println!("best {} crafts:", skill);
                game.leveling_helper
                    .best_crafts(char.skill_level(skill), skill)
                    .iter()
                    .for_each(|i| println!("{}", i.name))
            }
        },
        Commands::Events { action } => match action {
            EventsAction::List => {
                game.events
                    .data
                    .iter()
                    .for_each(|e| println!("{}", e.to_string()));
            }
            EventsAction::Active => {
                game.events
                    .active
                    .read()
                    .unwrap()
                    .iter()
                    .for_each(|e| println!("{}", e.to_string()));
            }
        },
        Commands::Char { i } => {
            character.clone_from(&game.account.get_character(i as usize));
            if let Some(char) = character.clone() {
                bail!("character '{}' selected", char.base.name());
            } else {
                bail!("character not found");
            }
        }
        Commands::Status => todo!(),
        Commands::Idle => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            char.toggle_idle();
        }
        Commands::Craft { item, quantity } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            char.craft_from_bank(&item, quantity, PostCraftAction::Keep)?;
        }
        Commands::Recycle { item, quantity } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            char.recycle_item(&item, quantity)?;
        }
        Commands::Delete { item, quantity } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            char.delete_item(&item, quantity)?;
        }
        Commands::Gear {
            can_craft,
            from_task,
            from_monster,
            from_gift,
            available,
            utilities,
            winning,
            monster,
        } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let Some(monster) = game.monsters.get(&monster) else {
                bail!("no character selected");
            };
            let filter = Filter {
                available,
                can_craft,
                from_task,
                from_monster,
                from_gift,
                utilities,
            };
            println!(
                "{}",
                if winning {
                    game.gear_finder.best_winning_against(char, monster, filter)
                } else {
                    game.gear_finder.best_against(char, monster, filter)
                }
            );
        }
        Commands::Simulate {
            available,
            can_craft,
            from_task,
            from_gift,
            from_monster,
            utilities,
            winning,
            monster,
        } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let Some(monster) = game.monsters.get(&monster) else {
                bail!("no character selected");
            };
            let filter = Filter {
                available,
                can_craft,
                from_task,
                from_monster,
                from_gift,
                utilities,
            };
            let gear = if winning {
                game.gear_finder.best_winning_against(char, monster, filter)
            } else {
                game.gear_finder.best_against(
                    char,
                    monster,
                    Filter {
                        available,
                        can_craft,
                        from_task,
                        from_monster,
                        from_gift,
                        utilities,
                    },
                )
            };
            println!("{}", gear);
            let fight = FightSimulator::new().simulate(char.level(), 0, &gear, monster, true);
            println!("{:?}", fight)
        }
        Commands::Deposit { item, quantity } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            if item == "all" {
                char.deposit_all();
            } else {
                char.deposit_item(&item, quantity, None)?;
            }
        }
        Commands::Unequip {
            slot: _,
            quantity: _,
        } => bail!("not yet implemented"),
        Commands::Skill { action } => match action {
            SkillAction::Add { skill } => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                char.conf()
                    .write()
                    .unwrap()
                    .skills
                    .insert(Skill::from_str(&skill).unwrap());
            }
            SkillAction::Remove { skill } => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                char.conf()
                    .write()
                    .unwrap()
                    .skills
                    .remove(&Skill::from_str(&skill).unwrap());
            }
            SkillAction::List => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                char.conf().read().unwrap().skills.iter().for_each(|s| {
                    println!(
                        "{}({}): {}/{} ({}%)",
                        s,
                        char.skill_level(*s),
                        char.skill_xp(*s),
                        char.skill_max_xp(*s),
                        (f64::from(char.skill_xp(*s)) / f64::from(char.skill_max_xp(*s)) * 100.0)
                            .round() as i32
                    )
                });
            }
        },
        Commands::Map => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let (x, y) = char.base.position();
            println!("{:?}", game.maps.get(x, y).unwrap());
        }
        Commands::Task => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            println!(
                "{} ({:?}) {}/{}",
                char.task(),
                char.task_type(),
                char.task_progress(),
                char.task_total()
            );
        }
    }
    Ok(())
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
    Events {
        #[command(subcommand)]
        action: EventsAction,
    },
    Char {
        #[arg(value_parser = value_parser!(i32), default_value = "0")]
        i: i32,
    },
    Map,
    Task,
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
        #[arg(short = 'a', long)]
        available: bool,
        #[arg(short = 'c', long)]
        can_craft: bool,
        #[arg(short = 't', long)]
        from_task: bool,
        #[arg(short = 'g', long)]
        from_gift: bool,
        #[arg(short = 'm', long)]
        from_monster: bool,
        #[arg(short = 'u', long)]
        utilities: bool,
        #[arg(short = 'w', long)]
        winning: bool,
        monster: String,
    },
    #[command(alias = "sim")]
    Simulate {
        #[arg(short = 'a', long)]
        available: bool,
        #[arg(short = 'c', long)]
        can_craft: bool,
        #[arg(short = 't', long)]
        from_task: bool,
        #[arg(short = 'g', long)]
        from_gift: bool,
        #[arg(short = 'm', long)]
        from_monster: bool,
        #[arg(short = 'u', long)]
        utilities: bool,
        #[arg(short = 'w', long)]
        winning: bool,
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
#[command(alias = "b")]
enum BankAction {
    #[command(alias = "res")]
    Reservations,
    Empty,
    #[command(alias = "l")]
    List,
}

#[derive(Subcommand)]
enum ItemsAction {
    #[command(alias = "ttg")]
    TimeToGet {
        item: String,
    },
    BestCraft {
        skill: Skill,
    },
    BestCrafts {
        skill: Skill,
    },
    Sources {
        item: String,
    },
}

#[derive(Subcommand)]
#[command(alias = "e")]
enum EventsAction {
    #[command(alias = "l")]
    List,
    #[command(alias = "a")]
    Active,
}

#[derive(Subcommand)]
#[command(alias = "s")]
enum SkillAction {
    Add {
        skill: String,
    },
    #[command(alias = "rm")]
    Remove {
        skill: String,
    },
    #[command(alias = "l")]
    List,
}
