use openapi::models::{
    CharacterFightSchema, CharacterSchema, EquipSchema, GeTransactionSchema,
    NpcItemTransactionSchema, RecyclingItemsSchema, RewardsSchema, SimpleItemSchema,
    SkillInfoSchema, TaskSchema, TaskTradeSchema, UnequipSchema,
};

use crate::{
    AccountClient, CharacterClient, CollectionClient, EventsClient, GrandExchangeClient, MapsClient, NpcsClient, NpcsItemsClient, ResourcesClient, TasksClient, TasksRewardsClient, character::{CharacterRequestHandler, error::RequestError}, client::{items::ItemsClient, monsters::MonstersClient}, entities::{CharacterHandle, Item, Monster},
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
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

pub static MAPS: LazyLock<MapsClient> = LazyLock::new(|| {
    let client = MapsClient::new(PATH, Box::new(HashMap::new), EventsClient::default());
    client.init();
    client
});

pub static RESOURCES: LazyLock<ResourcesClient> = LazyLock::new(|| {
    let client = ResourcesClient::new(PATH, Box::new(HashMap::new), EventsClient::default());
    client.init();
    client
});

pub static MONSTERS: LazyLock<MonstersClient> = LazyLock::new(|| {
    let client = MonstersClient::new(PATH, Box::new(HashMap::new), EventsClient::default());
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

    fn remaining_cooldown(&self) -> std::time::Duration {
        todo!()
    }

    fn request_move(&self, _x: i32, _y: i32) -> Result<crate::entities::RawMap, RequestError> {
        todo!()
    }

    fn request_transition(&self) -> Result<crate::entities::RawMap, RequestError> {
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
    CharacterClient::new(
        1,
        CharacterHandle::new(schema),
        Arc::new(MockCharacterRequestHandler),
        AccountClient::default(),
        ITEMS.clone(),
        RESOURCES.clone(),
        MONSTERS.clone(),
        MAPS.clone(),
        NPCS.clone(),
        TASKS.clone(),
        GrandExchangeClient::default(),
    )
}
