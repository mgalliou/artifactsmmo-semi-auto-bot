use super::{
    base_inventory::BaseInventory,
    request_handler::{CharacterRequestHandler, RequestError},
    CharacterData, HasCharacterData,
};
use crate::{
    gear::Slot,
    items::ItemSchemaExt,
    maps::{ContentType, MapSchemaExt},
    monsters::MonsterSchemaExt,
    resources::ResourceSchemaExt,
    BANK, ITEMS, MAPS,
};
use artifactsmmo_openapi::models::{
    FightSchema, MapSchema, RecyclingItemsSchema, RewardsSchema, SimpleItemSchema, SkillDataSchema,
    SkillInfoSchema, TaskSchema, TaskTradeSchema,
};
use std::sync::Arc;
use thiserror::Error;

pub struct BaseCharacter {
    pub id: usize,
    pub inner: CharacterRequestHandler,
    pub inventory: Arc<BaseInventory>,
}

impl BaseCharacter {
    pub fn new(id: usize, data: &CharacterData) -> Self {
        Self {
            id,
            inner: CharacterRequestHandler::new(data),
            inventory: Arc::new(BaseInventory::new(data)),
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
        Ok(self.inner.request_fight()?)
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
        Ok(self.inner.request_gather()?)
    }

    pub fn r#move(&self, x: i32, y: i32) -> Result<MapSchema, MoveError> {
        let Some(map) = MAPS.get(x, y) else {
            return Err(MoveError::MapNotFound);
        };
        Ok(self.inner.request_move(map.x, map.y)?)
    }

    pub fn rest(&self) -> Result<(), RestError> {
        if self.health() < self.max_health() {
            self.inner.request_rest()?;
        }
        Ok(())
    }

