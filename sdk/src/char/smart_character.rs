use super::{base_character::RequestError, BaseCharacter, HasCharacterData};
use crate::{
    inventory::Inventory, maps::MapSchemaExt, monsters::MonsterSchemaExt,
    resources::ResourceSchemaExt, MAPS,
};
use artifactsmmo_openapi::models::{CharacterSchema, FightSchema, MapSchema, SkillDataSchema};
use std::sync::{Arc, RwLock};
use thiserror::Error;

pub struct SmartCharacter {
    pub id: usize,
    pub inner: BaseCharacter,
    pub inventory: Arc<Inventory>,
}

impl SmartCharacter {
    pub fn new(id: usize, data: &Arc<RwLock<CharacterSchema>>) -> Self {
        Self {
            id,
            inner: BaseCharacter::new(data),
            inventory: Arc::new(Inventory::new(data)),
        }
    }

    pub fn fight(&self) -> Result<FightSchema, FightError> {
        let map = self.map();
        let Some(monster) = map.monster() else {
            return Err(FightError::NoMonsterOnMap);
        };
        if self.inventory.free_space() < monster.max_drop_quantity() {
            return Err(FightError::InsufficientInventorySpace);
        }
        Ok(self.inner.action_fight()?)
    }

    pub fn gather(&self) -> Result<SkillDataSchema, GatherError> {
        let map = self.map();
        let Some(resource) = map.resource() else {
            return Err(GatherError::NoResourceOnMap);
        };
        if self.skill_level(resource.skill.into()) < resource.level {
            return Err(GatherError::InsufficientSkillLevel);
        }
        if self.inventory.free_space() < resource.max_drop_quantity() {
            return Err(GatherError::InsufficientInventorySpace);
        }
        Ok(self.inner.action_gather()?)
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<MapSchema, MoveError> {
        let Some(map) = MAPS.get(x, y) else {
            return Err(MoveError::MapNotFound);
        };
        Ok(self.inner.action_move(map.x, map.y)?)
    }

    pub fn rest(&self) -> Result<FightSchema, RestError> {
        todo!()
    }

    pub fn r#use(&self) -> Result<FightSchema, UseError> {
        todo!()
    }

    pub fn craft(&self) -> Result<FightSchema, CraftError> {
        todo!()
    }

    pub fn recycle(&self) -> Result<FightSchema, RecycleError> {
        todo!()
    }

    pub fn delete(&self) -> Result<FightSchema, DeleteError> {
        todo!()
    }

    pub fn withdraw(&self) -> Result<FightSchema, WithdrawError> {
        todo!()
    }

    pub fn deposit_item(&self) -> Result<FightSchema, DepositError> {
        todo!()
    }

    pub fn withdraw_gold(&self) -> Result<FightSchema, WithdrawError> {
        todo!()
    }

    pub fn deposit_gold(&self) -> Result<FightSchema, GoldDepositError> {
        todo!()
    }

    pub fn expand_bank(&self) -> Result<FightSchema, BankExpansionError> {
        todo!()
    }

    pub fn equip(&self) -> Result<FightSchema, EquipError> {
        todo!()
    }

    pub fn unequip(&self) -> Result<FightSchema, UnequipError> {
        todo!()
    }

    pub fn accept_task(&self) -> Result<FightSchema, TaskAcceptationError> {
        todo!()
    }

    pub fn task_trade(&self) -> Result<FightSchema, TaskTradeError> {
        todo!()
    }

    pub fn complete_task(&self) -> Result<FightSchema, TaskCompletionError> {
        todo!()
    }

    pub fn cancel_task(&self) -> Result<FightSchema, TaskCancellationError> {
        todo!()
    }

    pub fn exchange_tasks_coin(&self) -> Result<FightSchema, TasksCoinExchangeError> {
        todo!()
    }

    pub fn exchange_gift(&self) -> Result<FightSchema, GiftExchangeError> {
        todo!()
    }

