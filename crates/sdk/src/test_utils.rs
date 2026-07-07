use crate::{
    CollectionClient,
    client::{items::ItemsClient, monsters::MonstersClient},
    entities::{Item, Monster},
};
use std::sync::LazyLock;

pub static ITEMS: LazyLock<ItemsClient> = LazyLock::new(|| {
    ItemsClient::from_cache(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/items.ron"
    ))
});

pub static MONSTERS: LazyLock<MonstersClient> = LazyLock::new(|| {
    MonstersClient::from_cache(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/monsters.ron"
    ))
});

pub fn item(code: &str) -> Item {
    ITEMS.get(code).unwrap()
}

pub fn monster(code: &str) -> Monster {
    MONSTERS.get(code).unwrap()
}
