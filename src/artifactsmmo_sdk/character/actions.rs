use super::Character;
use crate::artifactsmmo_sdk::{
    gear::Slot, ApiErrorResponseSchema, FightSchemaExt, MapSchemaExt, ResponseSchema,
    SkillInfoSchemaExt, SkillSchemaExt, TaskRewardsSchemaExt,
};
use artifactsmmo_openapi::{
    apis::Error,
    models::{
        ActionType, BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
        BankItemTransactionResponseSchema, BankSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, CharacterRestResponseSchema, CharacterSchema,
        DeleteItemResponseSchema, DropSchema, EquipmentResponseSchema, FightResult, FightSchema,
        MapSchema, RecyclingItemsSchema, RecyclingResponseSchema, SimpleItemSchema,
        SkillDataSchema, SkillInfoSchema, SkillResponseSchema, TaskCancelledResponseSchema,
        TaskResponseSchema, TaskRewardsSchema, TaskSchema, TaskTradeResponseSchema,
        TaskTradeSchema, TasksRewardDataResponseSchema, UseItemResponseSchema,
    },
};
use chrono::{DateTime, Utc};
use log::{debug, error, info};
use std::{cmp::Ordering, fmt::Display, sync::RwLockWriteGuard, thread::sleep, time::Duration};
use strum_macros::{Display, EnumIs};

