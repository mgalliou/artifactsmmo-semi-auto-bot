use super::Character;
use crate::artifactsmmo_sdk::{
    items::Slot, ApiErrorResponseSchema, FightSchemaExt, MapSchemaExt, ResponseSchema,
    SkillSchemaExt,
};
use artifactsmmo_openapi::{
    apis::Error,
    models::{
        cooldown_schema::Reason, fight_schema, BankItemTransactionResponseSchema,
        CharacterFightResponseSchema, CharacterMovementResponseSchema, CharacterSchema, DropSchema,
        EquipmentResponseSchema, FightSchema, MapContentSchema, MapSchema, RecyclingResponseSchema,
        SimpleItemSchema, SkillDataSchema, SkillResponseSchema, TaskCancelledResponseSchema,
        TaskResponseSchema, TaskSchema, TaskTradeResponseSchema, TaskTradeSchema,
        TasksRewardResponseSchema, TasksRewardSchema,
    },
};
use log::{error, info};
use std::{fmt::Display, thread::sleep, time::Duration};
use strum_macros::EnumIs;

impl Character {
    pub fn perform_action(&self, action: Action) -> Result<CharacterResponseSchema, RequestError> {
        self.wait_for_cooldown();
        let res: Result<CharacterResponseSchema, RequestError> = match action {
            Action::Move { x, y } => self
                .my_api
                .move_to(&self.name, x, y)
                .map(CharacterResponseSchema::Movement)
                .map_err(|e| e.into()),
            Action::Fight => self
                .my_api
                .fight(&self.name)
                .map(CharacterResponseSchema::Fight)
                .map_err(|e| e.into()),
            Action::Gather => self
                .my_api
                .gather(&self.name)
                .map(CharacterResponseSchema::Skill)
                .map_err(|e| e.into()),
            Action::Craft { code, quantity } => self
                .my_api
                .craft(&self.name, code, quantity)
                .map(CharacterResponseSchema::Skill)
                .map_err(|e| e.into()),
            Action::Withdraw { code, quantity } => self
                .my_api
                .withdraw(&self.name, code, quantity)
                .map(CharacterResponseSchema::BankItemTransaction)
                .map_err(|e| e.into()),
            Action::Deposit { code, quantity } => self
                .my_api
                .deposit(&self.name, code, quantity)
                .map(CharacterResponseSchema::BankItemTransaction)
                .map_err(|e| e.into()),
            Action::Recycle { code, quantity } => self
                .my_api
                .recycle(&self.name, code, quantity)
                .map(CharacterResponseSchema::Recycle)
                .map_err(|e| e.into()),
            Action::Equip {
                code,
                slot,
                quantity,
            } => self
                .my_api
                .equip(&self.name, code, slot.to_equip_schema(), Some(quantity))
                .map(CharacterResponseSchema::Equip)
                .map_err(|e| e.into()),
            Action::Unequip { slot, quantity } => {
                self.my_api
                    .unequip(&self.name, slot.to_unequip_schema(), Some(quantity))
            }
            .map(CharacterResponseSchema::Unequip)
            .map_err(|e| e.into()),
            Action::AcceptTask => self
                .my_api
                .accept_task(&self.name)
                .map(CharacterResponseSchema::Task)
                .map_err(|e| e.into()),
            Action::CompleteTask => self
                .my_api
                .complete_task(&self.name)
                .map(CharacterResponseSchema::CompleteTask)
                .map_err(|e| e.into()),
            Action::CancelTask => self
                .my_api
                .cancel_task(&self.name)
                .map(CharacterResponseSchema::TaskCancel)
                .map_err(|e| e.into()),
            Action::TaskTrade { code, quantity } => self
                .my_api
                .trade_task(&self.name, code, quantity)
                .map(CharacterResponseSchema::TaskTrade)
                .map_err(|e| e.into()),
        };
        match res {
            Ok(res) => {
                info!("{}", res.pretty());
                self.update_data(res.character());
                Ok(res)
            }
            Err(e) => self.handle_action_error(action, e),
        }
    }

    pub fn action_move(&self, x: i32, y: i32) -> Result<MapSchema, RequestError> {
        if self.position() == (x, y) {
            return Ok(self.map().clone());
        }
        self.perform_action(Action::Move { x, y }).map(|r| {
            if let CharacterResponseSchema::Movement(s) = r {
                *s.data.destination
            } else {
                unreachable!()
            }
        })
    }

