use crate::entities::Character;
use crate::{
    AccountClient,
    bank::Bank,
    character::{CharacterHandle, responses::ResponseSchema},
    client::{
        character::{CharacterRequestHandler, action_request::ActionRequest, error::RequestError},
        server::ServerClient,
    },
    entities::RawMap,
};
use api::ArtifactApi;
use chrono::Utc;
use log::{debug, error, info, warn};
use openapi::models::{
    BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    CharacterFightResponseSchema, CharacterFightSchema, CharacterMovementResponseSchema,
    CharacterTransitionResponseSchema, ClaimPendingItemResponseSchema, DeleteItemResponseSchema,
    GeCreateOrderTransactionResponseSchema, GeTransactionResponseSchema, GeTransactionSchema,
    GiveGoldResponseSchema, GiveItemResponseSchema, NpcItemTransactionSchema,
    NpcMerchantTransactionResponseSchema, RecyclingItemsSchema, RecyclingResponseSchema,
    RewardDataResponseSchema, RewardsSchema, SimpleItemSchema, SkillInfoSchema,
    SkillResponseSchema, TaskResponseSchema, TaskSchema, TaskTradeResponseSchema, TaskTradeSchema,
    UnequipSchema,
};
use openapi::models::{CharacterRestResponseSchema, EquipSchema};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::sleep;
use std::time::Duration;

/// First layer of abstraction around the character API.
/// It is responsible for handling the character action requests response and errors
/// by updating character and bank data, and retrying requests in case of errors.
pub struct CharacterHttpRequestHandler {
    api: ArtifactApi,
    data: CharacterHandle,
    account: AccountClient,
    server: ServerClient,
    pause_state: Arc<PauseState>,
}

impl CharacterHttpRequestHandler {
    pub fn new(
        api: ArtifactApi,
        data: CharacterHandle,
        account: AccountClient,
        server: ServerClient,
    ) -> Self {
        Self {
            api,
            data,
            account,
            server,
            pause_state: PauseState::default().into(),
        }
    }

    fn request_action(
        &self,
        action: ActionRequest,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        if !self.wait_for_ready() {
            *self.pause_state.canceled.lock().unwrap() = false;
            return Err(RequestError::Canceled);
        }
        match action.send(&self.data.name(), &self.api) {
            Ok(res) => {
                self.update_data(&*res);
                Ok(res)
            }
            Err(e) => self.handle_request_error(action, e),
        }
    }

