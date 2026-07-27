use itertools::Itertools;
use openapi::models::{
    BankSchema, CharacterFightSchema, CharacterSchema, EquipSchema, GeTransactionSchema,
    InventorySlotSchema, MapLayer, NpcItemTransactionSchema, RecyclingItemsSchema, RewardsSchema,
    SimpleItemSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema, UnequipSchema,
};

use crate::{
    AccountClient, CharacterClient, CollectionClient, EventsClient, GrandExchangeClient,
    MapsClient, NpcsClient, NpcsItemsClient, ResourcesClient, TasksClient, TasksRewardsClient,
    character::{CharacterRequestHandler, InventoryClient, error::RequestError},
    client::{bank::BankClient, items::ItemsClient, monsters::MonstersClient},
    entities::{CharacterHandle, Item, Monster, RawMap, Resource},
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Duration,
};

const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

pub static ITEMS: LazyLock<ItemsClient> = LazyLock::new(|| {
    let client = ItemsClient::new(
        PATH,
        Box::new(HashMap::new),
        RESOURCES.clone(),
        MONSTERS.clone(),
        TASKS_REWARDS.clone(),
        NPCS.clone(),
    );
    client.init();
    client
});

pub static EVENTS: LazyLock<EventsClient> = LazyLock::new(|| {
    let client = EventsClient::new(PATH, Box::new(HashMap::new), Box::new(Vec::new));
    client.init();
    client
});

pub static MAPS: LazyLock<MapsClient> = LazyLock::new(|| {
    let client = MapsClient::new(PATH, Box::new(HashMap::new), EVENTS.clone());
    client.init();
    client
});

pub static RESOURCES: LazyLock<ResourcesClient> = LazyLock::new(|| {
    let client = ResourcesClient::new(PATH, Box::new(HashMap::new), EVENTS.clone());
    client.init();
    client
});

pub static MONSTERS: LazyLock<MonstersClient> = LazyLock::new(|| {
    let client = MonstersClient::new(PATH, Box::new(HashMap::new), EVENTS.clone());
    client.init();
    client
});

pub static NPCS: LazyLock<NpcsClient> = LazyLock::new(|| {
    let client = NpcsClient::new(PATH, Box::new(HashMap::new), NPCS_ITEMS.clone());
    client.init();
    client
});

pub static NPCS_ITEMS: LazyLock<NpcsItemsClient> = LazyLock::new(|| {
    let client = NpcsItemsClient::new(PATH, Box::new(HashMap::new));
    client.init();
    client
});

pub static TASKS: LazyLock<TasksClient> = LazyLock::new(|| {
    let client = TasksClient::new(PATH, Box::new(HashMap::new), TASKS_REWARDS.clone());
    client.init();
    client
});

pub static TASKS_REWARDS: LazyLock<TasksRewardsClient> = LazyLock::new(|| {
    let client = TasksRewardsClient::new(PATH, Box::new(HashMap::new));
    client.init();
    client
});

pub fn item(code: &str) -> Item {
    ITEMS.get(code).unwrap()
}

pub fn monster(code: &str) -> Monster {
    MONSTERS.get(code).unwrap()
}

pub fn resource(code: &str) -> Resource {
    RESOURCES.get(code).unwrap()
}

pub static BANK: LazyLock<BankClient> = LazyLock::new(|| {
    BankClient::new(
        Box::new(|| panic!("test bank")),
        Box::new(|| panic!("test bank")),
    )
});

pub static ACCOUNT: LazyLock<AccountClient> = LazyLock::new(|| {
    AccountClient::new(
        "test_account".into(),
        BANK.clone(),
        Box::new(|_| panic!("test account")),
        Box::new(|_| panic!("test account")),
        Box::new(|| panic!("test account")),
        Box::new(|_, _, _, _| panic!("test account")),
    )
});

struct MockCharacterRequestHandler;

impl CharacterRequestHandler for MockCharacterRequestHandler {
    fn refresh_data(&self) {
        todo!()
    }

    fn pause(&self) {
        todo!()
    }

    fn resume(&self) {
        todo!()
    }

    fn cancel(&self) {
        todo!()
    }

    fn is_paused(&self) -> bool {
        todo!()
    }

    fn remaining_cooldown(&self) -> Duration {
        todo!()
    }

    fn request_move(&self, _x: i32, _y: i32) -> Result<RawMap, RequestError> {
        todo!()
    }

    fn request_transition(&self) -> Result<RawMap, RequestError> {
        todo!()
    }