impl Character {
    fn perform_action(&self, action: Action) -> Result<Box<dyn ResponseSchema>, RequestError> {
        let mut bank_content: Option<RwLockWriteGuard<'_, Vec<SimpleItemSchema>>> = None;
        let mut bank_details: Option<RwLockWriteGuard<'_, BankSchema>> = None;

        self.wait_for_cooldown();
        if action.is_deposit() || action.is_withdraw() {
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
        let res: Result<Box<dyn ResponseSchema>, RequestError> = match action {
            Action::Move { x, y } => self
                .my_api
                .move_to(&self.name, x, y)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Fight => self
                .my_api
                .fight(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Rest => self
                .my_api
                .rest(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::UseItem { item, quantity } => self
                .my_api
                .use_item(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Gather => self
                .my_api
                .gather(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Craft { item, quantity } => self
                .my_api
                .craft(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Delete { item, quantity } => self
                .my_api
                .delete(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Deposit { item, quantity } => self
                .my_api
                .deposit(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Withdraw { item, quantity } => self
                .my_api
                .withdraw(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::DepositGold { quantity } => self
                .my_api
                .deposit_gold(&self.name, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::WithdrawGold { quantity } => self
                .my_api
                .withdraw_gold(&self.name, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::ExpandBank => self
                .my_api
                .expand_bank(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Recycle { item, quantity } => self
                .my_api
                .recycle(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Equip {
                item,
                slot,
                quantity,
            } => self
                .my_api
                .equip(&self.name, item, slot.into(), Some(quantity))
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Unequip { slot, quantity } => {
                { self.my_api.unequip(&self.name, slot.into(), Some(quantity)) }
                    .map(|r| r.into())
                    .map_err(|e| e.into())
            }
            Action::AcceptTask => self
                .my_api
                .accept_task(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CompleteTask => self
                .my_api
                .complete_task(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CancelTask => self
                .my_api
                .cancel_task(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::TaskTrade { item, quantity } => self
                .my_api
                .trade_task(&self.name, item, quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::TaskExchange => self
                .my_api
                .task_exchange(&self.name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
        };
        match res {
            Ok(res) => {
                info!("{}", res.pretty());
                self.update_data(res.character());
                if let Some(s) = res.downcast_ref::<BankItemTransactionResponseSchema>() {
                    if let Some(mut bank_content) = bank_content {
                        *bank_content = s.data.bank.clone();
                    }
                } else if let Some(s) = res.downcast_ref::<BankGoldTransactionResponseSchema>() {
                    if let Some(mut bank_details) = bank_details {
                        bank_details.gold = s.data.bank.quantity
                    }
                } else if res
                    .downcast_ref::<BankExtensionTransactionResponseSchema>()
                    .is_some()
                {
                    if let Some(mut bank_details) = bank_details {
                        bank_details.slots += 20;
                    }
                };
                Ok(res)
            }
            Err(e) => {
                drop(bank_content);
                drop(bank_details);
                self.handle_action_error(action, e)
            }
        }
    }

    pub fn action_move(&self, x: i32, y: i32) -> Result<MapSchema, RequestError> {
        if self.position() == (x, y) {
            return Ok((*self.map()).clone());
        }
        self.perform_action(Action::Move { x, y })
            .and_then(|r| {
                r.downcast::<CharacterMovementResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.destination)
    }

    pub fn action_fight(&self) -> Result<FightSchema, RequestError> {
        self.perform_action(Action::Fight)
            .and_then(|r| {
                r.downcast::<CharacterFightResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.fight)
    }

    pub fn action_rest(&self) -> Result<i32, RequestError> {
        self.perform_action(Action::Rest)
            .and_then(|r| {
                r.downcast::<CharacterRestResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.hp_restored)
    }

    pub fn action_use_item(&self, item: &str, quantity: i32) -> Result<(), RequestError> {
        self.perform_action(Action::UseItem {
            item,
            quantity,
        })
        .map(|_| ())
    }

    pub fn action_gather(&self) -> Result<SkillDataSchema, RequestError> {
        self.perform_action(Action::Gather)
            .and_then(|r| {
                r.downcast::<SkillResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data)
    }

    pub fn action_deposit(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.perform_action(Action::Deposit {
            item,
            quantity,
        })
        .map(|_| SimpleItemSchema {
            code: item.to_owned(),
            quantity,
        })
    }

    pub fn action_withdraw(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.perform_action(Action::Withdraw { item, quantity })
            .map(|_| SimpleItemSchema {
                code: item.to_owned(),
                quantity,
            })
    }

    pub fn action_deposit_gold(&self, quantity: i32) -> Result<i32, RequestError> {
        self.perform_action(Action::DepositGold { quantity })
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.bank.quantity)
    }

    pub fn action_withdraw_gold(&self, quantity: i32) -> Result<i32, RequestError> {
        self.perform_action(Action::WithdrawGold { quantity })
            .and_then(|r| {
                r.downcast::<BankGoldTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.bank.quantity)
    }

    pub fn action_expand_bank(&self) -> Result<i32, RequestError> {
        self.perform_action(Action::ExpandBank)
            .and_then(|r| {
                r.downcast::<BankExtensionTransactionResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| s.data.transaction.price)
    }

    pub fn action_craft(&self, item: &str, quantity: i32) -> Result<SkillInfoSchema, RequestError> {
        self.perform_action(Action::Craft {
            item,
            quantity,
        })
        .and_then(|r| {
            r.downcast::<SkillResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|s| *s.data.details)
    }

    pub fn action_delete(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.perform_action(Action::Delete {
            item,
            quantity,
        })
        .and_then(|r| {
            r.downcast::<DeleteItemResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|s| *s.data.item)
    }

    pub fn action_recycle(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, RequestError> {
        self.perform_action(Action::Recycle {
            item,
            quantity,
        })
        .and_then(|r| {
            r.downcast::<RecyclingResponseSchema>()
                .map_err(|_| RequestError::DowncastError)
        })
        .map(|s| *s.data.details)
    }

    pub fn action_equip(&self, item: &str, slot: Slot, quantity: i32) -> Result<(), RequestError> {
        self.perform_action(Action::Equip {
            item,
            slot,
            quantity,
        })
        .map(|_| ())
    }

    pub fn action_unequip(&self, slot: Slot, quantity: i32) -> Result<(), RequestError> {
        self.perform_action(Action::Unequip { slot, quantity })
            .map(|_| ())
    }

    pub fn action_accept_task(&self) -> Result<TaskSchema, RequestError> {
        self.perform_action(Action::AcceptTask)
            .and_then(|r| {
                r.downcast::<TaskResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.task)
    }

    pub fn action_complete_task(&self) -> Result<TaskRewardsSchema, RequestError> {
        self.perform_action(Action::CompleteTask)
            .and_then(|r| {
                r.downcast::<TasksRewardDataResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.rewards)
    }

    pub fn action_cancel_task(&self) -> Result<(), RequestError> {
        self.perform_action(Action::CancelTask).map(|_| ())
    }

    pub fn action_task_trade(
        &self,
        item: &str,
        quantity: i32,
    ) -> Result<TaskTradeSchema, RequestError> {
        self.perform_action(Action::TaskTrade { item, quantity })
            .and_then(|r| {
                r.downcast::<TaskTradeResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.trade)
    }

    pub fn action_task_exchange(&self) -> Result<TaskRewardsSchema, RequestError> {
        self.perform_action(Action::TaskExchange)
            .and_then(|r| {
                r.downcast::<TasksRewardDataResponseSchema>()
                    .map_err(|_| RequestError::DowncastError)
            })
            .map(|s| *s.data.rewards)
    }

    fn handle_action_error(
        &self,
        action: Action,
        e: RequestError,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        match e {
            RequestError::ResponseError(ref res) => {
                if res.error.code == 499 {
                    error!(
                        "{}: code 499 received, resyncronizing server time",
                        self.name
                    );
                    self.server.update_offset();
                    return self.perform_action(action);
                }
                if res.error.code == 500 || res.error.code == 520 {
                    error!(
                        "{}: unknown error ({}), retrying in 10 secondes.",
                        self.name, res.error.code
                    );
                    sleep(Duration::from_secs(10));
                    return self.perform_action(action);
                }
            }
            RequestError::Reqwest(ref req) => {
                if req.is_timeout() {
                    error!("{}: request timeout, retrying...", self.name);
                    return self.perform_action(action);
                }
            }
            _ => {}
        }
        error!(
            "{}: request error during action {:?}: {:?}",
            self.name, action, e
        );
        Err(e)
    }

    fn wait_for_cooldown(&self) {
        let s = self.remaining_cooldown();
        if s.is_zero() {
            return;
        }
        debug!(
            "{}: cooling down for {}.{} secondes.",
            self.name,
            s.as_secs(),
            s.subsec_millis()
        );
        sleep(s);
    }

    /// Returns the remaining cooldown duration of the `Character`.
    fn remaining_cooldown(&self) -> Duration {
        if let Some(exp) = self.cooldown_expiration() {
            let synced = Utc::now() - *self.server.server_offset.read().unwrap();
            if synced.cmp(&exp.to_utc()) == Ordering::Less {
                return (exp.to_utc() - synced).to_std().unwrap();
            }
        }
        Duration::from_secs(0)
    }

    /// Returns the cooldown expiration timestamp of the `Character`.
    fn cooldown_expiration(&self) -> Option<DateTime<Utc>> {
        self.data
            .read()
            .unwrap()
            .cooldown_expiration
            .as_ref()
            .map(|cd| DateTime::parse_from_rfc3339(cd).ok().map(|dt| dt.to_utc()))?
    }
}

#[derive(Debug, EnumIs, Display)]
pub enum Action<'a> {
    Move {
        x: i32,
        y: i32,
    },
    Fight,
    Gather,
    Craft {
        item: &'a str,
        quantity: i32,
    },
    Delete {
        item: &'a str,
        quantity: i32,
    },
    Deposit {
        item: &'a str,
        quantity: i32,
    },
    Withdraw {
        item: &'a str,
        quantity: i32,
    },
    DepositGold {
        quantity: i32,
    },
    WithdrawGold {
        quantity: i32,
    },
    ExpandBank,
    Recycle {
        item: &'a str,
        quantity: i32,
    },
    Equip {
        item: &'a str,
        slot: Slot,
        quantity: i32,
    },
    Unequip {
        slot: Slot,
        quantity: i32,
    },
    AcceptTask,
    CompleteTask,
    CancelTask,
    TaskTrade {
        item: &'a str,
        quantity: i32,
    },
    TaskExchange,
    Rest,
    UseItem {
        item: &'a str,
        quantity: i32,
    },
}

pub enum PostCraftAction {
    Deposit,
    Recycle,
    None,
}

#[derive(Debug)]
pub enum RequestError {
    Reqwest(reqwest::Error),
    Serde(serde_json::Error),
    Io(std::io::Error),
    ResponseError(ApiErrorResponseSchema),
    DowncastError,
}

impl<T> From<Error<T>> for RequestError {
    fn from(value: Error<T>) -> Self {
        match value {
            Error::Reqwest(e) => RequestError::Reqwest(e),
            Error::Serde(e) => RequestError::Serde(e),
            Error::Io(e) => RequestError::Io(e),
            Error::ResponseError(res) => match serde_json::from_str(&res.content) {
                Ok(e) => RequestError::ResponseError(e),
                Err(e) => RequestError::Serde(e),
            },
        }
    }
}

impl ResponseSchema for CharacterMovementResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: moved to {}. {}s",
            self.data.character.name,
            self.data.destination.pretty(),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterFightResponseSchema {
    fn pretty(&self) -> String {
        match self.data.fight.result {
            FightResult::Win => format!(
                "{} won a fight after {} turns ({}xp, {}g, [{}]). {}s",
                self.data.character.name,
                self.data.fight.turns,
                self.data.fight.xp,
                self.data.fight.gold,
                DropSchemas(&self.data.fight.drops),
                self.data.cooldown.remaining_seconds
            ),
            FightResult::Loss => format!(
                "{} lost a fight after {} turns. {}s",
                self.data.character.name,
                self.data.fight.turns,
                self.data.cooldown.remaining_seconds
            ),
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterRestResponseSchema {
    fn pretty(&self) -> String {
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
    fn pretty(&self) -> String {
        format!(
            "{}: used item '{}'. {}s",
            self.data.character.name, self.data.item.code, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for SkillResponseSchema {
    fn pretty(&self) -> String {
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
    fn pretty(&self) -> String {
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
    fn pretty(&self) -> String {
        if self.data.cooldown.reason == ActionType::Withdraw {
            format!(
                "{}: withdrawed '{}' from the bank. {}s",
                self.data.character.name, self.data.item.code, self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: deposited '{}' to the bank. {}s",
                self.data.character.name, self.data.item.code, self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankGoldTransactionResponseSchema {
    fn pretty(&self) -> String {
        if self.data.cooldown.reason == ActionType::Withdraw {
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
    fn pretty(&self) -> String {
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
    fn pretty(&self) -> String {
        format!(
            "{}: recycled and received {}. {}s",
            self.data.character.name,
            DropSchemas(&self.data.details.items,),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for EquipmentResponseSchema {
    fn pretty(&self) -> String {
        if self.data.cooldown.reason == ActionType::Equip {
            format!(
                "{}: equiped '{}' in the '{:?}' slot. {}s",
                &self.data.character.name,
                &self.data.item.code,
                &self.data.slot,
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: unequiped '{}' from the '{:?}' slot. {}s",
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
    fn pretty(&self) -> String {
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

impl ResponseSchema for TasksRewardDataResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: completed task and was rewarded with [{:?}] and {}g. {}s",
            self.data.character.name,
            self.data.rewards.items,
            self.data.rewards.gold,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn pretty(&self) -> String {
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
    fn pretty(&self) -> String {
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

impl<T: ResponseSchema + 'static> From<T> for Box<dyn ResponseSchema> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

struct DropSchemas<'a>(&'a Vec<DropSchema>);

impl<'a> Display for DropSchemas<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut items: String = "".to_string();
        for item in self.0 {
            if !items.is_empty() {
                items.push_str(", ");
            }
            items.push_str(&format!("'{}'x{}", item.code, item.quantity));
        }
        write!(f, "{}", items)
    }
}

impl FightSchemaExt for FightSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.drops
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl SkillSchemaExt for SkillDataSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.details
            .items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl SkillInfoSchemaExt for SkillInfoSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}

impl TaskRewardsSchemaExt for TaskRewardsSchema {
    fn amount_of(&self, item: &str) -> i32 {
        self.items
            .iter()
            .find(|i| i.code == item)
            .map_or(0, |i| i.quantity)
    }
}
