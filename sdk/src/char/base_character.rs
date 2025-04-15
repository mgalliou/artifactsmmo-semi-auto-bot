use super::{
    base_inventory::BaseInventory,
    request_handler::{CharacterRequestHandler, RequestError},
    CharacterData, HasCharacterData,
};
use crate::{
    base_bank::{BaseBank, BASE_BANK},
    gear::Slot,
    items::ItemSchemaExt,
    maps::MapSchemaExt,
    monsters::MonsterSchemaExt,
    resources::ResourceSchemaExt,
    BASE_ACCOUNT, ITEMS, MAPS,
};
use artifactsmmo_openapi::models::{
    CharacterSchema, FightSchema, MapContentType, MapSchema, RecyclingItemsSchema, RewardsSchema,
    SimpleItemSchema, SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema,
};
use derive_more::TryFrom;
use sdk_derive::FromRequestError;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use thiserror::Error;

pub static BASE_CHARACTERS: LazyLock<HashMap<usize, Arc<BaseCharacter>>> = LazyLock::new(|| {
    BASE_ACCOUNT
        .characters()
        .iter()
        .map(|(id, data)| {
            (
                *id,
                Arc::new(BaseCharacter::new(*id, data.clone(), BASE_BANK.clone())),
            )
        })
        .collect::<_>()
});

pub struct BaseCharacter {
    pub id: usize,
    inner: CharacterRequestHandler,
    pub inventory: Arc<BaseInventory>,
    bank: Arc<BaseBank>,
}

impl BaseCharacter {
    pub fn new(id: usize, data: CharacterData, bank: Arc<BaseBank>) -> Self {
        Self {
            id,
            inner: CharacterRequestHandler::new(data.clone()),
            inventory: Arc::new(BaseInventory::new(data.clone())),
            bank,
        }
    }

    pub fn fight(&self) -> Result<FightSchema, FightError> {
        self.can_fight()?;
        Ok(self.inner.request_fight()?)
    }