    fn map(&self) -> Arc<MapSchema> {
        let (x, y) = self.inner.position();
        MAPS.get(x, y).unwrap()
    }
}

impl HasCharacterData for SmartCharacter {
    fn data(&self) -> Arc<RwLock<CharacterSchema>> {
        self.inner.data()
    }
}

#[derive(Debug, Error)]
pub enum FightError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No monster on map")]
    NoMonsterOnMap,
}

impl From<RequestError> for FightError {
    fn from(value: RequestError) -> Self {
        todo!()
    }
}

impl From<RequestError> for GatherError {
    fn from(value: RequestError) -> Self {
        todo!()
    }
}

impl From<RequestError> for MoveError {
    fn from(value: RequestError) -> Self {
        todo!()
    }
}

#[derive(Debug, Error)]
pub enum GatherError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No resource on map")]
    NoResourceOnMap,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel,
}

#[derive(Debug, Error)]
pub enum MoveError {
    #[error("MapNotFound")]
    MapNotFound,
}

#[derive(Debug, Error)]
pub enum RestError {}

#[derive(Debug, Error)]
pub enum UseError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not equipped")]
    ItemNotConsumable,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Insufficient character level")]
    InsufficientCharacterLevel,
}

#[derive(Debug, Error)]
pub enum CraftError {
    #[error("Craft not found")]
    CraftNotFound,
    #[error("Insufficient materials")]
    InsufficientMaterials,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Required workshop not on map")]
    NoWorkshopOnMap,
}

#[derive(Debug, Error)]
pub enum RecycleError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not recyclable")]
    ItemNotRecyclable,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel,
    #[error("Required workshop not on map")]
    NoWorkshopOnMap,
}

#[derive(Debug, Error)]
pub enum DeleteError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
}

#[derive(Debug, Error)]
pub enum WithdrawError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No bank on map")]
    NoBankOnMap,
}

#[derive(Debug, Error)]
pub enum DepositError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Insufficient bank space")]
    InsufficientBankSpace,
    #[error("No bank on map")]
    NoBankOnMap,
}

#[derive(Debug, Error)]
pub enum GoldWithdrawError {
    #[error("Insufficient gold in bank")]
    InsufficientQuantity,
    #[error("No bank on map")]
    NoBankOnMap,
}

#[derive(Debug, Error)]
pub enum GoldDepositError {
    #[error("Insufficient gold on character")]
    InsufficientQuantity,
    #[error("No bank on map")]
    NoBankOnMap,
}

#[derive(Debug, Error)]
pub enum BankExpansionError {
    #[error("No bank on map")]
    NoBankOnMap,
    #[error("Insufficient gold on character")]
    InsufficientGold,
}

#[derive(Debug, Error)]
pub enum EquipError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Item already equiped")]
    ItemAlreadyEquiped,
}

#[derive(Debug, Error)]
pub enum UnequipError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Item already equiped")]
    ItemAlreadyEquiped,
    #[error("Slot not empty")]
    SlotNotEmpty,
    #[error("Insufficient character level")]
    InsufficientCharacterLevel,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
}

#[derive(Debug, Error)]
pub enum TaskAcceptationError {
    #[error("Task already in progress")]
    TaskAlreadyInProgress,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
}

#[derive(Debug, Error)]
pub enum TaskTradeError {
    #[error("WrongTask")]
    WrongTask,
    #[error("Task already completed")]
    TaskAlreadyCompleted,
    #[error("Superfluous quantity")]
    SuperfluousQuantity,
    #[error("InsufficientQuantity")]
    InsufficientQuantity,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
}

#[derive(Debug, Error)]
pub enum TaskCompletionError {
    #[error("No current task")]
    NotCurrentTask,
    #[error("Task not fullfilled")]
    TaskNotFullfilled,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
}

#[derive(Debug, Error)]
pub enum TaskCancellationError {
    #[error("No current task")]
    NoCurrentTask,
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoin,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeError {
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoin,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
}

#[derive(Debug, Error)]
pub enum GiftExchangeError {}
