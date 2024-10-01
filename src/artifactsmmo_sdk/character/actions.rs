use super::Character;
use crate::artifactsmmo_sdk::{
    items::Slot, ApiErrorSchema, ApiRequestError, FightSchemaExt, MapSchemaExt, ResponseSchema, SkillSchemaExt
};
use artifactsmmo_openapi::{
    apis::Error,
    models::{
        cooldown_schema::Reason, fight_schema, BankItemTransactionResponseSchema,
        CharacterFightResponseSchema, CharacterMovementResponseSchema, CharacterSchema, DropSchema,
        EquipmentResponseSchema, FightSchema, MapContentSchema, RecyclingResponseSchema,
        SkillDataSchema, SkillResponseSchema, TaskCancelledResponseSchema, TaskResponseSchema,
        TaskTradeResponseSchema, TasksRewardResponseSchema,
    },
};
use log::{error, info};
use reqwest::StatusCode;
use std::fmt::Display;
use strum_macros::EnumIs;

impl Character {
    pub fn perform_action(
        &self,
        action: Action,
    ) -> Result<CharacterResponseSchema, Box<dyn ApiRequestError>> {
        self.wait_for_cooldown();
        let res: Result<CharacterResponseSchema, Box<dyn ApiRequestError>> = match action {
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
            .map(CharacterResponseSchema::Unquip)
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
                .task_trade(&self.name, code, quantity)
                .map(CharacterResponseSchema::TaskTrade)
                .map_err(|e| e.into()),
        };
        match res {
            Ok(res) => {
                info!("{}", res.pretty());
                self.update_data(res.character());
                if let CharacterResponseSchema::BankItemTransaction(ref schema) = res {
                    self.bank.update_content(&schema.data.bank)
                };
                Ok(res)
            }
            Err(e) => self.handle_action_error(action, e),
        }
    }

    pub fn action_move(&self, x: i32, y: i32) -> bool {
        if self.position() == (x, y) {
            return true;
        }
        self.perform_action(Action::Move { x, y }).is_ok()
    }

    pub fn action_fight(&self) -> Result<FightSchema, Box<dyn ApiRequestError>> {
        match self.perform_action(Action::Fight) {
            Ok(res) => match res {
                CharacterResponseSchema::Fight(s) => Ok(*s.data.fight),
                _ => unreachable!(),
            },
            Err(e) => Err(e),
        }
    }

    pub fn action_gather(&self) -> Result<SkillDataSchema, Box<dyn ApiRequestError>> {
        match self.perform_action(Action::Gather) {
            Ok(res) => match res {
                CharacterResponseSchema::Skill(s) => Ok(*s.data),
                _ => unreachable!(),
            },
            Err(e) => Err(e),
        }
    }

    pub fn action_withdraw(&self, code: &str, quantity: i32) -> bool {
        self.move_to_closest_map_of_type("bank");
        self.perform_action(Action::Withdraw { code, quantity })
            .is_ok()
    }

    pub fn action_deposit(&self, code: &str, quantity: i32) -> bool {
        self.move_to_closest_map_of_type("bank");
        self.perform_action(Action::Deposit { code, quantity })
            .is_ok()
    }

    pub fn action_craft(&self, code: &str, quantity: i32) -> bool {
        self.move_to_craft(code);
        self.perform_action(Action::Craft { code, quantity })
            .is_ok()
    }

    pub fn action_recycle(&self, code: &str, quantity: i32) -> bool {
        self.move_to_craft(code);
        self.perform_action(Action::Recycle { code, quantity })
            .is_ok()
    }

    pub fn action_equip(&self, code: &str, slot: Slot, quantity: i32) -> bool {
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
        .is_ok()
    }

    pub fn action_unequip(&self, slot: Slot, quantity: i32) -> bool {
        self.perform_action(Action::Unequip { slot, quantity })
            .is_ok()
    }

