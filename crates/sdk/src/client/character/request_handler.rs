use super::CharacterData;
use crate::{
    AccountClient, DropSchemas, SimpleItemSchemas,
    client::{
        bank::BankClient,
        character::{HasCharacterData, action::Action, error::RequestError},
        server::ServerClient,
    },
    consts::BANK_EXTENSION_SIZE,
    entities::Map,
    gear::Slot,
};
use api::ArtifactApi;
use openapi::models::{
    ActionType, BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    BankItemTransactionResponseSchema, BankSchema, CharacterFightResponseSchema,
    CharacterFightSchema, CharacterMovementResponseSchema, CharacterRestResponseSchema,
    CharacterSchema, CharacterTransitionResponseSchema, DeleteItemResponseSchema,
    EquipmentResponseSchema, FightResult, GeCreateOrderTransactionResponseSchema,
    GeTransactionResponseSchema, GeTransactionSchema, GiveGoldResponseSchema,
    GiveItemResponseSchema, MapSchema, NpcItemTransactionSchema,
    NpcMerchantTransactionResponseSchema, RecyclingItemsSchema, RecyclingResponseSchema,
    RewardDataResponseSchema, RewardsSchema, SimpleItemSchema, SkillDataSchema, SkillInfoSchema,
    SkillResponseSchema, TaskCancelledResponseSchema, TaskResponseSchema, TaskSchema,
    TaskTradeResponseSchema, TaskTradeSchema, UseItemResponseSchema,
};
use chrono::Utc;
use downcast_rs::{Downcast, impl_downcast};
use itertools::Itertools;
use log::{debug, error, info, warn};
use std::{
    cmp::Ordering,
    sync::{Arc, RwLockWriteGuard},
    thread::sleep,
    time::Duration,
};

/// First layer of abstraction around the character API.
/// It is responsible for handling the character action requests responce and errors
/// by updating character and bank data, and retrying requests in case of errors.
#[derive(Default, Debug)]
pub(crate) struct CharacterRequestHandler {
    api: Arc<ArtifactApi>,
    account: Arc<AccountClient>,
    data: CharacterData,
    bank: Arc<BankClient>,
    server: Arc<ServerClient>,
}

impl CharacterRequestHandler {
    pub fn new(
        api: Arc<ArtifactApi>,
        data: CharacterData,
        account: Arc<AccountClient>,
        server: Arc<ServerClient>,
    ) -> Self {
        Self {
            api,
            data,
            bank: account.bank.clone(),
            account,
            server,
        }
    }

