#![cfg(test)]
use crate::client::items::ItemsClient;
use crate::client::monsters::MonstersClient;
use std::sync::LazyLock;

pub static ITEMS: LazyLock<ItemsClient> =
    LazyLock::new(|| ItemsClient::from_cache("tests/fixtures/items.json"));

pub static MONSTERS: LazyLock<MonstersClient> =
    LazyLock::new(|| MonstersClient::from_cache("tests/fixtures/monsters.json"));
