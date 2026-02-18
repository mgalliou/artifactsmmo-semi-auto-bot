use crate::{
    AccountClient,
    character::{CharacterDataHandle, MeetsConditionsFor, responses::ResponseSchema},
    client::{
        character::{HasCharacterData, action_request::ActionRequest, error::RequestError},
        server::ServerClient,
    },
    entities::{Character, Map},
    gear::Slot,
};
use api::ArtifactApi;
use chrono::Utc;
use log::{debug, error, info, warn};
use openapi::models::{
    BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    BankItemTransactionResponseSchema, CharacterFightResponseSchema, CharacterFightSchema,
    CharacterMovementResponseSchema, CharacterRestResponseSchema, CharacterSchema,
    CharacterTransitionResponseSchema, DeleteItemResponseSchema,
    GeCreateOrderTransactionResponseSchema, GeTransactionResponseSchema, GeTransactionSchema,
    GiveGoldResponseSchema, GiveItemResponseSchema, NpcItemTransactionSchema,
    NpcMerchantTransactionResponseSchema, RecyclingItemsSchema, RecyclingResponseSchema,
    RewardDataResponseSchema, RewardsSchema, SimpleItemSchema, SkillDataSchema, SkillInfoSchema,
    SkillResponseSchema, TaskResponseSchema, TaskSchema, TaskTradeResponseSchema, TaskTradeSchema,
};
use std::{cmp::Ordering, sync::Arc, thread::sleep, time::Duration};

/// First layer of abstraction around the character API.
/// It is responsible for handling the character action requests responce and errors
/// by updating character and bank data, and retrying requests in case of errors.
#[derive(Default, Debug)]
pub(crate) struct CharacterRequestHandler {
    api: Arc<ArtifactApi>,
    account: AccountClient,
    data: CharacterDataHandle,
    server: Arc<ServerClient>,
}

impl CharacterRequestHandler {
    pub fn new(
        api: Arc<ArtifactApi>,
        data: CharacterDataHandle,
        account: AccountClient,
        server: Arc<ServerClient>,
    ) -> Self {
        Self {
            api,
            data,
            account,
            server,
        }
    }

