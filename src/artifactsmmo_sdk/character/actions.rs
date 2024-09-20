use std::fmt::Display;

use super::Character;
use crate::artifactsmmo_sdk::{items::Slot, MapSchemaExt, ResponseSchema};
use artifactsmmo_openapi::{
    apis::{
        my_characters_api::{
            ActionAcceptNewTaskMyNameActionTaskNewPostError,
            ActionCompleteTaskMyNameActionTaskCompletePostError,
            ActionCraftingMyNameActionCraftingPostError,
            ActionDepositBankMyNameActionBankDepositPostError,
            ActionEquipItemMyNameActionEquipPostError, ActionFightMyNameActionFightPostError,
            ActionGatheringMyNameActionGatheringPostError,
            ActionRecyclingMyNameActionRecyclingPostError,
            ActionTaskCancelMyNameActionTaskCancelPostError,
            ActionUnequipItemMyNameActionUnequipPostError,
            ActionWithdrawBankMyNameActionBankWithdrawPostError,
        },
        Error,
    },
    models::{
        fight_schema, BankItemTransactionResponseSchema, CharacterFightResponseSchema,
        CharacterMovementResponseSchema, DropSchema, EquipmentResponseSchema,
        RecyclingResponseSchema, SkillResponseSchema, TaskCancelledResponseSchema,
        TaskResponseSchema, TaskRewardResponseSchema,
    },
};
use log::{error, info};

impl ResponseSchema for CharacterMovementResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: moved to {}.",
            self.data.character.name,
            self.data.destination.pretty()
        )
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
}

impl ResponseSchema for SkillResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: gathered [{}] ({}xp).",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.details.xp,
        )
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
}

impl ResponseSchema for TaskRewardResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: completed task and was rewarded with '{}'x{}.",
            self.data.character.name, self.data.reward.code, self.data.reward.quantity
        )
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn pretty(&self) -> String {
        format!("{}: cancelled current task.", self.data.character.name,)
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

impl Character {
    pub(crate) fn action_move(&self, x: i32, y: i32) -> bool {
        if self.position() == (x, y) {
            return true;
        }
        self.wait_for_cooldown();
        match self.my_api.move_to(&self.name, x, y) {
            Ok(res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref())
            }
            Err(ref e) => error!("{}: error while moving to {},{}: {}", self.name, x, y, e),
        }
        false
    }

    pub(crate) fn action_fight(
        &self,
    ) -> Result<CharacterFightResponseSchema, Error<ActionFightMyNameActionFightPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.fight(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref())
            }
            Err(ref e) => error!("{}: error while fighting: {}", self.name, e),
        };
        res
    }

    pub(crate) fn action_gather(
        &self,
    ) -> Result<SkillResponseSchema, Error<ActionGatheringMyNameActionGatheringPostError>> {
        self.wait_for_cooldown();
        let res = self.my_api.gather(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while gathering: {}", self.name, e),
        };
        res
    }

    pub(crate) fn action_withdraw(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionWithdrawBankMyNameActionBankWithdrawPostError>,
    > {
        self.move_to_bank();
        self.wait_for_cooldown();
        let res = self.my_api.withdraw(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: withdrawed '{}'x{} from bank.",
                    self.name, code, quantity
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                self.bank.update_content(&res.data.bank);
            }
            Err(ref e) => error!(
                "{}: error while withdrawing '{}'x{}: {}.",
                self.name, code, quantity, e
            ),
        }
        res
    }

    pub(crate) fn action_deposit(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<
        BankItemTransactionResponseSchema,
        Error<ActionDepositBankMyNameActionBankDepositPostError>,
    > {
        self.move_to_bank();
        self.wait_for_cooldown();
        let res = self.my_api.deposit(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: deposited '{}'x{} into the bank.",
                    self.name, code, quantity
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                self.bank.update_content(&res.data.bank);
            }
            Err(ref e) => error!(
                "{}: error while depositing '{}'x{}: {}",
                self.name, code, quantity, e
            ),
        }
        res
    }

    pub(crate) fn action_craft(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<SkillResponseSchema, Error<ActionCraftingMyNameActionCraftingPostError>> {
        self.move_to_craft(code);
        self.wait_for_cooldown();
        let res = self.my_api.craft(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}: crafted '{}'x{}.", self.name, code, quantity);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!(
                "{}: error while crafting '{}'x{}: {}.",
                self.name, code, quantity, e
            ),
        };
        res
    }

    pub(crate) fn action_recycle(
        &self,
        code: &str,
        quantity: i32,
    ) -> Result<RecyclingResponseSchema, Error<ActionRecyclingMyNameActionRecyclingPostError>> {
        self.move_to_craft(code);
        self.wait_for_cooldown();
        let res = self.my_api.recycle(&self.name, code, quantity);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while recycling: {}.", self.name, e),
        };
        res
    }

    pub(crate) fn action_equip(
        &self,
        code: &str,
        slot: Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionEquipItemMyNameActionEquipPostError>> {
        if self.equipment_in(slot).is_some() {
            let _ = self.action_unequip(slot);
        }
        self.wait_for_cooldown();
        let res = self
            .my_api
            .equip(&self.name, code, slot.to_equip_schema(), None);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: equiped '{}' in the {:?} slot.",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while equiping: {}.", self.name, e),
        }
        res
    }

    pub(crate) fn action_unequip(
        &self,
        slot: Slot,
    ) -> Result<EquipmentResponseSchema, Error<ActionUnequipItemMyNameActionUnequipPostError>> {
        self.wait_for_cooldown();
        let res = self
            .my_api
            .unequip(&self.name, slot.to_unequip_schema(), None);
        match res {
            Ok(ref res) => {
                info!(
                    "{}: unequiped '{}' from the {:?} slot.",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while unequiping: {}.", self.name, e),
        }
        res
    }

    pub(crate) fn action_accept_task(
        &self,
    ) -> Result<TaskResponseSchema, Error<ActionAcceptNewTaskMyNameActionTaskNewPostError>> {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.accept_task(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting task: {}.", self.name, e),
        }
        res
    }

    pub(crate) fn action_complete_task(
        &self,
    ) -> Result<TaskRewardResponseSchema, Error<ActionCompleteTaskMyNameActionTaskCompletePostError>>
    {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.complete_task(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while completing task: {}.", self.name, e),
        }
        res
    }

    pub(crate) fn action_cancel_task(
        &self,
    ) -> Result<TaskCancelledResponseSchema, Error<ActionTaskCancelMyNameActionTaskCancelPostError>>
    {
        self.action_move(1, 2);
        self.wait_for_cooldown();
        let res = self.my_api.cancel_task(&self.name);
        match res {
            Ok(ref res) => {
                info!("{}", res.pretty());
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while cancelling task: {}.", self.name, e),
        }
        res
    }
}