    pub fn r#use(&self, item_code: &str, quantity: i32) -> Result<(), UseError> {
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
        Ok(self.inner.request_use_item(item_code, quantity)?)
    }

    pub fn craft(&self, item_code: &str, quantity: i32) -> Result<SkillInfoSchema, CraftError> {
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
        Ok(self.inner.request_craft(item_code, quantity)?)
    }

    pub fn recycle(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<RecyclingItemsSchema, RecycleError> {
        let Some(item) = ITEMS.get(item_code) else {
            return Err(RecycleError::ItemNotFound);
        };
        let Some(skill) = item.skill_to_craft() else {
            return Err(RecycleError::ItemNotRecyclable);
        };
        if self.skill_level(skill) < item.level {
            return Err(RecycleError::InsufficientSkillLevel);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(RecycleError::InsufficientQuantity);
        }
        if self.inventory.free_space() < item.recycled_quantity() {
            return Err(RecycleError::InsufficientInventorySpace);
        }
        if !self.map().content_code_is(skill.as_ref()) {
            return Err(RecycleError::NoWorkshopOnMap);
        }
        Ok(self.inner.request_recycle(item_code, quantity)?)
    }

    pub fn delete(&self, item_code: &str, quantity: i32) -> Result<SimpleItemSchema, DeleteError> {
        if ITEMS.get(item_code).is_none() {
            return Err(DeleteError::ItemNotFound);
        };
        if self.inventory.total_of(item_code) < quantity {
            return Err(DeleteError::InsufficientQuantity);
        }
        Ok(self.inner.request_delete(item_code, quantity)?)
    }

    pub fn withdraw(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, WithdrawError> {
        if ITEMS.get(item_code).is_none() {
            return Err(WithdrawError::ItemNotFound);
        };
        if BANK.total_of(item_code) < quantity {
            return Err(WithdrawError::InsufficientQuantity);
        }
        if self.inventory.free_space() < quantity {
            return Err(WithdrawError::InsufficientInventorySpace);
        }
        if !self.map().content_type_is(ContentType::Bank) {
            return Err(WithdrawError::NoBankOnMap);
        }
        Ok(self.inner.request_withdraw(item_code, quantity)?)
    }

    pub fn deposit_item(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<SimpleItemSchema, DepositError> {
        if ITEMS.get(item_code).is_none() {
            return Err(DepositError::ItemNotFound);
        };
        if self.inventory.total_of(item_code) < quantity {
            return Err(DepositError::InsufficientQuantity);
        }
        if BANK.total_of(item_code) <= 0 && BANK.free_slots() <= 0 {
            return Err(DepositError::InsufficientBankSpace);
        }
        if !self.map().content_type_is(ContentType::Bank) {
            return Err(DepositError::NoBankOnMap);
        }
        Ok(self.inner.request_deposit(item_code, quantity)?)
    }

    pub fn withdraw_gold(&self, quantity: i32) -> Result<i32, GoldWithdrawError> {
        if BANK.gold() < quantity {
            return Err(GoldWithdrawError::InsufficientGold);
        }
        if !self.map().content_type_is(ContentType::Bank) {
            return Err(GoldWithdrawError::NoBankOnMap);
        }
        Ok(self.inner.request_withdraw_gold(quantity)?)
    }

    pub fn deposit_gold(&self, quantity: i32) -> Result<i32, GoldDepositError> {
        if self.gold() < quantity {
            return Err(GoldDepositError::InsufficientGold);
        }
        if !self.map().content_type_is(ContentType::Bank) {
            return Err(GoldDepositError::NoBankOnMap);
        }
        Ok(self.inner.request_deposit_gold(quantity)?)
    }

    pub fn expand_bank(&self) -> Result<i32, BankExpansionError> {
        if self.gold() < BANK.details().next_expansion_cost {
            return Err(BankExpansionError::InsufficientGold);
        }
        if !self.map().content_type_is(ContentType::Bank) {
            return Err(BankExpansionError::NoBankOnMap);
        }
        Ok(self.inner.request_expand_bank()?)
    }

    pub fn equip(&self, item_code: &str, slot: Slot, quantity: i32) -> Result<(), EquipError> {
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
        Ok(self.inner.request_equip(item_code, slot, quantity)?)
    }

    pub fn unequip(&self, slot: Slot, quantity: i32) -> Result<(), UnequipError> {
        if self.gear().slot(slot).is_none() {
            return Err(UnequipError::SlotEmpty);
        }
        if self.quantity_in_slot(slot) < quantity {
            return Err(UnequipError::InsufficientQuantity);
        }
        if self.inventory.free_space() < quantity {
            return Err(UnequipError::InsufficientInventorySpace);
        }
        Ok(self.inner.request_unequip(slot, quantity)?)
    }

    pub fn accept_task(&self) -> Result<TaskSchema, TaskAcceptationError> {
        if !self.task().is_empty() {
            return Err(TaskAcceptationError::TaskAlreadyInProgress);
        }
        if !self.map().content_type_is(ContentType::TasksMaster) {
            return Err(TaskAcceptationError::NoTasksMasterOnMap);
        }
        Ok(self.inner.request_accept_task()?)
    }

    pub fn task_trade(
        &self,
        item_code: &str,
        quantity: i32,
    ) -> Result<TaskTradeSchema, TaskTradeError> {
        if ITEMS.get(item_code).is_none() {
            return Err(TaskTradeError::ItemNotFound);
        };
        if self.task_finished() {
            return Err(TaskTradeError::TaskAlreadyCompleted);
        }
        if item_code != self.task() {
            return Err(TaskTradeError::WrongTask);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(TaskTradeError::InsufficientQuantity);
        }
        if self.task_missing() < quantity {
            return Err(TaskTradeError::SuperfluousQuantity);
        }
        if !self.map().content_type_is(ContentType::TasksMaster) {
            return Err(TaskTradeError::NoTasksMasterOnMap);
        } else if !self.map().content_code_is("items") {
            return Err(TaskTradeError::WrongTasksMaster);
        }
        Ok(self.inner.request_task_trade(item_code, quantity)?)
    }

    pub fn complete_task(&self) -> Result<RewardsSchema, TaskCompletionError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCompletionError::NoCurrentTask);
        };
        if !self.task_finished() {
            return Err(TaskCompletionError::TaskNotFullfilled);
        }
        if self.inventory.free_space() < 2 {
            return Err(TaskCompletionError::InsufficientInventorySpace);
        }
        if !self.map().content_type_is(ContentType::TasksMaster) {
            return Err(TaskCompletionError::NoTasksMasterOnMap);
        } else if !self.map().content_code_is(&task_type.to_string()) {
            return Err(TaskCompletionError::WrongTasksMaster);
        }
        Ok(self.inner.request_complete_task()?)
    }

    pub fn cancel_task(&self) -> Result<(), TaskCancellationError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCancellationError::NoCurrentTask);
        };
        if self.inventory.total_of("tasks_coin") < 1 {
            return Err(TaskCancellationError::InsufficientTasksCoin);
        }
        if !self.map().content_type_is(ContentType::TasksMaster) {
            return Err(TaskCancellationError::NoTasksMasterOnMap);
        } else if !self.map().content_code_is(&task_type.to_string()) {
            return Err(TaskCancellationError::WrongTasksMaster);
        }
        Ok(self.inner.request_cancel_task()?)
    }

    pub fn exchange_tasks_coin(&self) -> Result<RewardsSchema, TasksCoinExchangeError> {
        if self.inventory.total_of("tasks_coin") < 6 {
            return Err(TasksCoinExchangeError::InsufficientTasksCoinQuantity);
        }
        if !self.map().content_type_is(ContentType::TasksMaster) {
            return Err(TasksCoinExchangeError::NoTasksMasterOnMap);
        }
        // TODO: check for conditions when InsufficientInventorySpace can happen
        Ok(self.inner.request_task_exchange()?)
    }

    pub fn exchange_gift(&self) -> Result<RewardsSchema, GiftExchangeError> {
        if self.inventory.total_of("tasks_coin") < 1 {
            return Err(GiftExchangeError::InsufficientGiftQuantity);
        }
        if !self.map().content_type_is(ContentType::SantaClaus) {
            return Err(GiftExchangeError::NoSantaClausOnMap);
        }
        // TODO: check for conditions when InsufficientInventorySpace can happen
        Ok(self.inner.request_gift_exchange()?)
    }
}