    fn update_data(&self, res: &(dyn ResponseSchema + 'static)) {
        info!("{res}");
        res.characters().into_iter().for_each(|c| {
            if let Some(char_client) = self.account.get_character(&c.name) {
                char_client.store(c.clone().into());
            }
        });
        if let Some(content) = res.bank_content() {
            self.account.bank().set_content(content.clone());
        }
        if let Some(gold) = res.bank_gold() {
            self.account.bank().set_gold(gold);
        }
        if let Some(extension_price) = res.extension_price() {
            self.account.bank().expand();
            self.account
                .bank()
                .set_gold(self.account.bank().gold() - extension_price);
        }
        if let Some(item) = res.claimed_pending_item() {
            self.account.update_pending_item(item.clone());
        }
    }

    fn handle_request_error(
        &self,
        action: ActionRequest,
        error: RequestError,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        error!(
            "{}: failed to request action '{action}': {error}",
            self.data.name(),
        );
        match error {
            RequestError::ResponseError(ref res) => {
                if res.error.code == 489 {
                    return self.request_action(action);
                }
                if res.error.code == 499 {
                    error!(
                        "{}: code 499 received, resyncronizing server time",
                        self.data.name()
                    );
                    self.server.update_offset();
                    return self.request_action(action);
                }
                if res.error.code == 500 || res.error.code == 520 {
                    error!(
                        "{}: unknown error ({}), retrying in 10 seconds.",
                        self.data.name(),
                        res.error.code
                    );
                    sleep(Duration::from_secs(10));
                    return self.request_action(action);
                }
            }
            RequestError::Reqwest(ref req) => {
                if req.is_timeout() {
                    error!("{}: request timed-out, retrying...", self.data.name());
                    return self.request_action(action);
                }
            }
            RequestError::Serde(_) | RequestError::Io(_) | RequestError::DowncastError => {
                warn!("{}: refreshing data", self.data.name());
                self.refresh_data();
            }
            RequestError::Canceled | RequestError::ReqwestMiddleware(_) => return Err(error),
        }
        Err(error)
    }

    fn wait_for_ready(&self) -> bool {
        self.warn_if_late();

        let cooldown = self.remaining_cooldown();
        debug!(
            "{}: cooling down for {}.{} seconds.",
            self.data.name(),
            cooldown.as_secs(),
            cooldown.subsec_millis()
        );

        let mut guard = self.pause_state.paused.lock().unwrap();
        while !self.pause_state.is_canceled() {
            if *guard {
                guard = self.pause_state.cv.wait(guard).unwrap();
            } else if !self.remaining_cooldown().is_zero() {
                guard = self
                    .pause_state
                    .cv
                    .wait_timeout(guard, self.remaining_cooldown())
                    .unwrap()
                    .0;
            } else {
                break;
            }
        }
        drop(guard);
        !self.pause_state.is_canceled()
    }

    fn warn_if_late(&self) {
        let Some(expiration) = self.data.cooldown_expiration() else {
            return;
        };
        let late = Utc::now().fixed_offset() - expiration;
        if late.num_seconds() > 1 {
            warn!("{}: is late by {}s", self.data.name(), late.num_seconds());
        }
    }

    /// Returns the remaining cooldown duration.
    pub fn remaining_cooldown(&self) -> Duration {
        let Some(expiration_time) = self.data.cooldown_expiration() else {
            return Duration::ZERO;
        };
        (expiration_time.to_utc() - self.server.synced_time())
            .to_std()
            .unwrap_or_default()
    }
}

impl CharacterRequestHandler for CharacterHttpRequestHandler {
    fn pause(&self) {
        info!("{}: paused", self.data.name());
        self.pause_state.pause();
    }

    fn resume(&self) {
        info!("{}: resumed", self.data.name());
        self.pause_state.resume();
    }

    fn cancel(&self) {
        info!("{}: request canceled", self.data.name());
        self.pause_state.cancel();
    }

    fn is_paused(&self) -> bool {
        self.pause_state.is_paused()
    }

    fn remaining_cooldown(&self) -> Duration {
        self.remaining_cooldown()
    }

    fn request_move(&self, x: i32, y: i32) -> Result<RawMap, RequestError> {
        self.request_action(ActionRequest::Move { x, y })
            .and_then(downcast_response::<CharacterMovementResponseSchema>)
            .map(|s| RawMap::from(*s.data.destination))
    }

    fn request_transition(&self) -> Result<RawMap, RequestError> {
        self.request_action(ActionRequest::Transition)
            .and_then(downcast_response::<CharacterTransitionResponseSchema>)
            .map(|s| RawMap::from(*s.data.destination))
    }

    fn request_fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError> {
        self.request_action(ActionRequest::Fight { participants })
            .and_then(downcast_response::<CharacterFightResponseSchema>)
            .map(|s| *s.data.fight)
    }

    fn request_rest(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::Rest)
            .and_then(downcast_response::<CharacterRestResponseSchema>)
            .map(|s| s.data.hp_restored as u32)
    }

    fn request_gather(&self) -> Result<SkillInfoSchema, RequestError> {
        self.request_action(ActionRequest::Gather)
            .and_then(downcast_response::<SkillResponseSchema>)
            .map(|s| *s.data.details)
    }

