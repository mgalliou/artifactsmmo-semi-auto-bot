use crate::{
    AccountClient, Level, Skill,
    character::{CharacterDataHandle, responses::ResponseSchema},
    client::{
        character::{HandleCharacterData, action_request::ActionRequest, error::RequestError},
        server::ServerClient,
    },
    entities::{Character, Map, RawCharacter},
    gear::Slot,
};
use api::ArtifactApi;
use chrono::{DateTime, Utc};
use log::{debug, error, info, warn};
use openapi::models::{
    BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    BankItemTransactionResponseSchema, CharacterFightResponseSchema, CharacterFightSchema,
    CharacterMovementResponseSchema, CharacterRestResponseSchema, CharacterSchema,
    CharacterTransitionResponseSchema, DeleteItemResponseSchema,
    GeCreateOrderTransactionResponseSchema, GeTransactionResponseSchema, GeTransactionSchema,
    GiveGoldResponseSchema, GiveItemResponseSchema, InventorySlot, MapLayer,
    NpcItemTransactionSchema, NpcMerchantTransactionResponseSchema, RecyclingItemsSchema,
    RecyclingResponseSchema, RewardDataResponseSchema, RewardsSchema, SimpleItemSchema,
    SkillInfoSchema, SkillResponseSchema, TaskResponseSchema, TaskSchema, TaskTradeResponseSchema,
    TaskTradeSchema, TaskType,
};
use std::{sync::Arc, thread::sleep, time::Duration};

/// First layer of abstraction around the character API.
/// It is responsible for handling the character action requests responce and errors
/// by updating character and bank data, and retrying requests in case of errors.
#[derive(Default, Debug)]
pub struct CharacterRequestHandler {
    api: ArtifactApi,
    data: CharacterDataHandle,
    account: AccountClient,
    server: ServerClient,
}

fn downcast_response<T: ResponseSchema + 'static>(
    r: Box<dyn ResponseSchema>,
) -> Result<T, RequestError> {
    r.downcast()
        .map(|b| *b)
        .map_err(|_| RequestError::DowncastError)
}

impl CharacterRequestHandler {
    pub const fn new(
        api: ArtifactApi,
        data: CharacterDataHandle,
        account: AccountClient,
        server: ServerClient,
    ) -> Self {
        Self {
            api,
            data,
            account,
            server,
        }
    }

    fn request_action(
        &self,
        action: ActionRequest,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        self.wait_for_cooldown();
        // if action.is_deposit_item() || action.is_withdraw_item() {
        //     bank_content = Some(
        //         self.account.bank()
        //             .content()
        //             .write()
        //             .expect("bank_content to be writable"),
        //     );
        // }
        // if action.is_deposit_gold() || action.is_withdraw_gold() || action.is_expand_bank() {
        //     bank_details = Some(
        //         self.account
        //             .bank()
        //             .details
        //             .write()
        //             .expect("bank_details to be writable"),
        //     );
        // }
        match action.send(&self.name(), &self.api) {
            Ok(res) => {
                info!("{}", res.to_string());
                if let Some(res) = res.downcast_ref::<CharacterFightResponseSchema>() {
                    res.data.characters.iter().for_each(|c| {
                        if let Some(char_client) = self.account.get_character(&c.name) {
                            char_client.update_data(c.clone());
                        }
                    });
                } else {
                    self.update_data(res.character().clone());
                }
                if let Some(res) = res.downcast_ref::<BankItemTransactionResponseSchema>() {
                    self.account.bank().update_content(res.data.bank.clone());
                } else if let Some(res) = res.downcast_ref::<BankGoldTransactionResponseSchema>() {
                    self.account.bank().update_gold(res.data.bank.quantity);
                } else if res
                    .downcast_ref::<BankExtensionTransactionResponseSchema>()
                    .is_some()
                {
                    self.account.bank().expand();
                }
                if let Some(res) = res.downcast_ref::<GiveItemResponseSchema>()
                    && let Some(char) = self
                        .account
                        .get_character(&res.data.receiver_character.name)
                {
                    char.update_data(*res.data.receiver_character.clone());
                }
                if let Some(res) = res.downcast_ref::<GiveGoldResponseSchema>()
                    && let Some(char) = self
                        .account
                        .get_character(&res.data.receiver_character.name)
                {
                    char.update_data(*res.data.receiver_character.clone());
                }
                Ok(res)
            }
            Err(e) => {
                // drop(bank_content);
                // drop(bank_details);
                self.handle_request_error(action, e)
            }
        }
    }

