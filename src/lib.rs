use artifactsmmo_sdk::consts::{
    APPLE, APPLE_PIE, CARROT, COOKED_HELLHOUND_MEAT, EGG, FISH_SOUP, MAPLE_SYRUP, MUSHROOM_SOUP,
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

pub const FOOD_BLACK_LIST: [&str; 8] = [
    APPLE,
    APPLE_PIE,
    EGG,
    CARROT,
    MUSHROOM_SOUP,
    FISH_SOUP,
    COOKED_HELLHOUND_MEAT,
    MAPLE_SYRUP,
];

pub const MIN_COIN_THRESHOLD: i32 = 4;
pub const MIN_FOOD_THRESHOLD: i32 = 1000;
