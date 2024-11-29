use artifactsmmo_playground::artifactsmmo_sdk::{
    character::Character,
    fight_simulator::FightSimulator,
    game::Game,
    gear_finder::{Filter, GearFinder},
    orderboard::{Order, Purpose},
};
use clap::{arg, value_parser, Command};
use itertools::Itertools;
use log::LevelFilter;
use rustyline::{error::ReadlineError, DefaultEditor};
use std::{sync::Arc, thread::sleep, time::Duration};

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

fn cli() -> Command {
    const PARSER_TEMPLATE: &str = "\
        {all-args}
    ";
    // strip out name/version
    const APPLET_TEMPLATE: &str = "\
        {about-with-newline}\n\
        {usage-heading}\n    {usage}\n\
        \n\
        {all-args}{after-help}\
    ";
    Command::new("artifactsmmo-playground")
        .multicall(true)
        .arg_required_else_help(true)
        .subcommand_required(true)
        .subcommand_value_name("ARTIFACTS_MMO")
        .subcommand_help_heading("ARTIFACTS_MMO")
        .help_template(PARSER_TEMPLATE)
        .subcommand(
            Command::new("orderboard")
                .alias("ob")
                .subcommand(
                    Command::new("add")
                        .arg_required_else_help(true)
                        .arg(arg!(item: [ITEM]))
                        .arg(
                            arg!(quantity: [QUANTITY])
                                .default_value("1")
                                .value_parser(value_parser!(i32)),
                        )
                        .help_template(APPLET_TEMPLATE),
                )
                .subcommand(
                    Command::new("remove")
                        .alias("rm")
                        .arg_required_else_help(true)
                        .arg(arg!(item: [ITEM]))
                        .arg(
                            arg!(quantity: [QUANTITY])
                                .default_value("1")
                                .value_parser(value_parser!(i32)),
                        )
                        .help_template(APPLET_TEMPLATE),
                ),
        )
        .subcommand(
            Command::new("bank")
                .subcommand(
                    Command::new("reservations")
                        .alias("res")
                        .help_template(APPLET_TEMPLATE),
                )
                .subcommand(Command::new("empty").help_template(APPLET_TEMPLATE)),
        )
        .subcommand(
            Command::new("char")
                .arg(arg!(i: [INDEX]).value_parser(value_parser!(i32)))
                .help_template(APPLET_TEMPLATE),
        )
        .subcommand(Command::new("status").help_template(APPLET_TEMPLATE))
        .subcommand(Command::new("idle").help_template(APPLET_TEMPLATE))
        .subcommand(
            Command::new("craft")
                .arg(arg!(item: [ITEM]))
                .arg(
                    arg!(quantity: [QUANTITY])
                        .default_value("1")
                        .value_parser(value_parser!(i32)),
                )
                .help_template(APPLET_TEMPLATE),
        )
        .subcommand(
            Command::new("recycle")
                .arg(arg!(item: [ITEM]))
                .arg(
                    arg!(quantity: [QUANTITY])
                        .default_value("1")
                        .value_parser(value_parser!(i32)),
                )
                .help_template(APPLET_TEMPLATE),
        )
        .subcommand(Command::new("gear").arg(arg!(monster: [MONSTER])))
        .subcommand(
            Command::new("simulate")
                .alias("sim")
                .arg(arg!(monster: [MONSTER]))
                .help_template(APPLET_TEMPLATE),
        )
        .subcommand(
            Command::new("deposit")
                .arg(arg!(item: [ITEM]))
                .arg(
                    arg!(quantity: [QUANTITY])
                        .default_value("1")
                        .value_parser(value_parser!(i32)),
                )
                .help_template(APPLET_TEMPLATE),
        )
        .subcommand(
            Command::new("unequip")
                .arg(arg!(slot: [SLOT]))
                .arg(
                    arg!(quantity: [QUANTITY])
                        .default_value("1")
                        .value_parser(value_parser!(i32)),
                )
                .help_template(APPLET_TEMPLATE),
        )
}

static mut CHAR: Option<Arc<Character>> = None;