impl HasCharacterData for BaseCharacter {
    fn data(&self) -> CharacterData {
        self.inner.data()
    }
}

const ENTITY_NOT_FOUND: isize = 404;
const INSUFFICIENT_GOLD_IN_BANK: isize = 460;
const BANK_FULL: isize = 462;
const ITEM_NOT_RECYCLABLE: isize = 473;
const WRONG_TASK: isize = 474;
const TASK_ALREADY_COMPLETED_OR_TOO_MANY_ITEM_TRADED: isize = 475;
const ITEM_NOT_CONSUMABLE: isize = 476;
const MISSING_OR_INSUFFICIENT_QUANTITY: isize = 478;
const SUPERFLOUS_UTILITY_QUANTITY: isize = 484;
const ITEM_ALREADY_EQUIPED: isize = 485;
const ACTION_ALREADY_IN_PROGRESS: isize = 486;
const NO_TASK: isize = 487;
const TASK_NOT_COMPLETED: isize = 488;
const TASK_ALREADY_IN_PROGRESS: isize = 489;
const INVALID_SLOT_STATE: isize = 491;
const INSUFFICIENT_GOLD_ON_CHARACTER: isize = 492;
const NOT_SKILL_LEVEL_REQUIRED: isize = 493;
const CHARACTER_LEVEL_INSUFFICIENT: isize = 496;
const INVENTORY_FULL: isize = 497;
const CHARACTER_NOT_FOUND: isize = 498;
const CHARACTER_ON_COOLDOWN: isize = 499;
const ENTITY_NOT_FOUND_ON_MAP: isize = 598;

