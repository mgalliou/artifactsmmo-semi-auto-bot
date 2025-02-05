use super::{
    base_inventory::BaseInventory,
    request_handler::{CharacterRequestHandler, RequestError},
    CharacterData, HasCharacterData, CHARACTERS_DATA,
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
    CharacterSchema, FightSchema, MapSchema, RecyclingItemsSchema, RewardsSchema, SimpleItemSchema,
    SkillDataSchema, SkillInfoSchema, TaskSchema, TaskTradeSchema,
};
use derive_more::TryFrom;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
};
use thiserror::Error;

pub static BASE_CHARACTERS: LazyLock<HashMap<usize, Arc<BaseCharacter>>> = LazyLock::new(|| {
    CHARACTERS_DATA
        .iter()
        .map(|(id, data)| (*id, Arc::new(BaseCharacter::new(*id, data.clone()))))
        .collect::<_>()
});

pub struct BaseCharacter {
    pub id: usize,
    pub inner: CharacterRequestHandler,
    pub inventory: Arc<BaseInventory>,
}

impl BaseCharacter {
    pub fn new(id: usize, data: CharacterData) -> Self {
        Self {
            id,
            inner: CharacterRequestHandler::new(data.clone()),
            inventory: Arc::new(BaseInventory::new(data.clone())),
        }
    }

    pub fn fight(&self) -> Result<FightSchema, FightError> {
        let Some(monster) = self.map().monster() else {
            return Err(FightError::NoMonsterOnMap);
        };
        if self.inventory.free_space() < monster.max_drop_quantity() {
            return Err(FightError::InsufficientInventorySpace);
        }
        Ok(self.inner.request_fight()?)
    }

    pub fn gather(&self) -> Result<SkillDataSchema, GatherError> {
        let Some(resource) = self.map().resource() else {
            return Err(GatherError::NoResourceOnMap);
        };
        if self.skill_level(resource.skill.into()) < resource.level {
            return Err(GatherError::SkillLevelInsufficient);
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
        if item_code != self.task() {
            return Err(TaskTradeError::WrongTask);
        }
        if self.inventory.total_of(item_code) < quantity {
            return Err(TaskTradeError::InsufficientQuantity);
        }
        if self.task_missing() < quantity {
            return Err(TaskTradeError::SuperfluousQuantity);
        }
        if !self.map().content_type_is(ContentType::TasksMaster)
            || !self.map().content_code_is("items")
        {
            return Err(TaskTradeError::WrongOrNoTasksMasterOnMap);
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
        if !self.map().content_type_is(ContentType::TasksMaster)
            || !self.map().content_code_is(&task_type.to_string())
        {
            return Err(TaskCompletionError::WrongOrNoTasksMasterOnMap);
        }
        Ok(self.inner.request_complete_task()?)
    }

    pub fn cancel_task(&self) -> Result<(), TaskCancellationError> {
        let Some(task_type) = self.task_type() else {
            return Err(TaskCancellationError::NoCurrentTask);
        };
        if self.inventory.total_of("tasks_coin") < 1 {
            return Err(TaskCancellationError::InsufficientTasksCoinQuantity);
        }
        if !self.map().content_type_is(ContentType::TasksMaster)
            || !self.map().content_code_is(&task_type.to_string())
        {
            return Err(TaskCancellationError::WrongOrNoTasksMasterOnMap);
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

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for FightError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for GatherError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum MoveError {
    #[error("MapNotFound")]
    MapNotFound = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for MoveError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum RestError {
    #[error(transparent)]
    UnhandledError(RequestError),
}

impl From<RequestError> for RestError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for UseError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for CraftError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for RecycleError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for DeleteError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for WithdrawError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for DepositError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for GoldWithdrawError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for GoldDepositError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for BankExpansionError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for EquipError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for UnequipError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for TaskAcceptationError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for TaskTradeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for TaskCompletionError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for TaskCancellationError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for TasksCoinExchangeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}

#[derive(Debug, Error, TryFrom)]
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

impl From<RequestError> for GiftExchangeError {
    fn from(value: RequestError) -> Self {
        if let RequestError::ResponseError(ref schema) = value {
            return Self::try_from(schema.error.code as isize)
                .unwrap_or(Self::UnhandledError(value));
        };
        Self::UnhandledError(value)
    }
}
