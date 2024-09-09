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
        CharacterMovementResponseSchema, EquipmentResponseSchema, RecyclingResponseSchema,
        SkillResponseSchema, TaskCancelledResponseSchema, TaskResponseSchema,
        TaskRewardResponseSchema,
    },
};
use log::{error, info};

impl ResponseSchema for CharacterMovementResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{} moved to {}",
            self.data.character.name,
            self.data.destination.pretty()
        )
    }
}

impl ResponseSchema for CharacterFightResponseSchema {
    fn pretty(&self) -> String {
        match self.data.fight.result {
            fight_schema::Result::Win => format!(
                "{} win his fight after {} turns ({}xp, {}g, {:#?}).",
                self.data.character.name,
                self.data.fight.turns,
                self.data.fight.xp,
                self.data.fight.gold,
                self.data.fight.drops
            ),
            fight_schema::Result::Lose => format!(
                "{} loose his fight after {} turns.",
                self.data.character.name, self.data.fight.turns
            ),
        }
    }
}

impl Character {
    pub(crate) fn action_move(&self, x: i32, y: i32) -> bool {
        if (self.data().x, self.data().y) == (x, y) {
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
            Err(ref e) => error!("{}: error while moving: {}", self.name, e),
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
            Err(ref e) => error!("{}: error during fight: {}", self.name, e),
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
                info!("{}: gathered: {:#?}", self.name, res.data.details);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during gathering: {}", self.name, e),
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
                info!("{}: withdrawed {} {}", self.name, code, quantity);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => error!(
                "{}: error while withdrawing {} * {}: {}",
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
                info!("{}: deposited {} * {}", self.name, code, quantity);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
                let _ = self
                    .bank
                    .write()
                    .map(|mut bank| bank.content = res.data.bank.clone());
            }
            Err(ref e) => error!(
                "{}: error while depositing {} * {}: {}",
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
                info!("{}: crafted {}, {}", self.name, quantity, code);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during crafting: {}", self.name, e),
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
                info!("{}: recycled {}, {}", self.name, quantity, code);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error during crafting: {}", self.name, e),
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
                    "{}: equiped {} in {:#?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while unequiping: {}", self.name, e),
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
                    "{}: unequiped {} from {:#?} slot",
                    self.name, res.data.item.code, res.data.slot
                );
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while unequiping: {}", self.name, e),
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
                info!("{}: accepted new task: {:#?}", self.name, res.data.task);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
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
                error!("{}: completed task: {:#?}", self.name, res.data.reward);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
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
                info!("{}: canceled task: {:#?}", self.name, self.data().task);
                self.data
                    .write()
                    .unwrap()
                    .clone_from(res.data.character.as_ref());
            }
            Err(ref e) => error!("{}: error while accepting: {}", self.name, e),
        }
        res
    }
}