#[derive(Debug, Error)]
pub enum FightError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No monster on map")]
    NoMonsterOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for FightError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => {
                    return Self::NoMonsterOnMap;
                }
                _ => {}
            }
        }
        Self::UnhandledError(value)
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
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for GatherError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                NOT_SKILL_LEVEL_REQUIRED => return Self::InsufficientSkillLevel,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoResourceOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum MoveError {
    #[error("MapNotFound")]
    MapNotFound,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for MoveError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            if schema.error.code as isize == ENTITY_NOT_FOUND {
                return Self::MapNotFound;
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum RestError {
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for RestError {
    fn from(value: RequestError) -> Self {
        Self::UnhandledError(value)
    }
}

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
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for UseError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                ITEM_NOT_CONSUMABLE => return Self::ItemNotConsumable,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                CHARACTER_LEVEL_INSUFFICIENT => return Self::InsufficientCharacterLevel,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum CraftError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not craftable")]
    ItemNotCraftable,
    #[error("Insufficient materials")]
    InsufficientMaterials,
    #[error("Insufficient skill level")]
    InsufficientSkillLevel,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Required workshop not on map")]
    NoWorkshopOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for CraftError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotCraftable,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientMaterials,
                NOT_SKILL_LEVEL_REQUIRED => return Self::InsufficientSkillLevel,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoWorkshopOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
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
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for RecycleError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                ITEM_NOT_RECYCLABLE => return Self::ItemNotRecyclable,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                NOT_SKILL_LEVEL_REQUIRED => return Self::InsufficientSkillLevel,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoWorkshopOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum DeleteError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for DeleteError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
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
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for WithdrawError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoBankOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
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
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for DepositError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                BANK_FULL => return Self::InsufficientBankSpace,

                ENTITY_NOT_FOUND_ON_MAP => return Self::NoBankOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum GoldWithdrawError {
    #[error("Insufficient gold in bank")]
    InsufficientGold,
    #[error("No bank on map")]
    NoBankOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for GoldWithdrawError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INSUFFICIENT_GOLD_IN_BANK => return Self::InsufficientGold,

                ENTITY_NOT_FOUND_ON_MAP => return Self::NoBankOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum GoldDepositError {
    #[error("Insufficient gold on character")]
    InsufficientGold,
    #[error("No bank on map")]
    NoBankOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for GoldDepositError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INSUFFICIENT_GOLD_ON_CHARACTER => return Self::InsufficientGold,

                ENTITY_NOT_FOUND_ON_MAP => return Self::NoBankOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum BankExpansionError {
    #[error("Insufficient gold on character")]
    InsufficientGold,
    #[error("No bank on map")]
    NoBankOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for BankExpansionError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INSUFFICIENT_GOLD_ON_CHARACTER => return Self::InsufficientGold,

                ENTITY_NOT_FOUND_ON_MAP => return Self::NoBankOnMap,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum EquipError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Item already equiped")]
    ItemAlreadyEquiped,
    #[error("Quantity greater than slot max quantity")]
    QuantityGreaterThanSlotMaxixum,
    #[error("Slot not empty")]
    SlotNotEmpty,
    #[error("Insufficient character level")]
    InsufficientCharacterLevel,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for EquipError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                ITEM_ALREADY_EQUIPED => return Self::ItemAlreadyEquiped,
                SUPERFLOUS_UTILITY_QUANTITY => return Self::QuantityGreaterThanSlotMaxixum,
                INVALID_SLOT_STATE => return Self::SlotNotEmpty,
                CHARACTER_LEVEL_INSUFFICIENT => return Self::InsufficientCharacterLevel,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum UnequipError {
    #[error("Slot is empty")]
    SlotEmpty,
    #[error("Insufficient quantity")]
    InsufficientQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for UnequipError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                INVALID_SLOT_STATE => return Self::SlotEmpty,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                _ => {}
            }
        }
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
#[repr(isize)]
pub enum TaskAcceptationError {
    #[error("Task already in progress")]
    TaskAlreadyInProgress = TASK_ALREADY_IN_PROGRESS,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for TaskAcceptationError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                TASK_ALREADY_IN_PROGRESS => return Self::TaskAlreadyInProgress,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoTasksMasterOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum TaskTradeError {
    #[error("Item not found")]
    ItemNotFound,
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
    #[error("Wrong tasks master")]
    WrongTasksMaster,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for TaskTradeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                ENTITY_NOT_FOUND => return Self::ItemNotFound,
                WRONG_TASK => return Self::WrongTask,
                TASK_ALREADY_COMPLETED_OR_TOO_MANY_ITEM_TRADED => return Self::SuperfluousQuantity,
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientQuantity,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoTasksMasterOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum TaskCompletionError {
    #[error("No current task")]
    NoCurrentTask,
    #[error("Task not fullfilled")]
    TaskNotFullfilled,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
    #[error("Wrong tasks master")]
    WrongTasksMaster,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for TaskCompletionError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                NO_TASK => return Self::NoCurrentTask,
                TASK_NOT_COMPLETED => return Self::TaskNotFullfilled,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoTasksMasterOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum TaskCancellationError {
    #[error("No current task")]
    NoCurrentTask,
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoin,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
    #[error("Wrong tasks master")]
    WrongTasksMaster,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for TaskCancellationError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientTasksCoin,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoTasksMasterOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeError {
    #[error("Insufficient tasks coin quantity")]
    InsufficientTasksCoinQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No tasks master on map")]
    NoTasksMasterOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for TasksCoinExchangeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientTasksCoinQuantity,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoTasksMasterOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error)]
pub enum GiftExchangeError {
    #[error("Insufficient gift quantity")]
    InsufficientGiftQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No Santa Claus on map")]
    NoSantaClausOnMap,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for GiftExchangeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            match schema.error.code as isize {
                MISSING_OR_INSUFFICIENT_QUANTITY => return Self::InsufficientGiftQuantity,
                INVENTORY_FULL => return Self::InsufficientInventorySpace,
                ENTITY_NOT_FOUND_ON_MAP => return Self::NoSantaClausOnMap,
                _ => {}
            }
        };
        Self::UnhandledError(value)
    }
}