    fn request_action(&self, action: Action) -> Result<Box<dyn ResponseSchema>, RequestError> {
        let mut bank_content: Option<RwLockWriteGuard<'_, Arc<Vec<SimpleItemSchema>>>> = None;
        let mut bank_details: Option<RwLockWriteGuard<'_, Arc<BankSchema>>> = None;

        self.wait_for_cooldown();
        if action.is_deposit_item() || action.is_withdraw_item() {
            bank_content = Some(
                self.bank
                    .content
                    .write()
                    .expect("bank_content to be writable"),
            );
        }
        if action.is_deposit_gold() || action.is_withdraw_gold() || action.is_expand_bank() {
            bank_details = Some(
                self.bank
                    .details
                    .write()
                    .expect("bank_details to be writable"),
            );
        }
        match action.request(&self.name(), &self.api) {
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
                if let Some(res) = res.downcast_ref::<BankItemTransactionResponseSchema>()
                    && let Some(mut content) = bank_content
                {
                    *content = res.data.bank.clone().into();
                } else if let Some(res) = res.downcast_ref::<BankGoldTransactionResponseSchema>()
                    && let Some(mut details) = bank_details
                {
                    let mut new_details = (*(*details)).clone();
                    new_details.gold = res.data.bank.quantity;
                    *details = Arc::new(new_details);
                } else if res
                    .downcast_ref::<BankExtensionTransactionResponseSchema>()
                    .is_some()
                    && let Some(mut details) = bank_details
                {
                    let mut new_details = (*(*details)).clone();
                    new_details.slots += BANK_EXTENSION_SIZE;
                    *details = Arc::new(new_details);
                };
                if let Some(res) = res.downcast_ref::<GiveItemResponseSchema>()
                    && let Some(c) = self
                        .account
                        .get_character_by_name(&res.data.receiver_character.name)
                {
                    c.update_data(*res.data.receiver_character.clone());
                }
                if let Some(res) = res.downcast_ref::<GiveGoldResponseSchema>()
                    && let Some(c) = self
                        .account
                        .get_character_by_name(&res.data.receiver_character.name)
                {
                    c.update_data(*res.data.receiver_character.clone());
                }
                Ok(res)
            }
            Err(e) => {
                drop(bank_content);
                drop(bank_details);
                self.handle_request_error(action, e)
            }
        }
    }

    fn handle_request_error(
        &self,
        action: Action,
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
        self.request_action(Action::Move { x, y })
            .and_then(|r| {
                r.downcast::<CharacterMovementResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| Map::new(*s.data.destination))
    }

    pub fn request_transition(&self) -> Result<Arc<MapSchema>, RequestError> {
        self.request_action(Action::Transition)
            .and_then(|r| {
                r.downcast::<CharacterTransitionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| Arc::new(*s.data.destination))
    }

    pub fn request_fight(
        &self,
        participants: Option<&[String; 2]>,
    ) -> Result<CharacterFightSchema, RequestError> {
        self.request_action(Action::Fight { participants })
            .and_then(|r| {
                r.downcast::<CharacterFightResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.fight)
    }

    pub fn request_rest(&self) -> Result<u32, RequestError> {
        self.request_action(Action::Rest)
            .and_then(|r| {
                r.downcast::<CharacterRestResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.hp_restored as u32)
    }

    pub fn request_gather(&self) -> Result<SkillDataSchema, RequestError> {
        self.request_action(Action::Gather)
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
        self.request_action(Action::Craft {
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
        self.request_action(Action::Delete {
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
        self.request_action(Action::Recycle {
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
        self.request_action(Action::DepositItem { items })
            .map(|_| ())
    }

    pub fn request_withdraw_item(&self, items: &[SimpleItemSchema]) -> Result<(), RequestError> {
        self.request_action(Action::WithdrawItem { items })
            .map(|_| ())
    }

    pub fn request_deposit_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(Action::DepositGold { quantity })
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_withdraw_gold(&self, quantity: u32) -> Result<u32, RequestError> {
        self.request_action(Action::WithdrawGold { quantity })
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| r.data.bank.quantity)
    }

    pub fn request_expand_bank(&self) -> Result<u32, RequestError> {
        self.request_action(Action::ExpandBank)
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
        self.request_action(Action::Equip {
            item_code,
            slot,
            quantity,
        })
        .map(|_| ())
    }

    pub fn request_unequip(&self, slot: Slot, quantity: u32) -> Result<(), RequestError> {
        self.request_action(Action::Unequip { slot, quantity })
            .map(|_| ())
    }

    pub fn request_use_item(&self, item_code: &str, quantity: u32) -> Result<(), RequestError> {
        self.request_action(Action::UseItem {
            item_code,
            quantity,
        })
        .map(|_| ())
    }

    pub fn request_accept_task(&self) -> Result<TaskSchema, RequestError> {
        self.request_action(Action::AcceptTask)
            .and_then(|r| {
                r.downcast::<TaskResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| *r.data.task)
    }

    pub fn request_complete_task(&self) -> Result<RewardsSchema, RequestError> {
        self.request_action(Action::CompleteTask)
            .and_then(|r| {
                r.downcast::<RewardDataResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.rewards)
    }

    pub fn request_cancel_task(&self) -> Result<(), RequestError> {
        self.request_action(Action::CancelTask).map(|_| ())
    }

    pub fn request_trade_task_item(
        &self,
        item_code: &str,
        quantity: u32,
    ) -> Result<TaskTradeSchema, RequestError> {
        self.request_action(Action::TradeTaskItem {
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
        self.request_action(Action::ExchangeTasksCoins)
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
        self.request_action(Action::NpcBuy {
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
        self.request_action(Action::NpcSell {
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
        self.request_action(Action::GiveItem { items, character })
            .and_then(|r| {
                r.downcast::<GiveItemResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|_| ())
    }

    pub fn request_give_gold(&self, quantity: u32, character: &str) -> Result<(), RequestError> {
        self.request_action(Action::GiveGold {
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
        self.request_action(Action::GeBuyOrder { id, quantity })
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
        self.request_action(Action::GeCreateOrder {
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
        self.request_action(Action::GeCancelOrder { id })
            .and_then(|r| {
                r.downcast::<GeTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|r| *r.data.order)
    }
}

impl HasCharacterData for CharacterRequestHandler {
    fn data(&self) -> Arc<CharacterSchema> {
        self.data.read().unwrap().clone()
    }

    fn refresh_data(&self) {
        let Ok(res) = self.api.character.get(&self.name()) else {
            return;
        };
        self.update_data(*res.data)
    }

    fn update_data(&self, schema: CharacterSchema) {
        *self.data.write().unwrap() = Arc::new(schema)
    }
}

pub trait ResponseSchema: Downcast {
    fn character(&self) -> &CharacterSchema;
    fn to_string(&self) -> String;
}
impl_downcast!(ResponseSchema);

impl ResponseSchema for CharacterMovementResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: moved to {}. {}s",
            self.data.character.name,
            Map::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterTransitionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: transitioned to {}. {}s",
            self.data.character.name,
            Map::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterFightResponseSchema {
    fn to_string(&self) -> String {
        let chars = &self.data.fight.characters;
        let names = chars.iter().map(|c| c.character_name.to_string()).join(",");
        let drops = chars.iter().flat_map(|c| c.drops.clone()).collect_vec();
        let xp = chars.iter().map(|c| c.xp).join("/");
        let gold = chars.iter().map(|c| c.gold).join("/");
        match self.data.fight.result {
            FightResult::Win => format!(
                "{} won a fight after {} turns ([{}], {}xp, {}g). {}s",
                names,
                self.data.fight.turns,
                DropSchemas(&drops),
                xp,
                gold,
                self.data.cooldown.remaining_seconds
            ),
            FightResult::Loss => format!(
                "{} lost a fight after {} turns. {}s",
                self.data.characters.first().unwrap().name,
                self.data.fight.turns,
                self.data.cooldown.remaining_seconds
            ),
        }
    }

    fn character(&self) -> &CharacterSchema {
        self.data.characters.first().unwrap()
    }
}

impl ResponseSchema for CharacterRestResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: rested and restored {}hp. {}s",
            self.data.character.name, self.data.hp_restored, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for UseItemResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: used '{}'. {}s",
            self.data.character.name, self.data.item.code, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for SkillResponseSchema {
    fn to_string(&self) -> String {
        let reason = if self.data.cooldown.reason == ActionType::Crafting {
            "crafted"
        } else {
            "gathered"
        };
        format!(
            "{}: {reason} [{}] ({}xp). {}s",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.details.xp,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for DeleteItemResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: deleted '{}'x{}",
            self.data.character.name, self.data.item.code, self.data.item.quantity
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankItemTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::WithdrawItem {
            format!(
                "{}: withdrawed [{}] from the bank. {}s",
                self.data.character.name,
                SimpleItemSchemas(&self.data.items),
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: deposited [{}] to the bank. {}s",
                self.data.character.name,
                SimpleItemSchemas(&self.data.items),
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankGoldTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::WithdrawGold {
            format!(
                "{}: withdrawed gold from the bank. {}s",
                self.data.character.name, self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: deposited gold to the bank. {}s",
                self.data.character.name, self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankExtensionTransactionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: bought bank expansion for {} golds. {}s",
            self.data.character.name,
            self.data.transaction.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for RecyclingResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: recycled and received {}. {}s",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for EquipmentResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::Equip {
            format!(
                "{}: equiped '{}' in the '{}' slot. {}s",
                &self.data.character.name,
                &self.data.item.code,
                &self.data.slot,
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: unequiped '{}' from the '{}' slot. {}s",
                &self.data.character.name,
                &self.data.item.code,
                &self.data.slot,
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: accepted new [{:?}] task: '{}'x{}. {}s",
            self.data.character.name,
            self.data.task.r#type,
            self.data.task.code,
            self.data.task.total,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for RewardDataResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: completed task and was rewarded with [{}] and {}g. {}s",
            self.data.character.name,
            SimpleItemSchemas(&self.data.rewards.items),
            self.data.rewards.gold,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: cancelled current task. {}s",
            self.data.character.name, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskTradeResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: traded '{}'x{} with the taskmaster. {}s",
            self.data.character.name,
            self.data.trade.code,
            self.data.trade.quantity,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for NpcMerchantTransactionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: traded {} {} for {} {}(s) at {} each. {}s",
            self.data.character.name,
            self.data.transaction.quantity,
            self.data.transaction.code,
            self.data.transaction.total_price,
            self.data.transaction.currency,
            self.data.transaction.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GiveItemResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: gave '{}' to {}. {}s",
            self.data.character.name,
            SimpleItemSchemas(&self.data.items),
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GiveGoldResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: gave {} gold to {}. {}s",
            self.data.character.name,
            self.data.quantity,
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GeTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::BuyGe {
            format!(
                "{}: bought '{}'x{} for {}g from the grand exchange. {}",
                self.data.character.name,
                self.data.order.code,
                self.data.order.quantity,
                self.data.order.total_price,
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: canceled order '{}'x{} for {}g at the grand exchange. {}",
                self.data.character.name,
                self.data.order.code,
                self.data.order.quantity,
                self.data.order.total_price,
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GeCreateOrderTransactionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: created order '{}'x{} for {}g at the grand exchange. {}s",
            self.data.character.name,
            self.data.order.code,
            self.data.order.quantity,
            self.data.order.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl<T: ResponseSchema + 'static> From<T> for Box<dyn ResponseSchema> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}