    fn request_fight(
        &self,
        _participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError> {
        todo!()
    }

    fn request_rest(&self) -> Result<u32, RequestError> {
        todo!()
    }

    fn request_gather(&self) -> Result<SkillInfoSchema, RequestError> {
        todo!()
    }

    fn request_craft(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<SkillInfoSchema, RequestError> {
        todo!()
    }

    fn request_delete(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<SimpleItemSchema, RequestError> {
        todo!()
    }

    fn request_recycle(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<RecyclingItemsSchema, RequestError> {
        todo!()
    }

    fn request_deposit_item(&self, _items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        todo!()
    }

    fn request_withdraw_item(&self, _items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        todo!()
    }

    fn request_deposit_gold(&self, _quantity: u32) -> Result<u32, RequestError> {
        todo!()
    }

    fn request_withdraw_gold(&self, _quantity: u32) -> Result<u32, RequestError> {
        todo!()
    }

    fn request_expand_bank(&self) -> Result<u32, RequestError> {
        todo!()
    }

    fn request_equip(&self, _items: &[EquipSchema]) -> Result<(), RequestError> {
        todo!()
    }

    fn request_unequip(&self, _slots: &[UnequipSchema]) -> Result<(), RequestError> {
        todo!()
    }

    fn request_use_item(&self, _item_code: &str, _quantity: u32) -> Result<(), RequestError> {
        todo!()
    }

    fn request_accept_task(&self) -> Result<TaskSchema, RequestError> {
        todo!()
    }

    fn request_complete_task(&self) -> Result<RewardsSchema, RequestError> {
        todo!()
    }

    fn request_cancel_task(&self) -> Result<(), RequestError> {
        todo!()
    }

    fn request_trade_task_item(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<TaskTradeSchema, RequestError> {
        todo!()
    }

    fn request_exchange_tasks_coin(&self) -> Result<RewardsSchema, RequestError> {
        todo!()
    }

    fn request_npc_buy(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError> {
        todo!()
    }

    fn request_npc_sell(
        &self,
        _item_code: &str,
        _quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError> {
        todo!()
    }

    fn request_give_item(
        &self,
        _items: &[SimpleItemSchema],
        _character: &str,
    ) -> Result<(), RequestError> {
        todo!()
    }

    fn request_give_gold(&self, _quantity: u32, _character: &str) -> Result<(), RequestError> {
        todo!()
    }

    fn request_claim_pending_item(&self, _id: &str) -> Result<(), RequestError> {
        todo!()
    }

    fn request_ge_buy_order(
        &self,
        _id: &str,
        _quantity: u32,
    ) -> Result<GeTransactionSchema, RequestError> {
        todo!()
    }

    fn request_ge_create_order(
        &self,
        _item_code: &str,
        _quantity: u32,
        _price: u32,
    ) -> Result<(), RequestError> {
        todo!()
    }

    fn request_ge_cancel_order(&self, _id: &str) -> Result<GeTransactionSchema, RequestError> {
        todo!()
    }
}

pub fn character(schema: CharacterSchema) -> CharacterClient {
    let char = CharacterClient::new(
        1,
        CharacterHandle::new(schema),
        Arc::new(MockCharacterRequestHandler),
        ACCOUNT.clone(),
        ITEMS.clone(),
        RESOURCES.clone(),
        MONSTERS.clone(),
        MAPS.clone(),
        NPCS.clone(),
        TASKS.clone(),
        GrandExchangeClient::default(),
    );
    ACCOUNT.add_character(char.clone());
    char
}

#[must_use]
pub fn inventory_client(slots: Vec<InventorySlotSchema>, max_items: u32) -> InventoryClient {
    let schema = CharacterSchema {
        inventory: Some(slots),
        inventory_max_items: max_items as i32,
        ..default_schema()
    };
    let char = character(schema);
    char.inventory().clone()
}

#[allow(clippy::unnecessary_wraps)]
#[must_use]
pub fn empty_inventory() -> Option<Vec<InventorySlotSchema>> {
    Some(inventory_with(&[]))
}

#[must_use]
pub fn inventory_with(items: &[(&str, u32)]) -> Vec<InventorySlotSchema> {
    let mut result = items
        .iter()
        .enumerate()
        .map(|(i, &(code, qty))| InventorySlotSchema::new((i + 1) as i32, code.into(), qty as i32))
        .collect_vec();
    let next_id = result.len() as i32 + 1;
    result.extend((next_id..=20).map(|id| InventorySlotSchema::new(id, String::new(), 0)));
    result
}

#[must_use]
pub fn default_schema() -> CharacterSchema {
    CharacterSchema {
        x: 0,
        y: 0,
        layer: MapLayer::Overworld,
        inventory_max_items: 100,
        inventory: empty_inventory(),
        ..Default::default()
    }
}

#[must_use]
pub const fn empty_bank_details() -> BankSchema {
    BankSchema {
        slots: 100,
        expansions: 0,
        next_expansion_cost: 100,
        gold: 0,
    }
}
