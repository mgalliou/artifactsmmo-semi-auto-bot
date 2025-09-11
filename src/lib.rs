use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering::SeqCst},
};

use artifactsmmo_sdk::{
    ContainerSlot, HasQuantity, ItemContainer,
    consts::{
        APPLE, APPLE_PIE, CARROT, COOKED_HELLHOUND_MEAT, EGG, FISH_SOUP, MAPLE_SYRUP, MUSHROOM_SOUP,
    },
};

pub mod account;
pub mod bank;
pub mod bot;
pub mod bot_config;
pub mod character;
pub mod cli;
pub mod error;
pub mod gear_finder;
pub mod inventory;
pub mod leveling_helper;
pub mod orderboard;

pub const FOOD_BLACK_LIST: [&str; 9] = [
    APPLE,
    APPLE_PIE,
    EGG,
    CARROT,
    MUSHROOM_SOUP,
    FISH_SOUP,
    COOKED_HELLHOUND_MEAT,
    MAPLE_SYRUP,
    "corrupted_fruit",
];

pub const MIN_COIN_THRESHOLD: u32 = 4;
pub const MIN_FOOD_THRESHOLD: u32 = 6000;

trait HasReservation: ItemContainer {
    type Reservation: ContainerSlot + HasQuantity;

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
}

pub trait Reservation: HasQuantity {
    fn inc_quantity(&self, n: u32) {
        self.quantity_atomic().fetch_add(n, SeqCst);
    }

    fn dec_quantity(&self, n: u32) {
        let new = self.quantity().saturating_sub(n);
        self.quantity_atomic().store(new, SeqCst);
    }

    fn quantity_atomic(&self) -> &AtomicU32;
}
