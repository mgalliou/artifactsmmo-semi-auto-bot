use crate::client::{items::ItemsClient, monsters::MonstersClient};
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