    pub fn action_fight(&self) -> Result<FightSchema, RequestError> {
        self.perform_action(Action::Fight).map(|r| {
            if let CharacterResponseSchema::Fight(s) = r {
                *s.data.fight
            } else {
                unreachable!()
            }
        })
    }

    pub fn action_gather(&self) -> Result<SkillDataSchema, RequestError> {
        self.perform_action(Action::Gather).map(|r| {
            if let CharacterResponseSchema::Skill(s) = r {
                *s.data
            } else {
                unreachable!()
            }
        })
    }

    pub fn action_withdraw(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.move_to_closest_map_of_type("bank");
        let mut bank_content = self
            .bank
            .content
            .write()
            .expect("bank_content to be writable");
        self.perform_action(Action::Withdraw { code, quantity })
            .map(|r| {
                if let CharacterResponseSchema::BankItemTransaction(s) = r {
                    *bank_content = s.data.bank
                };
                SimpleItemSchema {
                    code: code.to_owned(),
                    quantity,
                }
            })
    }

    pub fn action_deposit(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, RequestError> {
        self.move_to_closest_map_of_type("bank");
        let mut bank_content = self
            .bank
            .content
            .write()
            .expect("bank_content to be writable");
        self.perform_action(Action::Deposit { code, quantity })
            .map(|r| {
                if let CharacterResponseSchema::BankItemTransaction(s) = r {
                    *bank_content = s.data.bank
                };
                SimpleItemSchema {
                    code: code.to_owned(),
                    quantity,
                }
            })
    }

    pub fn action_craft(&self, code: &str, quantity: i32) -> Result<(), RequestError> {
        self.move_to_craft(code);
        self.perform_action(Action::Craft { code, quantity })
            .map(|_| ())
    }

    pub fn action_recycle(&self, code: &str, quantity: i32) -> Result<(), RequestError> {
        self.move_to_craft(code);
        self.perform_action(Action::Recycle { code, quantity })
            .map(|_| ())
    }

    pub fn action_equip(&self, code: &str, slot: Slot, quantity: i32) -> Result<(), RequestError> {
        if self.equiped_in(slot).is_some() {
            let quantity = match slot {
                Slot::Consumable1 => self.data.read().unwrap().consumable1_slot_quantity,
                Slot::Consumable2 => self.data.read().unwrap().consumable2_slot_quantity,
                _ => 1,
            };
            let _ = self.action_unequip(slot, quantity);
        }
        self.perform_action(Action::Equip {
            code,
            slot,
            quantity,
        })
        .map(|_| ())
    }

    pub fn action_unequip(&self, slot: Slot, quantity: i32) -> Result<(), RequestError> {
        self.perform_action(Action::Unequip { slot, quantity })
            .map(|_| ())
    }

    pub fn action_accept_task(&self, r#type: &str) -> Result<TaskSchema, RequestError> {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: r#type.to_owned(),
        });
        self.perform_action(Action::AcceptTask).map(|r| {
            if let CharacterResponseSchema::Task(s) = r {
                *s.data.task
            } else {
                unreachable!()
            }
        })
    }

    pub fn action_complete_task(&self) -> Result<TasksRewardSchema, RequestError> {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: self.task_type().to_owned(),
        });
        self.perform_action(Action::CompleteTask).map(|r| {
            if let CharacterResponseSchema::CompleteTask(s) = r {
                *s.data.reward
            } else {
                unreachable!()
            }
        })
    }

    pub fn action_cancel_task(&self) -> Result<(), RequestError> {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: self.task_type().to_owned(),
        });
        self.perform_action(Action::CancelTask).map(|_| ())
    }

    pub fn action_task_trade(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<TaskTradeSchema, RequestError> {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: "items".to_owned(),
        });
        self.perform_action(Action::TaskTrade { code, quantity })
            .map(|r| {
                if let CharacterResponseSchema::TaskTrade(s) = r {
                    *s.data.trade
                } else {
                    unreachable!()
                }
            })
    }

    fn handle_action_error(
        &self,
        action: Action,
        e: RequestError,
    ) -> Result<CharacterResponseSchema, RequestError> {
        if let RequestError::ResponseError(ref res) = e {
            if res.error.code == 499 {
                self.game.update_offset();
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
        Err(e)
    }
}

#[derive(Debug, EnumIs)]
pub enum Action<'a> {
    Move {
        x: i32,
        y: i32,
    },
    Fight,
    Gather,
    Craft {
        code: &'a str,
        quantity: i32,
    },
    Withdraw {
        code: &'a str,
        quantity: i32,
    },
    Deposit {
        code: &'a str,
        quantity: i32,
    },
    Recycle {
        code: &'a str,
        quantity: i32,
    },
    Equip {
        code: &'a str,
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
        code: &'a str,
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

pub enum CharacterResponseSchema {
    Movement(CharacterMovementResponseSchema),
    Fight(CharacterFightResponseSchema),
    Skill(SkillResponseSchema),
    Recycle(RecyclingResponseSchema),
    BankItemTransaction(BankItemTransactionResponseSchema),
    Task(TaskResponseSchema),
    TaskCancel(TaskCancelledResponseSchema),
    TaskTrade(TaskTradeResponseSchema),
    Equip(EquipmentResponseSchema),
    Unequip(EquipmentResponseSchema),
    CompleteTask(TasksRewardResponseSchema),
}

impl CharacterResponseSchema {
    fn character(&self) -> &CharacterSchema {
        match self {
            CharacterResponseSchema::Movement(s) => s.character(),
            CharacterResponseSchema::Fight(s) => s.character(),
            CharacterResponseSchema::Skill(s) => s.character(),
            CharacterResponseSchema::Recycle(s) => s.character(),
            CharacterResponseSchema::BankItemTransaction(s) => s.character(),
            CharacterResponseSchema::Task(s) => s.character(),
            CharacterResponseSchema::TaskCancel(s) => s.character(),
            CharacterResponseSchema::TaskTrade(s) => s.character(),
            CharacterResponseSchema::Equip(s) => s.character(),
            CharacterResponseSchema::Unequip(s) => s.character(),
            CharacterResponseSchema::CompleteTask(s) => s.character(),
        }
    }

    fn pretty(&self) -> String {
        match self {
            CharacterResponseSchema::Movement(s) => s.pretty(),
            CharacterResponseSchema::Fight(s) => s.pretty(),
            CharacterResponseSchema::Skill(s) => s.pretty(),
            CharacterResponseSchema::Recycle(s) => s.pretty(),
            CharacterResponseSchema::BankItemTransaction(s) => s.pretty(),
            CharacterResponseSchema::Task(s) => s.pretty(),
            CharacterResponseSchema::TaskCancel(s) => s.pretty(),
            CharacterResponseSchema::TaskTrade(s) => s.pretty(),
            CharacterResponseSchema::Equip(s) => s.pretty(),
            CharacterResponseSchema::Unequip(s) => s.pretty(),
            CharacterResponseSchema::CompleteTask(s) => s.pretty(),
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
            fight_schema::Result::Win => format!(
                "{} won a fight after {} turns ({}xp, {}g, [{}]). {}s",
                self.data.character.name,
                self.data.fight.turns,
                self.data.fight.xp,
                self.data.fight.gold,
                DropSchemas(&self.data.fight.drops),
                self.data.cooldown.remaining_seconds
            ),
            fight_schema::Result::Lose => format!(
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

impl ResponseSchema for SkillResponseSchema {
    fn pretty(&self) -> String {
        let reason = if self.data.cooldown.reason == Reason::Crafting {
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

impl ResponseSchema for BankItemTransactionResponseSchema {
    fn pretty(&self) -> String {
        if self.data.cooldown.reason == Reason::WithdrawBank {
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
        if self.data.cooldown.reason == Reason::Equip {
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

impl ResponseSchema for TasksRewardResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: completed task and was rewarded with '{}'x{}. {}s",
            self.data.character.name,
            self.data.reward.code,
            self.data.reward.quantity,
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
    fn amount_of(&self, code: &str) -> i32 {
        self.drops
            .iter()
            .find(|i| i.code == code)
            .map_or(0, |i| i.quantity)
    }
}

impl SkillSchemaExt for SkillDataSchema {
    fn amount_of(&self, code: &str) -> i32 {
        self.details
            .items
            .iter()
            .find(|i| i.code == code)
            .map_or(0, |i| i.quantity)
    }
}