    fn handle_request_error(
        &self,
        action: ActionRequest,
        error: RequestError,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        error!(
            "{}: failed to request action '{action}': {error}",
            self.name(),
        );
        match error {
            RequestError::ResponseError(ref res) => {
                if res.error.code == 489 {
                    return self.request_action(action);
                }
                if res.error.code == 499 {
                    error!(
                        "{}: code 499 received, resyncronizing server time",
                        self.name()
                    );
                    self.server.update_offset();
                    return self.request_action(action);
                }
                if res.error.code == 500 || res.error.code == 520 {
                    error!(
                        "{}: unknown error ({}), retrying in 10 secondes.",
                        self.name(),
                        res.error.code
                    );
                    sleep(Duration::from_secs(10));
                    return self.request_action(action);
                }
            }
            RequestError::Reqwest(ref req) => {
                if req.is_timeout() {
                    error!("{}: request timed-out, retrying...", self.name());
                    return self.request_action(action);
                }
            }
            RequestError::Serde(_) | RequestError::Io(_) | RequestError::DowncastError => {
                warn!("{}: refreshing data", self.name());
                self.refresh_data();
            }
        }
        Err(error)
    }

    fn wait_for_cooldown(&self) {
        if let Some(expiration) = self.cooldown_expiration() {
            let late = Utc::now() - expiration;
            if late.num_seconds() > 1 {
                warn!("{}: is late by {}s", self.name(), late.num_seconds());
            }
        }
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        debug!(
            "{}: cooling down for {}.{} secondes.",
            self.name(),
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    pub fn remaining_cooldown(&self) -> Duration {
        let Some(expiration_time) = self.cooldown_expiration() else {
            return Duration::default();
        };
        let current_time = Utc::now() - self.server.time_offset();
        (expiration_time.to_utc() - current_time)
            .to_std()
            .unwrap_or_default()
    }

    pub fn request_move(&self, x: i32, y: i32) -> Result<Map, RequestError> {
        self.request_action(ActionRequest::Move { x, y })
            .and_then(downcast_response::<CharacterMovementResponseSchema>)
            .map(|s| Map::from(*s.data.destination))
    }

    pub fn request_transition(&self) -> Result<Map, RequestError> {
        self.request_action(ActionRequest::Transition)
            .and_then(downcast_response::<CharacterTransitionResponseSchema>)
            .map(|s| Map::from(*s.data.destination))
    }

    pub fn request_fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError> {
        self.request_action(ActionRequest::Fight { participants })
            .and_then(downcast_response::<CharacterFightResponseSchema>)
            .map(|s| *s.data.fight)
    }

    pub fn request_rest(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::Rest)
            .and_then(downcast_response::<CharacterRestResponseSchema>)
            .map(|s| s.data.hp_restored as u32)
    }

    pub fn request_gather(&self) -> Result<SkillInfoSchema, RequestError> {
        self.request_action(ActionRequest::Gather)
            .and_then(downcast_response::<SkillResponseSchema>)
            .map(|s| *s.data.details)
    }

    pub fn request_craft(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<SkillInfoSchema, RequestError> {
        self.request_action(ActionRequest::Craft {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<SkillResponseSchema>)
        .map(|s| *s.data.details)
    }

    pub fn request_delete(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.request_action(ActionRequest::Delete {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<DeleteItemResponseSchema>)
        .map(|r| *r.data.item)
    }

    pub fn request_recycle(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<RecyclingItemsSchema, RequestError> {
        self.request_action(ActionRequest::Recycle {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<RecyclingResponseSchema>)
        .map(|r| *r.data.details)
    }

    pub fn request_deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::DepositItem { items })
            .map(|_| ())
    }

    pub fn request_withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::WithdrawItem { items })
            .map(|_| ())
    }

    pub fn request_deposit_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::DepositGold { quantity })
            .and_then(downcast_response::<BankGoldTransactionResponseSchema>)
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_withdraw_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::WithdrawGold { quantity })
            .and_then(downcast_response::<BankGoldTransactionResponseSchema>)
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_expand_bank(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::ExpandBank)
            .and_then(downcast_response::<BankExtensionTransactionResponseSchema>)
            .map(|r| r.data.transaction.price)
    }

    pub fn request_equip(
        &self,
        item_code: &str,
        slot: Slot,
        quantity: u32,
    ) -> Result<(), RequestError> {
        self.request_action(ActionRequest::Equip {
            item_code,
            slot,
            quantity,
        })
        .map(|_| ())
    }

    pub fn request_unequip(&self, slot: Slot, quantity: u32) -> Result<(), RequestError> {
        self.request_action(ActionRequest::Unequip { slot, quantity })
            .map(|_| ())
    }

    pub fn request_use_item(&self, item_code: &str, quantity: u32) -> Result<(), RequestError> {
        self.request_action(ActionRequest::UseItem {
            item_code,
            quantity,
        })
        .map(|_| ())
    }

    pub fn request_accept_task(&self) -> Result<TaskSchema, RequestError> {
        self.request_action(ActionRequest::AcceptTask)
            .and_then(downcast_response::<TaskResponseSchema>)
            .map(|r| *r.data.task)
    }

    pub fn request_complete_task(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::CompleteTask)
            .and_then(downcast_response::<RewardDataResponseSchema>)
            .map(|s| *s.data.rewards)
    }

    pub fn request_cancel_task(&self) -> Result<(), RequestError> {
        self.request_action(ActionRequest::CancelTask).map(|_| ())
    }

    pub fn request_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<TaskTradeSchema, RequestError> {
        self.request_action(ActionRequest::TradeTaskItem {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<TaskTradeResponseSchema>)
        .map(|r| *r.data.trade)
    }

    pub fn request_exchange_tasks_coin(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::ExchangeTasksCoins)
            .and_then(downcast_response::<RewardDataResponseSchema>)
            .map(|r| *r.data.rewards)
    }

    pub fn request_npc_buy(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError> {
        self.request_action(ActionRequest::NpcBuy {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<NpcMerchantTransactionResponseSchema>)
        .map(|r| *r.data.transaction)
    }

    pub fn request_npc_sell(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<NpcItemTransactionSchema, RequestError> {
        self.request_action(ActionRequest::NpcSell {
            item_code,
            quantity,
        })
        .and_then(downcast_response::<NpcMerchantTransactionResponseSchema>)
        .map(|r| *r.data.transaction)
    }

    pub fn request_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveItem { items, character })
            .and_then(downcast_response::<GiveItemResponseSchema>)
            .map(|_| ())
    }

    pub fn request_give_gold(&self, quantity: u32, character: &str) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveGold {
            quantity,
            character,
        })
        .and_then(downcast_response::<GiveGoldResponseSchema>)
        .map(|_| ())
    }

    pub fn request_ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeBuyOrder { id, quantity })
            .and_then(downcast_response::<GeTransactionResponseSchema>)
            .map(|r| *r.data.order)
    }

    pub fn request_ge_create_order(
        &self,
        item_code: &str,
        quantity: u32,
        price: u32,
    ) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GeCreateOrder {
            item_code,
            quantity,
            price,
        })
        .and_then(downcast_response::<GeCreateOrderTransactionResponseSchema>)
        .map(|_| ())
    }

    pub fn request_ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeCancelOrder { id })
            .and_then(downcast_response::<GeTransactionResponseSchema>)
            .map(|r| *r.data.order)
    }
}