    pub fn can_fight(&self) -> Result<(), FightError> {
        let Some(monster) = self.map().monster() else {
            return Err(FightError::NoMonsterOnMap);
        };
        if self.inventory.free_space() < monster.max_drop_quantity() {
            return Err(FightError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn gather(&self) -> Result<SkillDataSchema, GatherError> {
        self.can_gather()?;
        Ok(self.inner.request_gather()?)
    }

    pub fn can_gather(&self) -> Result<(), GatherError> {
        let Some(resource) = self.map().resource() else {
            return Err(GatherError::NoResourceOnMap);
        };
        if self.skill_level(resource.skill.into()) < resource.level {
            return Err(GatherError::SkillLevelInsufficient);
        }
        if self.inventory.free_space() < resource.max_drop_quantity() {
            return Err(GatherError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<MapSchema, MoveError> {
        self.can_move(x, y)?;
        Ok(self.inner.request_move(x, y)?)
    }

    pub fn can_move(&self, x: i32, y: i32) -> Result<(), MoveError> {
        if MAPS.get(x, y).is_none() {
            return Err(MoveError::MapNotFound);
        }
        Ok(())
    }

    pub fn rest(&self) -> Result<i32, RestError> {
        if self.health() < self.max_health() {
            return Ok(self.inner.request_rest()?);
        }
        Ok(0)
    }

    pub fn r#use(&self, item_code: &str, quantity: i32) -> Result<(), UseError> {
        self.can_use(item_code, quantity)?;
        Ok(self.inner.request_use_item(item_code, quantity)?)
    }

    pub fn can_use(&self, item_code: &str, quantity: i32) -> Result<(), UseError> {
        let Some(item) = ITEMS.get(item_code) else {
            return Err(UseError::ItemNotFound);
        };
        if !item.is_consumable() {
            return Err(UseError::ItemNotConsumable);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(UseError::InsufficientQuantity);
        }
        if self.level() < item.level {
            return Err(UseError::InsufficientCharacterLevel);
        }
        Ok(())
    }

    pub fn craft(&self, item_code: &str, quantity: i32) -> Result<SkillInfoSchema, CraftError> {
        self.can_craft(item_code, quantity)?;
        Ok(self.inner.request_craft(item_code, quantity)?)
    }

    pub fn can_craft(&self, item_code: &str, quantity: i32) -> Result<(), CraftError> {
        let Some(item) = ITEMS.get(item_code) else {
            return Err(CraftError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(CraftError::ItemNotCraftable);
        };
        if self.skill_level(skill) < item.level {
            return Err(CraftError::InsufficientSkillLevel);
        }
        if !self.inventory.contains_mats_for(item_code, quantity) {
            return Err(CraftError::InsufficientMaterials);
        }
        // TODO: check if InssuficientInventorySpace can happen
        if !self.map().content_code_is(skill.as_ref()) {
            return Err(CraftError::NoWorkshopOnMap);
        }
        Ok(())
    }

    pub fn recycle(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, RecycleError> {
        self.can_recycle(item_code, quantity)?;
        Ok(self.inner.request_recycle(item_code, quantity)?)
    }

    pub fn can_recycle(&self, item_code: &str, quantity: i32) -> Result<(), RecycleError> {
        let Some(item) = ITEMS.get(item_code) else {
            return Err(RecycleError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(RecycleError::ItemNotRecyclable);
        };
        if skill.is_cooking() || skill.is_alchemy() {
            return Err(RecycleError::ItemNotRecyclable);
        }
        if self.skill_level(skill) < item.level {
            return Err(RecycleError::InsufficientSkillLevel);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(RecycleError::InsufficientQuantity);
        }
        if self.inventory.free_space() + quantity < item.recycled_quantity() {
            return Err(RecycleError::InsufficientInventorySpace);
        }
        if !self.map().content_code_is(skill.as_ref()) {
            return Err(RecycleError::NoWorkshopOnMap);
        }
        Ok(())
    }

    pub fn delete(&self, item_code: &str, quantity: i32) -> Result<SimpleItemSchema, DeleteError> {
        self.can_delete(item_code, quantity)?;
        Ok(self.inner.request_delete(item_code, quantity)?)
    }

    pub fn can_delete(&self, item_code: &str, quantity: i32) -> Result<(), DeleteError> {
        if ITEMS.get(item_code).is_none() {
            return Err(DeleteError::ItemNotFound);
        };
        if self.inventory.total_of(item_code) < quantity {
            return Err(DeleteError::InsufficientQuantity);
        }
        Ok(())
    }

    pub fn withdraw(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, WithdrawError> {
        self.can_withdraw(item_code, quantity)?;
        Ok(self.inner.request_withdraw(item_code, quantity)?)
    }

    pub fn can_withdraw(&self, item_code: &str, quantity: i32) -> Result<(), WithdrawError> {
        if ITEMS.get(item_code).is_none() {
            return Err(WithdrawError::ItemNotFound);
        };
        if self.bank.total_of(item_code) < quantity {
            return Err(WithdrawError::InsufficientQuantity);
        }
        if self.inventory.free_space() < quantity {
            return Err(WithdrawError::InsufficientInventorySpace);
        }
        if !self.map().content_type_is(MapContentType::Bank) {
            return Err(WithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn deposit(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, DepositError> {
        self.can_deposit(item_code, quantity)?;
        Ok(self.inner.request_deposit(item_code, quantity)?)
    }

    fn can_deposit(&self, item_code: &str, quantity: i32) -> Result<(), DepositError> {
        if ITEMS.get(item_code).is_none() {
            return Err(DepositError::ItemNotFound);
        };
        if self.inventory.total_of(item_code) < quantity {
            return Err(DepositError::InsufficientQuantity);
        }
        if self.bank.total_of(item_code) <= 0 && self.bank.free_slots() <= 0 {
            return Err(DepositError::InsufficientBankSpace);
        }
        if !self.map().content_type_is(MapContentType::Bank) {
            return Err(DepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn withdraw_gold(&self, quantity: i32) -> Result<i32, GoldWithdrawError> {
        self.can_withdraw_gold(quantity)?;
        Ok(self.inner.request_withdraw_gold(quantity)?)
    }

    pub fn can_withdraw_gold(&self, quantity: i32) -> Result<(), GoldWithdrawError> {
        if self.bank.gold() < quantity {
            return Err(GoldWithdrawError::InsufficientGold);
        }
        if !self.map().content_type_is(MapContentType::Bank) {
            return Err(GoldWithdrawError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn deposit_gold(&self, quantity: i32) -> Result<i32, GoldDepositError> {
        self.can_deposit_gold(quantity)?;
        Ok(self.inner.request_deposit_gold(quantity)?)
    }

    pub fn can_deposit_gold(&self, quantity: i32) -> Result<(), GoldDepositError> {
        if self.gold() < quantity {
            return Err(GoldDepositError::InsufficientGold);
        }
        if !self.map().content_type_is(MapContentType::Bank) {
            return Err(GoldDepositError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn expand_bank(&self) -> Result<i32, BankExpansionError> {
        self.can_expand_bank()?;
        Ok(self.inner.request_expand_bank()?)
    }

    pub fn can_expand_bank(&self) -> Result<(), BankExpansionError> {
        if self.gold() < self.bank.details().next_expansion_cost {
            return Err(BankExpansionError::InsufficientGold);
        }
        if !self.map().content_type_is(MapContentType::Bank) {
            return Err(BankExpansionError::NoBankOnMap);
        }
        Ok(())
    }

    pub fn equip(&self, item_code: &str, slot: Slot, quantity: i32) -> Result<(), EquipError> {
        self.can_equip(item_code, slot, quantity)?;
        Ok(self.inner.request_equip(item_code, slot, quantity)?)
    }

    pub fn can_equip(&self, item_code: &str, slot: Slot, quantity: i32) -> Result<(), EquipError> {
        let Some(item) = ITEMS.get(item_code) else {
            return Err(EquipError::ItemNotFound);
        };
        if self.inventory.total_of(item_code) < quantity {
            return Err(EquipError::InsufficientQuantity);
        }
        if let Some(equiped) = self.gear().slot(slot) {
            if equiped.code == item_code {
                if slot.max_quantity() <= 1 {
                    return Err(EquipError::ItemAlreadyEquiped);
                } else if self.quantity_in_slot(slot) + quantity > slot.max_quantity() {
                    return Err(EquipError::QuantityGreaterThanSlotMaxixum);
                }
            } else {
                return Err(EquipError::SlotNotEmpty);
            }
        }
        if self.level() < item.level {
            return Err(EquipError::InsufficientCharacterLevel);
        }
        if self.inventory.free_space() + item.inventory_space() <= 0 {
            return Err(EquipError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn unequip(&self, slot: Slot, quantity: i32) -> Result<(), UnequipError> {
        self.can_unequip(slot, quantity)?;
        Ok(self.inner.request_unequip(slot, quantity)?)
    }

    pub fn can_unequip(&self, slot: Slot, quantity: i32) -> Result<(), UnequipError> {
        if self.gear().slot(slot).is_none() {
            return Err(UnequipError::SlotEmpty);
        }
        if self.quantity_in_slot(slot) < quantity {
            return Err(UnequipError::InsufficientQuantity);
        }
        if self.inventory.free_space() < quantity {
            return Err(UnequipError::InsufficientInventorySpace);
        }
        Ok(())
    }

    pub fn accept_task(&self) -> Result<TaskSchema, TaskAcceptationError> {
        self.can_accept_task()?;
        Ok(self.inner.request_accept_task()?)
    }

    pub fn can_accept_task(&self) -> Result<(), TaskAcceptationError> {
        if !self.task().is_empty() {
            return Err(TaskAcceptationError::TaskAlreadyInProgress);
        }
        if !self.map().content_type_is(MapContentType::TasksMaster) {
            return Err(TaskAcceptationError::NoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn task_trade(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<TaskTradeSchema, TaskTradeError> {
        self.can_task_trade(item_code, quantity)?;
        Ok(self.inner.request_task_trade(item_code, quantity)?)
    }

    pub fn can_task_trade(&self, item_code: &str, quantity: i32) -> Result<(), TaskTradeError> {
        if ITEMS.get(item_code).is_none() {
            return Err(TaskTradeError::ItemNotFound);
        };
        if item_code != self.task() {
            return Err(TaskTradeError::WrongTask);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(TaskTradeError::InsufficientQuantity);
        }
        if self.task_missing() < quantity {
            return Err(TaskTradeError::SuperfluousQuantity);
        }
        if !self.map().content_type_is(MapContentType::TasksMaster)
            || !self.map().content_code_is("items")
        {
            return Err(TaskTradeError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn complete_task(&self) -> Result<RewardsSchema, TaskCompletionError> {
        self.can_complete_task()?;
        Ok(self.inner.request_complete_task()?)
    }

    pub fn can_complete_task(&self) -> Result<(), TaskCompletionError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCompletionError::NoCurrentTask);
        };
        if !self.task_finished() {
            return Err(TaskCompletionError::TaskNotFullfilled);
        }
        if self.inventory.free_space() < 2 {
            return Err(TaskCompletionError::InsufficientInventorySpace);
        }
        if !self.map().content_type_is(MapContentType::TasksMaster)
            || !self.map().content_code_is(&task_type.to_string())
        {
            return Err(TaskCompletionError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn cancel_task(&self) -> Result<(), TaskCancellationError> {
        self.can_cancel_task()?;
        Ok(self.inner.request_cancel_task()?)
    }

    pub fn can_cancel_task(&self) -> Result<(), TaskCancellationError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCancellationError::NoCurrentTask);
        };
        if self.inventory.total_of("tasks_coin") < 1 {
            return Err(TaskCancellationError::InsufficientTasksCoinQuantity);
        }
        if !self.map().content_type_is(MapContentType::TasksMaster)
            || !self.map().content_code_is(&task_type.to_string())
        {
            return Err(TaskCancellationError::WrongOrNoTasksMasterOnMap);
        }
        Ok(())
    }

    pub fn exchange_tasks_coin(&self) -> Result<RewardsSchema, TasksCoinExchangeError> {
        self.can_exchange_tasks_coin()?;
        Ok(self.inner.request_task_exchange()?)
    }

    pub fn can_exchange_tasks_coin(&self) -> Result<(), TasksCoinExchangeError> {
        if self.inventory.total_of("tasks_coin") < 6 {
            return Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity);
        }
        if !self.map().content_type_is(MapContentType::TasksMaster) {
            return Err(TasksCoinExchangeError::NoTasksMasterOnMap);
        }
        // TODO: check for conditions when InsufficientInventorySpace can happen
        Ok(())
    }

    //pub fn exchange_gift(&self) -> Result<RewardsSchema, GiftExchangeError> {
    //    self.can_exchange_gift()?;
    //    Ok(self.inner.request_gift_exchange()?)
    //}

    // pub fn can_exchange_gift(&self) -> Result<(), GiftExchangeError> {
    //     if self.inventory.total_of("tasks_coin") < 1 {
    //         return Err(GiftExchangeError::InsufficientGiftQuantity);
    //     }
    //     if !self.map().content_type_is(MapContentType::SantaClaus) {
    //         return Err(GiftExchangeError::NoSantaClausOnMap);
    //     }
    //     // TODO: check for conditions when InsufficientInventorySpace can happen
    //     Ok(())
    // }
}

impl HasCharacterData for BaseCharacter {
    fn data(&self) -> Arc<CharacterSchema> {
        self.inner.data()
    }
}

const ENTITY_NOT_FOUND: isize = 404;
const BANK_GOLD_INSUFFICIENT: isize = 460;
//const TRANSACTION_ALREADY_IN_PROGRESS: isize = 461;
const BANK_FULL: isize = 462;
const ITEM_NOT_RECYCLABLE: isize = 473;
const WRONG_TASK: isize = 474;
const TASK_ALREADY_COMPLETED_OR_TOO_MANY_ITEM_TRADED: isize = 475;
const ITEM_NOT_CONSUMABLE: isize = 476;
const MISSING_ITEM_OR_INSUFFICIENT_QUANTITY: isize = 478;
const SUPERFLOUS_UTILITY_QUANTITY: isize = 484;
const ITEM_ALREADY_EQUIPED: isize = 485;
//const ACTION_ALREADY_IN_PROGRESS: isize = 486;
const NO_TASK: isize = 487;
const TASK_NOT_COMPLETED: isize = 488;
const TASK_ALREADY_IN_PROGRESS: isize = 489;
const INVALID_SLOT_STATE: isize = 491;
const CHARACTER_GOLD_INSUFFICIENT: isize = 492;
const SKILL_LEVEL_INSUFFICIENT: isize = 493;
const CHARACTER_LEVEL_INSUFFICIENT: isize = 496;
const INVENTORY_FULL: isize = 497;
//const CHARACTER_NOT_FOUND: isize = 498;
//const CHARACTER_ON_COOLDOWN: isize = 499;
const ENTITY_NOT_FOUND_ON_MAP: isize = 598;

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum FightError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No monster on map")]
    NoMonsterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum GatherError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No resource on map")]
    NoResourceOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error("Insufficient skill level")]
    SkillLevelInsufficient = SKILL_LEVEL_INSUFFICIENT,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum MoveError {
    #[error("MapNotFound")]
    MapNotFound = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum RestError {
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum UseError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Item not equipped")]
    ItemNotConsumable = ITEM_NOT_CONSUMABLE,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient character level")]
    InsufficientCharacterLevel = CHARACTER_LEVEL_INSUFFICIENT,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum CraftError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not craftable")]
    ItemNotCraftable = ENTITY_NOT_FOUND,
    #[error("Insufficient materials")]
    InsufficientMaterials = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel = SKILL_LEVEL_INSUFFICIENT,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Required workshop not on map")]
    NoWorkshopOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum RecycleError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Item not recyclable")]
    ItemNotRecyclable = ITEM_NOT_RECYCLABLE,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel = SKILL_LEVEL_INSUFFICIENT,
    #[error("Required workshop not on map")]
    NoWorkshopOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum DeleteError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum WithdrawError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No bank on map")]
    NoBankOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum DepositError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient bank space")]
    InsufficientBankSpace = BANK_FULL,
    #[error("No bank on map")]
    NoBankOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum GoldWithdrawError {
    #[error("Insufficient gold in bank")]
    InsufficientGold = BANK_GOLD_INSUFFICIENT,
    #[error("No bank on map")]
    NoBankOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum GoldDepositError {
    #[error("Insufficient gold on character")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("No bank on map")]
    NoBankOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum BankExpansionError {
    #[error("Insufficient gold on character")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("No bank on map")]
    NoBankOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum EquipError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Item already equiped")]
    ItemAlreadyEquiped = ITEM_ALREADY_EQUIPED,
    #[error("Quantity greater than slot max quantity")]
    QuantityGreaterThanSlotMaxixum = SUPERFLOUS_UTILITY_QUANTITY,
    #[error("Slot not empty")]
    SlotNotEmpty = INVALID_SLOT_STATE,
    #[error("Insufficient character level")]
    InsufficientCharacterLevel = CHARACTER_LEVEL_INSUFFICIENT,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum UnequipError {
    #[error("Slot is empty")]
    SlotEmpty = INVALID_SLOT_STATE,
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum TaskAcceptationError {
    #[error("Task already in progress")]
    TaskAlreadyInProgress = TASK_ALREADY_IN_PROGRESS,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum TaskTradeError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("WrongTask")]
    WrongTask = WRONG_TASK,
    #[error("Superfluous quantity")]
    SuperfluousQuantity = TASK_ALREADY_COMPLETED_OR_TOO_MANY_ITEM_TRADED,
    #[error("InsufficientQuantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Wrong or no tasks master on map")]
    WrongOrNoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum TaskCompletionError {
    #[error("No current task")]
    NoCurrentTask = NO_TASK,
    #[error("Task not fullfilled")]
    TaskNotFullfilled = TASK_NOT_COMPLETED,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Wrong or no tasks master on map")]
    WrongOrNoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum TaskCancellationError {
    #[error("No current task")]
    NoCurrentTask = NO_TASK,
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoinQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Wrong or no tasks master on map")]
    WrongOrNoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum TasksCoinExchangeError {
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoinQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[derive(Debug, Error, TryFrom, FromRequestError)]
#[try_from(repr)]
#[repr(isize)]
pub enum GiftExchangeError {
    #[error("Insufficient gift quantity")]
    InsufficientGiftQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No Santa Claus on map")]
    NoSantaClausOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use artifactsmmo_openapi::models::InventorySlot;
    use std::sync::RwLock;

    impl From<CharacterSchema> for BaseCharacter {
        fn from(value: CharacterSchema) -> Self {
            Self::new(
                1,
                Arc::new(RwLock::new(Arc::new(value))),
                Arc::new(BaseBank::default()),
            )
        }
    }

    #[test]
    fn can_fight() {
        // monster on 0,2 is "cow"
        let char = BaseCharacter::from(CharacterSchema {
            x: 0,
            y: 2,
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(char.can_fight().is_ok());
        let char = BaseCharacter::from(CharacterSchema {
            x: 0,
            y: 2,
            inventory_max_items: &char.map().monster().unwrap().max_drop_quantity() - 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_fight(),
            Err(FightError::InsufficientInventorySpace)
        ));
    }

    #[test]
    fn can_gather() {
        let char = BaseCharacter::from(CharacterSchema {
            x: 2,
            y: 0,
            mining_level: 1,
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(char.can_gather().is_ok());
        let char = BaseCharacter::from(CharacterSchema {
            x: 0,
            y: 0,
            mining_level: 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_gather(),
            Err(GatherError::NoResourceOnMap)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            x: 1,
            y: 7,
            mining_level: 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_gather(),
            Err(GatherError::SkillLevelInsufficient)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            x: 2,
            y: 0,
            mining_level: 1,
            inventory_max_items: MAPS
                .get(2, 0)
                .unwrap()
                .resource()
                .unwrap()
                .max_drop_quantity()
                - 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_gather(),
            Err(GatherError::InsufficientInventorySpace)
        ));
    }

    #[test]
    fn can_move() {
        let char = BaseCharacter::from(CharacterSchema::default());
        assert!(char.can_move(0, 0).is_ok());
        assert!(matches!(
            char.can_move(1000, 0),
            Err(MoveError::MapNotFound)
        ));
    }

    #[test]
    fn can_use() {
        let item1 = "cooked_chicken";
        let item2 = "cooked_shrimp";
        let char = BaseCharacter::from(CharacterSchema {
            level: 5,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: item1.to_owned(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: item2.to_owned(),
                    quantity: 1,
                },
            ]),
            ..Default::default()
        });
        assert!(matches!(
            char.can_use("random_item", 1),
            Err(UseError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_use("copper", 1),
            Err(UseError::ItemNotConsumable)
        ));
        assert!(matches!(
            char.can_use(item1, 5),
            Err(UseError::InsufficientQuantity)
        ));
        assert!(matches!(
            char.can_use(item2, 1),
            Err(UseError::InsufficientCharacterLevel)
        ));
        assert!(char.can_use(item1, 1).is_ok());
    }

    #[test]
    fn can_craft() {
        let char = BaseCharacter::from(CharacterSchema {
            cooking_level: 1,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: "gudgeon".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: "shrimp".to_string(),
                    quantity: 1,
                },
            ]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_craft("random_item", 1),
            Err(CraftError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_craft("copper_ore", 1),
            Err(CraftError::ItemNotCraftable)
        ));
        assert!(matches!(
            char.can_craft("cooked_chicken", 1),
            Err(CraftError::InsufficientMaterials)
        ));
        assert!(matches!(
            char.can_craft("cooked_gudgeon", 5),
            Err(CraftError::InsufficientMaterials)
        ));
        assert!(matches!(
            char.can_craft("cooked_shrimp", 1),
            Err(CraftError::InsufficientSkillLevel)
        ));
        assert!(matches!(
            char.can_craft("cooked_gudgeon", 1),
            Err(CraftError::NoWorkshopOnMap)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            cooking_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "gudgeon".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            x: 1,
            y: 1,
            ..Default::default()
        });
        assert!(char.can_craft("cooked_gudgeon", 1).is_ok());
    }

    #[test]
    fn can_recycle() {
        let char = BaseCharacter::from(CharacterSchema {
            cooking_level: 1,
            weaponcrafting_level: 1,
            inventory: Some(vec![
                InventorySlot {
                    slot: 0,
                    code: "copper_dagger".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 1,
                    code: "iron_sword".to_string(),
                    quantity: 1,
                },
                InventorySlot {
                    slot: 2,
                    code: "cooked_gudgeon".to_string(),
                    quantity: 1,
                },
            ]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_recycle("random_item", 1),
            Err(RecycleError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_recycle("cooked_gudgeon", 1),
            Err(RecycleError::ItemNotRecyclable)
        ));
        assert!(matches!(
            char.can_recycle("wooden_staff", 1),
            Err(RecycleError::InsufficientQuantity)
        ));
        assert!(matches!(
            char.can_recycle("iron_sword", 1),
            Err(RecycleError::InsufficientSkillLevel)
        ));
        assert!(matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::NoWorkshopOnMap)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 1,
            x: 2,
            y: 1,
            ..Default::default()
        });
        assert!(matches!(
            char.can_recycle("copper_dagger", 1),
            Err(RecycleError::InsufficientInventorySpace)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            x: 2,
            y: 1,
            ..Default::default()
        });
        assert!(char.can_recycle("copper_dagger", 1).is_ok());
    }

    #[test]
    fn can_delete() {
        let char = BaseCharacter::from(CharacterSchema {
            cooking_level: 1,
            weaponcrafting_level: 1,
            inventory: Some(vec![InventorySlot {
                slot: 0,
                code: "copper_dagger".to_string(),
                quantity: 1,
            }]),
            inventory_max_items: 100,
            ..Default::default()
        });
        assert!(matches!(
            char.can_delete("random_item", 1),
            Err(DeleteError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_delete("copper_dagger", 2),
            Err(DeleteError::InsufficientQuantity)
        ));
        assert!(char.can_delete("copper_dagger", 1).is_ok());
    }

    #[test]
    fn can_withdraw() {
        let char = BaseCharacter::from(CharacterSchema {
            inventory_max_items: 100,
            ..Default::default()
        });
        char.bank.update_content(vec![
            SimpleItemSchema {
                code: "copper_dagger".to_string(),
                quantity: 1,
            },
            SimpleItemSchema {
                code: "iron_sword".to_string(),
                quantity: 101,
            },
        ]);
        assert!(matches!(
            char.can_withdraw("random_item", 1),
            Err(WithdrawError::ItemNotFound)
        ));
        assert!(matches!(
            char.can_withdraw("copper_dagger", 2),
            Err(WithdrawError::InsufficientQuantity)
        ));
        assert!(matches!(
            char.can_withdraw("iron_sword", 101),
            Err(WithdrawError::InsufficientInventorySpace)
        ));
        assert!(matches!(
            char.can_withdraw("iron_sword", 10),
            Err(WithdrawError::NoBankOnMap)
        ));
        let char = BaseCharacter::from(CharacterSchema {
            inventory_max_items: 100,
            x: 4,
            y: 1,
            ..Default::default()
        });
        char.bank.update_content(vec![
            SimpleItemSchema {
                code: "copper_dagger".to_string(),
                quantity: 1,
            },
            SimpleItemSchema {
                code: "iron_sword".to_string(),
                quantity: 101,
            },
        ]);
        assert!(char.can_withdraw("iron_sword", 10).is_ok());
    }
    //TODO: add more tests
}