    pub fn action_accept_task(&self, r#type: &str) -> bool {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: r#type.to_owned(),
        });
        self.perform_action(Action::AcceptTask).is_ok()
    }

    pub fn action_complete_task(&self) -> bool {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: self.task_type().to_owned(),
        });
        self.perform_action(Action::CompleteTask).is_ok()
    }

    pub fn action_cancel_task(&self) -> bool {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: self.task_type().to_owned(),
        });
        self.perform_action(Action::CancelTask).is_ok()
    }

    pub fn action_task_trade(&self, code: &str, quantity: i32) -> bool {
        self.move_to_closest_map_with_content_schema(&MapContentSchema {
            r#type: "tasks_master".to_owned(),
            code: "items".to_owned(),
        });
        self.perform_action(Action::TaskTrade { code, quantity })
            .is_ok()
    }

    fn handle_action_error(
        &self,
        action: Action,
        e: Box<dyn ApiRequestError>,
    ) -> Result<CharacterResponseSchema, Box<dyn ApiRequestError>> {
        if e.status_code()
            .is_some_and(|s| s.eq(&StatusCode::from_u16(499).unwrap()))
        {
            self.game.update_offset();
            return self.perform_action(action);
        };
        match e.api_error() {
            Some(e) => error!(
                "{}: error while performing action '{:?}': {} ({}).",
                self.name, action, e.error.message, e.error.code,
            ),
            None => error!(
                "{}: unkown error while performing action '{:?}'.",
                self.name, action,
            ),
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
    None
}

impl<T> ApiRequestError for Error<T> {
    fn status_code(&self) -> Option<StatusCode> {
        if let Error::ResponseError(e) = self {
            return Some(e.status);
        }
        None
    }

    fn api_error(&self) -> Option<ApiErrorSchema> {
        if let Error::ResponseError(e) = self {
            match serde_json::from_str(&e.content) {
                Ok(e) => return Some(e),
                Err(e) => {
                    error!("{}", e);
                    return None;
                }
            }
        }
        None
    }
}

impl<T: 'static> From<Error<T>> for Box<dyn ApiRequestError> {
    fn from(value: Error<T>) -> Self {
        Box::new(value)
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
    Unquip(EquipmentResponseSchema),
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
            CharacterResponseSchema::Unquip(s) => s.character(),
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
            CharacterResponseSchema::Unquip(s) => s.pretty(),
            CharacterResponseSchema::CompleteTask(s) => s.pretty(),
        }
    }
}

impl ResponseSchema for CharacterMovementResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: moved to {}.",
            self.data.character.name,
            self.data.destination.pretty()
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
                "{} won a fight after {} turns ({}xp, {}g, [{}]).",
                self.data.character.name,
                self.data.fight.turns,
                self.data.fight.xp,
                self.data.fight.gold,
                DropSchemas(&self.data.fight.drops)
            ),
            fight_schema::Result::Lose => format!(
                "{} lost a fight after {} turns.",
                self.data.character.name, self.data.fight.turns
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
            "{}: {reason} [{}] ({}xp).",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.details.xp,
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
                "{}: withdrawed '{}' from the bank.",
                self.data.character.name, self.data.item.code
            )
        } else {
            format!(
                "{}: deposited '{}' to the bank.",
                self.data.character.name, self.data.item.code
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
            "{}: recycled and received {}.",
            self.data.character.name,
            DropSchemas(&self.data.details.items)
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
                "{}: equiped '{}' in the '{:?}' slot",
                &self.data.character.name, &self.data.item.code, &self.data.slot
            )
        } else {
            format!(
                "{}: unequiped '{}' from the '{:?}' slot",
                &self.data.character.name, &self.data.item.code, &self.data.slot
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
            "{}: accepted new [{:?}] task: '{}'x{}.",
            self.data.character.name,
            self.data.task.r#type,
            self.data.task.code,
            self.data.task.total,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TasksRewardResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: completed task and was rewarded with '{}'x{}.",
            self.data.character.name, self.data.reward.code, self.data.reward.quantity
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn pretty(&self) -> String {
        format!("{}: cancelled current task.", self.data.character.name,)
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskTradeResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: traded '{}'x{} with the taskmaster.",
            self.data.character.name, self.data.trade.code, self.data.trade.quantity
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

#[derive(Debug)]
pub enum SkillError {
    InsuffisientSkillLevel,
    InsuffisientMaterials,
    InvalidQuantity,
    ApiError(ApiErrorSchema),
}

#[derive(Debug)]
pub enum FightError {
    NoEquipmentToKill,
    ApiError(ApiErrorSchema),
}
