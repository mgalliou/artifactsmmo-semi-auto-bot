use chrono::{DateTime, Utc};
use log::error;
use sdk::{
    Client, Code, ItemContainer, Quantity,
    consts::{
        APPLE, APPLE_PIE, CARROT, COOKED_HELLHOUND_MEAT, FISH_SOUP, MAPLE_SYRUP, MUSHROOM_SOUP,
    },
    entities::{Character, Monster, Resource},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc, RwLock,
        atomic::{AtomicU32, Ordering::SeqCst},
    },
    thread::{Builder, sleep},
    time::Duration,
};

use crate::{
    account::AccountController, bank::BankController, bot_config::BotConfig,
    gear_finder::GearFinder, leveling_helper::LevelingHelper, orderboard::OrderBoard,
};

pub mod account;
pub mod bank;
pub mod bot;
pub mod bot_config;
pub mod character;
pub mod error;
pub mod gear_finder;
pub mod inventory;
pub mod leveling_helper;
pub mod orderboard;

pub const FOOD_ORDER_BLACKLIST: [&str; 11] = [
    APPLE,
    APPLE_PIE,
    MUSHROOM_SOUP,
    "fried_eggs",
    "cooked_beef",
    "cooked_chicken",
    "cheese",
    FISH_SOUP,
    COOKED_HELLHOUND_MEAT,
    MAPLE_SYRUP,
    "corrupted_fruit",
];

pub const FOOD_CONSUMPTION_BLACKLIST: [&str; 2] = [APPLE, CARROT];

pub const MIN_COIN_THRESHOLD: u32 = 4;
pub const MIN_FOOD_THRESHOLD: u32 = 6000;

pub trait HasReservation: ItemContainer {
    type Reservation: Reservation;
    type Discriminant: Discriminant;

    fn reservations(&self) -> Vec<Arc<Self::Reservation>>;

    fn quantity_reserved(&self, item: &str) -> u32 {
        self.reservations()
            .iter()
            .filter_map(|r| (r.code() == item).then_some(r.quantity()))
            .sum()
    }

    fn quantity_reservable(&self, item: &str) -> u32 {
        self.total_of(item)
            .saturating_sub(self.quantity_reserved(item))
    }

    fn is_reserved(&self, item: &str) -> bool {
        self.quantity_reserved(item) > 0
    }

    fn get_reservation(&self, discriminant: Self::Discriminant) -> Option<Arc<Self::Reservation>> {
        self.reservations()
            .iter()
            .find(|r| Self::discriminate(r) == discriminant)
            .cloned()
    }

    fn discriminate(reservation: &Self::Reservation) -> Self::Discriminant;
}

pub trait Reservation: Code + Quantity {
    fn inc_quantity(&self, n: u32) {
        self.quantity_mut().fetch_add(n, SeqCst);
    }

    fn dec_quantity(&self, n: u32) {
        let new = self.quantity().saturating_sub(n);
        self.quantity_mut().store(new, SeqCst);
    }

    fn quantity_mut(&self) -> &AtomicU32;
}

#[derive(PartialEq, Eq)]
pub struct InventoryDiscriminant {
    item: String,
}

#[derive(PartialEq, Eq)]
pub struct BankDiscriminant {
    item: String,
    owner: String,
}

pub trait Discriminant: PartialEq {}

impl Discriminant for InventoryDiscriminant {}
impl Discriminant for BankDiscriminant {}

impl From<&str> for InventoryDiscriminant {
    fn from(value: &str) -> Self {
        Self {
            item: value.to_string(),
        }
    }
}

impl From<(&str, &str)> for BankDiscriminant {
    fn from(value: (&str, &str)) -> Self {
        Self {
            item: value.0.to_string(),
            owner: value.0.to_string(),
        }
    }
}

pub struct Bot {
    pub config: BotConfig,
    pub client: Client,
    pub order_board: OrderBoard,
    pub gear_finder: GearFinder,
    pub leveling_helper: LevelingHelper,
    pub account: AccountController,
    pub bank: BankController,
}

impl Bot {
    pub fn new(client: &Client) -> Self {
        let config = BotConfig::from_file();
        let bank = BankController::new(client.account.bank(), client.items.clone());
        let account = AccountController::new(
            config.clone(),
            client.account.clone(),
            client.items.clone(),
            client.npcs.clone(),
            bank.clone(),
        );
        Self {
            config,
            client: client.clone(),
            order_board: OrderBoard::new(client.items.clone(), account.clone()),
            gear_finder: GearFinder::new(client.items.clone(), account.clone()),
            leveling_helper: LevelingHelper::new(
                client.items.clone(),
                client.monsters.clone(),
                client.resources.clone(),
                client.maps.clone(),
                account.clone(),
                bank.clone(),
            ),
            account,
            bank,
        }
    }

    pub fn run(&self) {
        self.account.init_characters(
            &self.client,
            &self.account,
            &self.order_board,
            &self.gear_finder,
            &self.leveling_helper,
        );
        for char in self.account.characters() {
            sleep(Duration::from_millis(250));
            if let Err(e) = Builder::new().name(char.name().to_string()).spawn(move || {
                char.run_loop();
            }) {
                error!("failed to spawn character thread: {e}");
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum CharacterCommand {
    Craft { code: String, quantity: u32 },
    Kill { monster: Monster },
    Gather { resource: Resource },
    Recycle { code: String, quantity: u32 },
    Delete { code: String, quantity: u32 },
    BuyItem { code: String, quantity: u32 },
    SellItem { code: String, quantity: u32 },
}

#[derive(Clone)]
pub struct CommandWrapper {
    command: CharacterCommand,
    creation: DateTime<Utc>,
}

#[derive(Default)]
pub struct CommandQueue {
    commands: RwLock<VecDeque<CommandWrapper>>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            commands: RwLock::default(),
        }
    }

    pub fn push(&self, command: CharacterCommand) {
        let cmd = CommandWrapper {
            command,
            creation: Utc::now(),
        };
        self.commands.write().unwrap().push_back(cmd);
    }

    pub fn remove(&self, other: &CommandWrapper) {
        self.commands
            .write()
            .unwrap()
            .retain(|c| c.creation != other.creation && c.command == other.command);
    }

    pub fn commands(&self) -> Vec<CommandWrapper> {
        self.commands.read().unwrap().iter().cloned().collect()
    }
}
