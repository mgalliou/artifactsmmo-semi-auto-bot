use crate::{client::character::error::RequestError, entities::RawMap};
use openapi::models::{
    CharacterFightSchema, EquipSchema, GeTransactionSchema, NpcItemTransactionSchema,
    RecyclingItemsSchema, RewardsSchema, SimpleItemSchema, SkillInfoSchema, TaskSchema,
    TaskTradeSchema, UnequipSchema,
};
use std::time::Duration;

/// Trait that abstracts the action-execution and cooldown/control layer
/// behind `CharacterClient`.
///
/// The sole production implementation is `CharacterHttpRequestHandler`.
/// Test code can provide a lightweight stub to exercise `CharacterClient`'s
/// validation logic without an HTTP client.
pub trait CharacterRequestHandler: Send + Sync {
    fn refresh_data(&self);
    fn pause(&self);
    fn resume(&self);
    fn cancel(&self);
    fn is_paused(&self) -> bool;
    fn remaining_cooldown(&self) -> Duration;

    fn request_move(&self, x: i32, y: i32) -> Result<RawMap, RequestError>;
    fn request_transition(&self) -> Result<RawMap, RequestError>;
    fn request_fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError>;
    fn request_rest(&self) -> Result<u32, RequestError>;
    fn request_gather(&self) -> Result<SkillInfoSchema, RequestError>;
    fn request_craft(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<SkillInfoSchema, RequestError>;
    fn request_delete(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<SimpleItemSchema, RequestError>;
    fn request_recycle(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<RecyclingItemsSchema, RequestError>;
    fn request_deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError>;
    fn request_withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError>;
    fn request_deposit_gold(&self, quantity: u32) -> Result<u32, RequestError>;
    fn request_withdraw_gold(&self, quantity: u32) -> Result<u32, RequestError>;
    fn request_expand_bank(&self) -> Result<u32, RequestError>;
    fn request_equip(&self, items: &[EquipSchema]) -> Result<(), RequestError>;
    fn request_unequip(&self, slots: &[UnequipSchema]) -> Result<(), RequestError>;
    fn request_use_item(&self, item_code: &str, quantity: u32) -> Result<(), RequestError>;
    fn request_accept_task(&self) -> Result<TaskSchema, RequestError>;
    fn request_complete_task(&self) -> Result<RewardsSchema, RequestError>;
    fn request_cancel_task(&self) -> Result<(), RequestError>;
    fn request_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<TaskTradeSchema, RequestError>;
    fn request_exchange_tasks_coin(&self) -> Result<RewardsSchema, RequestError>;
    fn request_npc_buy(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError>;
    fn request_npc_sell(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError>;
    fn request_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), RequestError>;
    fn request_give_gold(&self, quantity: u32, character: &str) -> Result<(), RequestError>;
    fn request_claim_pending_item(&self, id: &str) -> Result<(), RequestError>;
    fn request_ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, RequestError>;
    fn request_ge_create_order(
        &self,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<(), RequestError>;
    fn request_ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, RequestError>;
}