impl HandleCharacterData for CharacterRequestHandler {
    fn data(&self) -> RawCharacter {
        self.data.read()
    }

    fn refresh_data(&self) {
        let Ok(res) = self.api.character.get(&self.name()) else {
            return;
        };
        self.data.update(RawCharacter::from(*res.data));
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.data.update(RawCharacter::from(schema));
    }
}

impl Level for CharacterRequestHandler {
    fn level(&self) -> u32 {
        self.data().level()
    }
}

impl Character for CharacterRequestHandler {
    fn name(&self) -> Arc<str> {
        self.data().name()
    }

    fn position(&self) -> (MapLayer, i32, i32) {
        self.data().position()
    }

    fn skill_level(&self, skill: Skill) -> u32 {
        self.data().skill_level(skill)
    }

    fn skill_xp(&self, skill: Skill) -> i32 {
        self.data().skill_xp(skill)
    }

    fn skill_max_xp(&self, skill: Skill) -> i32 {
        self.data().skill_max_xp(skill)
    }

    fn hp(&self) -> i32 {
        self.data().hp()
    }

    fn max_hp(&self) -> i32 {
        self.data().max_hp()
    }

    fn missing_hp(&self) -> i32 {
        self.data().missing_hp()
    }

    fn task(&self) -> Arc<str> {
        self.data().task()
    }

    fn task_type(&self) -> Option<TaskType> {
        self.data().task_type()
    }

    fn task_progress(&self) -> u32 {
        self.data().task_progress()
    }

    fn task_total(&self) -> u32 {
        self.data().task_total()
    }

    fn task_missing(&self) -> u32 {
        self.data().task_missing()
    }

    fn task_finished(&self) -> bool {
        self.data().task_finished()
    }

    fn inventory_items(&self) -> Arc<Option<Vec<InventorySlot>>> {
        self.data().inventory_items()
    }

    fn inventory_max_items(&self) -> i32 {
        self.data().inventory_max_items()
    }

    fn gold(&self) -> u32 {
        self.data().gold()
    }

    fn equiped_in(&self, slot: Slot) -> String {
        self.data().equiped_in(slot)
    }

    fn has_equiped(&self, item_code: &str) -> u32 {
        self.data().has_equiped(item_code)
    }

    fn quantity_in_slot(&self, slot: Slot) -> u32 {
        self.data().quantity_in_slot(slot)
    }

    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data().cooldown_expiration()
    }
}