    fn request_craft(
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

    fn request_delete(
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

    fn request_recycle(
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

    fn request_deposit_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::DepositItem { items })
            .map(|_| ())
    }

    fn request_withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::WithdrawItem { items })
            .map(|_| ())
    }

    fn request_deposit_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::DepositGold { quantity })
            .and_then(downcast_response::<BankGoldTransactionResponseSchema>)
            .map(|r| r.data.bank.quantity)
    }

    fn request_withdraw_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::WithdrawGold { quantity })
            .and_then(downcast_response::<BankGoldTransactionResponseSchema>)
            .map(|r| r.data.bank.quantity)
    }

    fn request_expand_bank(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::ExpandBank)
            .and_then(downcast_response::<BankExtensionTransactionResponseSchema>)
            .map(|r| r.data.transaction.price)
    }

    fn request_equip(&self, items: &[EquipSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::Equip { items })
            .map(|_| ())
    }

    fn request_unequip(&self, slots: &[UnequipSchema]) -> Result<(), RequestError> {
        self.request_action(ActionRequest::Unequip { slots })
            .map(|_| ())
    }

    fn request_use_item(&self, item_code: &str, quantity: u32) -> Result<(), RequestError> {
        self.request_action(ActionRequest::UseItem {
            item_code,
            quantity,
        })
        .map(|_| ())
    }

    fn request_accept_task(&self) -> Result<TaskSchema, RequestError> {
        self.request_action(ActionRequest::AcceptTask)
            .and_then(downcast_response::<TaskResponseSchema>)
            .map(|r| *r.data.task)
    }

    fn request_complete_task(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::CompleteTask)
            .and_then(downcast_response::<RewardDataResponseSchema>)
            .map(|s| *s.data.rewards)
    }

    fn request_cancel_task(&self) -> Result<(), RequestError> {
        self.request_action(ActionRequest::CancelTask).map(|_| ())
    }

    fn request_trade_task_item(
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

    fn request_exchange_tasks_coin(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::ExchangeTasksCoins)
            .and_then(downcast_response::<RewardDataResponseSchema>)
            .map(|r| *r.data.rewards)
    }

    fn request_npc_buy(
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

    fn request_npc_sell(
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

    fn request_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveItem { items, character })
            .and_then(downcast_response::<GiveItemResponseSchema>)
            .map(|_| ())
    }

    fn request_give_gold(&self, quantity: u32, character: &str) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveGold {
            quantity,
            character,
        })
        .and_then(downcast_response::<GiveGoldResponseSchema>)
        .map(|_| ())
    }

    fn request_claim_pending_item(&self, id: &str) -> Result<(), RequestError> {
        self.request_action(ActionRequest::ClaimPendingItem { id })
            .and_then(downcast_response::<ClaimPendingItemResponseSchema>)
            .map(|_| ())
    }

    fn request_ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeBuyOrder { id, quantity })
            .and_then(downcast_response::<GeTransactionResponseSchema>)
            .map(|r| *r.data.order)
    }

    fn request_ge_create_order(
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

    fn request_ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeCancelOrder { id })
            .and_then(downcast_response::<GeTransactionResponseSchema>)
            .map(|r| *r.data.order)
    }

    fn refresh_data(&self) {
        let Ok(res) = self.api.character.get(&self.data.name()) else {
            return;
        };
        self.data.store((*res.data).into());
    }
}

#[derive(Default, Debug)]
struct PauseState {
    paused: Mutex<bool>,
    canceled: Mutex<bool>,
    cv: Condvar,
}

impl PauseState {
    fn pause(&self) {
        *self.paused.lock().unwrap() = true;
    }

    fn resume(&self) {
        *self.paused.lock().unwrap() = false;
        self.cv.notify_all();
    }

    fn cancel(&self) {
        *self.canceled.lock().unwrap() = true;
        self.cv.notify_all();
    }

    fn is_paused(&self) -> bool {
        *self.paused.lock().unwrap()
    }

    fn is_canceled(&self) -> bool {
        *self.canceled.lock().unwrap()
    }
}

fn downcast_response<T: ResponseSchema + 'static>(
    r: Box<dyn ResponseSchema>,
) -> Result<T, RequestError> {
    r.downcast()
        .map(|b| *b)
        .map_err(|_| RequestError::DowncastError)
}