    pub fn character(&self) -> Character {
        self.data.read()
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
                        if let Some(char_client) = self.account.get_character_by_name(&c.name) {
                            char_client.update_data(c.clone());
                        }
                    });
                } else {
                    self.update_data(res.character().clone());
                }
                if let Some(res) = res.downcast_ref::<BankItemTransactionResponseSchema>() {
                    self.account().bank().update_content(res.data.bank.clone());
                } else if let Some(res) = res.downcast_ref::<BankGoldTransactionResponseSchema>() {
                    self.account().bank().update_gold(res.data.bank.quantity);
                } else if res
                    .downcast_ref::<BankExtensionTransactionResponseSchema>()
                    .is_some()
                {
                    self.account().bank().expand();
                };
                if let Some(res) = res.downcast_ref::<GiveItemResponseSchema>()
                    && let Some(char) = self
                        .account
                        .get_character_by_name(&res.data.receiver_character.name)
                {
                    char.update_data(*res.data.receiver_character.clone());
                }
                if let Some(res) = res.downcast_ref::<GiveGoldResponseSchema>()
                    && let Some(char) = self
                        .account
                        .get_character_by_name(&res.data.receiver_character.name)
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
            "{}: failed to request action '{}': {}",
            self.name(),
            action,
            error
        );
        match error {
            RequestError::ResponseError(ref res) => {
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
                self.refresh_data()
            }
        }
        Err(error)
    }

    fn wait_for_cooldown(&self) {
        if let Some(expiration) = self.cooldown_expiration() {
            let late = Utc::now() - expiration;
            if late.num_seconds() > 1 {
                warn!("{}: is late by {}s", self.name(), late.num_seconds())
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
        if let Some(exp) = self.cooldown_expiration() {
            let synced = Utc::now() - *self.server.server_offset.read().unwrap();
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    pub fn request_move(&self, x: i32, y: i32) -> Result<Map, RequestError> {
        self.request_action(ActionRequest::Move { x, y })
            .and_then(|r| {
                r.downcast::<CharacterMovementResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| Map::new(*s.data.destination))
    }

    pub fn request_transition(&self) -> Result<Map, RequestError> {
        self.request_action(ActionRequest::Transition)
            .and_then(|r| {
                r.downcast::<CharacterTransitionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| Map::new(*s.data.destination))
    }

    pub fn request_fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError> {
        self.request_action(ActionRequest::Fight { participants })
            .and_then(|r| {
                r.downcast::<CharacterFightResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.fight)
    }

    pub fn request_rest(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::Rest)
            .and_then(|r| {
                r.downcast::<CharacterRestResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.hp_restored as u32)
    }

    pub fn request_gather(&self) -> Result<SkillDataSchema, RequestError> {
        self.request_action(ActionRequest::Gather)
            .and_then(|r| {
                r.downcast::<SkillResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data)
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
        .and_then(|r| {
            r.downcast::<SkillResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
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
        .and_then(|resp| {
            resp.downcast::<DeleteItemResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|resp| *resp.data.item)
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
        .and_then(|r| {
            r.downcast::<RecyclingResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
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
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_withdraw_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::WithdrawGold { quantity })
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_expand_bank(&self) -> Result<u32, RequestError> {
        self.request_action(ActionRequest::ExpandBank)
            .and_then(|r| {
                r.downcast::<BankExtensionTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
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
            .and_then(|r| {
                r.downcast::<TaskResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| *r.data.task)
    }

    pub fn request_complete_task(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::CompleteTask)
            .and_then(|r| {
                r.downcast::<RewardDataResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
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
        .and_then(|r| {
            r.downcast::<TaskTradeResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|r| *r.data.trade)
    }

    pub fn request_exchange_tasks_coin(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(ActionRequest::ExchangeTasksCoins)
            .and_then(|r| {
                r.downcast::<RewardDataResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
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
        .and_then(|r| {
            r.downcast::<NpcMerchantTransactionResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
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
        .and_then(|r| {
            r.downcast::<NpcMerchantTransactionResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|r| *r.data.transaction)
    }

    pub fn request_give_item(
        &self,
        items: &[SimpleItemSchema],
        character: &str,
    ) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveItem { items, character })
            .and_then(|r| {
                r.downcast::<GiveItemResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|_| ())
    }

    pub fn request_give_gold(&self, quantity: u32, character: &str) -> Result<(), RequestError> {
        self.request_action(ActionRequest::GiveGold {
            quantity,
            character,
        })
        .and_then(|r| {
            r.downcast::<GiveGoldResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|_| ())
    }

    pub fn request_ge_buy_order(
        &self,
        id: &str,
        quantity: u32,
    ) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeBuyOrder { id, quantity })
            .and_then(|r| {
                r.downcast::<GeTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
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
        .and_then(|r| {
            r.downcast::<GeCreateOrderTransactionResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|_| ())
    }

    pub fn request_ge_cancel_order(&self, id: &str) -> Result<GeTransactionSchema, RequestError> {
        self.request_action(ActionRequest::GeCancelOrder { id })
            .and_then(|r| {
                r.downcast::<GeTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| *r.data.order)
    }
}

impl MeetsConditionsFor for CharacterRequestHandler {
    fn account(&self) -> AccountClient {
        self.account.clone()
    }
}

impl HasCharacterData for CharacterRequestHandler {
    fn data(&self) -> Character {
        self.data.read()
    }

    fn refresh_data(&self) {
        let Ok(res) = self.api.character.get(&self.name()) else {
            return;
        };
        self.update_data(*res.data)
    }

    fn update_data(&self, schema: CharacterSchema) {
        self.data.update(Character::new(schema));
    }
}
