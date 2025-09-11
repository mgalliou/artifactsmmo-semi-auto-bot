use anyhow::{Result, bail};
use artifactsmmo_sdk::{
    ItemContainer, Simulator,
    char::{HasCharacterData, Skill},
    events::EventSchemaExt,
};
use clap::{Parser, Subcommand, value_parser};
use rustyline::{DefaultEditor, error::ReadlineError};
use std::{process::exit, sync::Arc};

use crate::{
    HasReservation, bot::Bot, character::CharacterController, gear_finder::Filter,
    orderboard::Purpose,
};

pub fn run(bot: Arc<Bot>) -> Result<()> {
    let mut rl = DefaultEditor::new()?;
    let mut chars: Option<Arc<CharacterController>> = None;
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
            Ok(line) => match respond(&line, bot.clone(), &mut chars) {
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

fn respond(
    line: &str,
    bot: Arc<Bot>,
    character: &mut Option<Arc<CharacterController>>,
) -> Result<()> {
    match Cli::try_parse_from(line.split_whitespace())?.command {
        Commands::Orderboard { action } => match action {
            OrderboardAction::Add { item, quantity } => {
                bot.order_board.add(&item, quantity, None, Purpose::Cli)?;
            }
            OrderboardAction::Remove { item } => {
                let Some(o) = bot.order_board.get(&item, None, &Purpose::Cli) else {
                    bail!("order not found");
                };
                bot.order_board.remove(&o);
            }
            OrderboardAction::List => {
                println!("orders (by priority):");
                bot.order_board.orders_by_priority().iter().for_each(|o| {
                    println!(
                        "{}, in inventory: {}",
                        o,
                        bot.account.available_in_inventories(&o.item)
                    )
                });
            }
        },
        Commands::Bank { action } => match action {
            BankAction::Reservations => {
                println!("reservations:");
                bot.bank
                    .reservations()
                    .iter()
                    .for_each(|r| println!("{}", r));
            }
            BankAction::List => {
                bot.bank
                    .content()
                    .iter()
                    .for_each(|i| println!("{}: {}", i.code, i.quantity));
            }
            BankAction::Empty => {
                bail!("not yet implemented");
            }
        },
        Commands::Items { action } => match action {
            ItemsAction::TimeToGet { item } => println!("{:?}", bot.account.time_to_get(&item)),
            ItemsAction::Sources { item } => bot
                .client
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
                    bot.leveling_helper
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
                bot.leveling_helper
                    .best_crafts(char.skill_level(skill), skill)
                    .iter()
                    .for_each(|i| println!("{}", i.name))
            }
        },
        Commands::Events { action } => match action {
            EventsAction::List => {
                bot.client
                    .events
                    .all()
                    .iter()
                    .for_each(|e| println!("{}", e.to_string()));
            }
            EventsAction::Active => {
                bot.client
                    .events
                    .active()
                    .iter()
                    .for_each(|e| println!("{}", e.to_string()));
            }
        },
        Commands::Char { i } => {
            character.clone_from(&bot.account.get_character(i as usize));
            if let Some(char) = character.clone() {
                bail!("character '{}' selected", char.name());
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
            char.craft_from_bank(&item, quantity)?;
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
            available,
            craftable,
            from_task,
            from_monster,
            from_npc,
            utilities,
            winning,
            monster,
        } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let Some(monster) = bot.client.monsters.get(&monster) else {
                bail!("no character selected");
            };
            let filter = Filter {
                available,
                craftable,
                from_task,
                from_monster,
                from_npc,
                utilities,
            };
            let gear = if winning {
                bot.gear_finder.best_winning_against(char, &monster, filter)
            } else {
                Some(bot.gear_finder.best_against(char, &monster, filter))
            };
            if let Some(gear) = gear {
                println!("{gear}")
            } else {
                println!("no winning gear found")
            }
        }
        Commands::Simulate {
            available,
            craftable,
            from_task,
            from_npc,
            from_monster,
            utilities,
            winning,
            monster,
        } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let Some(monster) = bot.client.monsters.get(&monster) else {
                bail!("no character selected");
            };
            let filter = Filter {
                available,
                craftable,
                from_task,
                from_monster,
                from_npc,
                utilities,
            };
            let gear = if winning {
                bot.gear_finder.best_winning_against(char, &monster, filter)
            } else {
                Some(bot.gear_finder.best_against(char, &monster, filter))
            };
            if let Some(gear) = gear {
                println!("{}", gear);
                let fight = Simulator::average_fight(char.level(), 0, &gear, &monster, true);
                println!("{:?}", fight)
            } else {
                println!("no winning gear found")
            }
        }
        Commands::Deposit { item, quantity } => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            if item == "all" {
                char.deposit_all()?;
            } else {
                char.deposit_item(&item, quantity)?;
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
                char.config().enable_skill(skill);
            }
            SkillAction::Remove { skill } => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                char.config().disable_skill(skill);
            }
            SkillAction::List => {
                let Some(char) = character else {
                    bail!("no character selected");
                };
                char.config().skills().iter().for_each(|s| {
                    println!(
                        "{}({}): {}/{} ({}%)",
                        s,
                        char.skill_level(*s),
                        char.skill_xp(*s),
                        char.skill_max_xp(*s),
                        (f64::from(char.skill_xp(*s)) / f64::from(char.skill_max_xp(*s)) * 100.0)
                            .round() as u32
                    )
                });
            }
        },
        Commands::Map => {
            let Some(char) = character else {
                bail!("no character selected");
            };
            let (x, y) = char.position();
            println!("{:?}", bot.client.maps.get(x, y).unwrap());
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
        Commands::Config { action } => match action {
            ConfigAction::Reload => bot.config.reload(),
        },
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
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
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
        #[arg(value_parser = value_parser!(u32), default_value = "0")]
        i: u32,
    },
    Map,
    Task,
    Status,
    Idle,
    Craft {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: u32,
    },
    Recycle {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: u32,
    },
    Delete {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: u32,
    },
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    Gear {
        #[arg(short = 'a', long)]
        available: bool,
        #[arg(short = 'c', long)]
        craftable: bool,
        #[arg(short = 't', long)]
        from_task: bool,
        #[arg(short = 'n', long)]
        from_npc: bool,
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
        craftable: bool,
        #[arg(short = 't', long)]
        from_task: bool,
        #[arg(short = 'n', long)]
        from_npc: bool,
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
        quantity: u32,
    },
    Unequip {
        slot: String,
        #[arg(default_value_t = 1)]
        quantity: u32,
    },
}

#[derive(Subcommand)]
#[command(alias = "cfg")]
enum ConfigAction {
    #[command(alias = "rl")]
    Reload,
}

#[derive(Subcommand)]
#[command(alias = "ob")]
enum OrderboardAction {
    Add {
        item: String,
        #[arg(default_value_t = 1)]
        quantity: u32,
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
        skill: Skill,
    },
    #[command(alias = "rm")]
    Remove {
        skill: Skill,
    },
    #[command(alias = "l")]
    List,
}