fn respond(line: String, game: &Game) -> Result<bool, String> {
    let args = line.split_whitespace().collect_vec();
    let gear_finder = GearFinder::new(&game.items);
    let matches = cli()
        .try_get_matches_from(args)
        .map_err(|err| format!("{}", err))?;
    match matches.subcommand() {
        Some(("orderboard", ob_matches)) => match ob_matches.subcommand() {
            Some(("add", add_matches)) => {
                let item = add_matches
                    .get_one::<String>("item")
                    .map(|s| s.as_str())
                    .unwrap_or("none");
                let quantity = add_matches.get_one::<i32>("quantity").unwrap_or(&1);
                game.orderboard
                    .add(Order::new(None, item, *quantity, Purpose::Cli));
            }
            Some(("remove", remove_matches)) => {
                let item = remove_matches
                    .get_one::<String>("item")
                    .map(|s| s.as_str())
                    .unwrap_or("none");
                let quantity = remove_matches.get_one::<i32>("quantity").unwrap_or(&1);
                game.orderboard
                    .remove(&Order::new(None, item, *quantity, Purpose::Cli));
            }
            None => {
                println!("orders (by priority):");
                game.orderboard.orders_by_priority().iter().for_each(|o| {
                    println!(
                        "{}, in inventory: {}",
                        o,
                        game.account.available_in_inventories(&o.item)
                    )
                });
            }
            _ => {
                unreachable!("error");
            }
        },
        Some(("bank", _matches)) => match _matches.subcommand() {
            Some(("reservations", _matches)) => {
                println!("reservations:");
                game.account
                    .bank
                    .reservations()
                    .iter()
                    .for_each(|r| println!("{}", r));
            }
            Some(("empty", _matches)) => {
                println!("not yet implemented");
            }
            None => game
                .account
                .bank
                .content
                .read()
                .unwrap()
                .iter()
                .for_each(|i| println!("{}: {}", i.code, i.quantity)),
            _ => {
                unreachable!("error");
            }
        },
        Some(("char", char_matches)) => {
            let index = *char_matches.get_one::<i32>("i").unwrap_or(&0);
            unsafe { CHAR = game.account.get_character(index as usize) };
            if let Some(char) = unsafe { CHAR.clone() } {
                println!("character '{}' selected", char.name);
            } else {
                println!("character not found");
            }
        }
        Some(("idle", _m)) => {
            if let Some(char) = unsafe { CHAR.clone() } {
                char.toggle_idle();
            } else {
                println!("no character selected");
            }
        }
        Some(("craft", char_matches)) => {
            let item = char_matches
                .get_one::<String>("item")
                .map(|s| s.as_str())
                .unwrap_or("none");
            let quantity = char_matches.get_one::<i32>("quantity").unwrap_or(&1);
            if let Some(char) = unsafe { CHAR.clone() } {
                char.craft_items(item, *quantity);
            } else {
                println!("no character selected");
            }
        }
        Some(("recycle", char_matches)) => {
            let item = char_matches
                .get_one::<String>("item")
                .map(|s| s.as_str())
                .unwrap_or("none");
            let quantity = char_matches.get_one::<i32>("quantity").unwrap_or(&1);
            if let Some(char) = unsafe { CHAR.clone() } {
                char.recycle_item(item, *quantity)
                    .map_err(|e| e.to_string())?;
            } else {
                println!("no character selected");
            }
        }
        Some(("gear", gear_matches)) => {
            let monster = gear_matches
                .get_one::<String>("monster")
                .map(|s| s.as_str())
                .unwrap_or("none");
            if let Some(char) = unsafe { CHAR.clone() } {
                if let Some(monster) = game.monsters.get(monster) {
                    println!(
                        "{}",
                        gear_finder.best_against(&char, monster, Filter::Available)
                    );
                } else {
                    println!("monster not found");
                }
            } else {
                println!("no character selected");
            }
        }
        Some(("simulate", sim_matches)) => {
            let monster = sim_matches
                .get_one::<String>("monster")
                .map(|s| s.as_str())
                .unwrap_or("none");
            if let Some(char) = unsafe { CHAR.clone() } {
                if let Some(monster) = game.monsters.get(monster) {
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
        Some(("deposit", _matches)) => {
            println!("not yet implemented");
        }
        Some(("unequip", _matches)) => {
            println!("not yet implemented");
        }
        Some((cmd, _matches)) => {
            println!("unknown command: {}", cmd);
        }
        None => {
            unreachable!("error");
        }
    }
    Ok(true)
}
