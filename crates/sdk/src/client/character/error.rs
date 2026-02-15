use derive_more::TryFrom;
use openapi::apis::Error;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use thiserror::Error;

const ENTITY_NOT_FOUND: isize = 404;
const MAXIMUM_ORDERS_CREATED: isize = 433;
const INSUFFICIENT_ORDER_QUANTITY: isize = 434;
const CANNOT_TRADE_WITH_SELF: isize = 435;
// const TRANSACTION_ALREADY_IN_PROGRESS: isize = 436;
const GE_ITEM_NOT_SALABLE: isize = 437;
const ORDER_NOT_OWNED: isize = 438;
const ITEM_NOT_BUYABLE: isize = 441;
const ITEM_NOT_SALABLE: isize = 442;
const BANK_GOLD_INSUFFICIENT: isize = 460;
//const TRANSACTION_ALREADY_IN_PROGRESS: isize = 461;
const BANK_FULL: isize = 462;
const ITEM_NOT_RECYCLABLE: isize = 473;
const WRONG_TASK: isize = 474;
const TASK_ALREADY_COMPLETED_OR_TOO_MANY_ITEM_TRADED: isize = 475;
const ITEM_NOT_CONSUMABLE: isize = 476;
const MISSING_ITEM_OR_INSUFFICIENT_QUANTITY: isize = 478;
const INSUFFICIENT_HEALTH: isize = 483;
const SUPERFLOUS_UTILITY_QUANTITY: isize = 484;
const ITEM_ALREADY_EQUIPED: isize = 485;
const ACTION_ALREADY_IN_PROGRESS: isize = 486;
const NO_TASK: isize = 487;
const TASK_NOT_COMPLETED: isize = 488;
const TASK_ALREADY_IN_PROGRESS: isize = 489;
const ALREADY_ON_MAP: isize = 490;
const INVALID_SLOT_STATE: isize = 491;
const CHARACTER_GOLD_INSUFFICIENT: isize = 492;
const SKILL_LEVEL_INSUFFICIENT: isize = 493;
const CONDITIONS_NOT_MET: isize = 496;
const INVENTORY_FULL: isize = 497;
const CHARACTER_NOT_FOUND: isize = 498;
//const CHARACTER_ON_COOLDOWN: isize = 499;
const NO_PATH_AVAILABLE: isize = 595;
const ENTITY_NOT_FOUND_ON_MAP: isize = 598;

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("reqwest error: {0}")]
    Reqwest(reqwest::Error),
    #[error("serde error: {0}")]
    Serde(serde_json::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
    #[error("response error: {0}")]
    ResponseError(ApiErrorResponseSchema),
    #[error("downcast error")]
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

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorResponseSchema {
    pub error: ApiErrorSchema,
}

impl Display for ApiErrorResponseSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.error.message, self.error.code)
    }
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApiErrorSchema {
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum FightError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Monster is not a boss")]
    MonsterIsNotABoss = ACTION_ALREADY_IN_PROGRESS,
    #[error("No monster on map")]
    NoMonsterOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum MoveError {
    #[error("Map not found")]
    MapNotFound = ENTITY_NOT_FOUND,
    #[error("Already on map")]
    AlreadyOnMap = ALREADY_ON_MAP,
    #[error("Conditions are not met")]
    ConditionsNotMet = CONDITIONS_NOT_MET,
    #[error("No path available")]
    NoPathAvailable = NO_PATH_AVAILABLE,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum TransitionError {
    #[error("Transition not found")]
    TransitionNotFound = ENTITY_NOT_FOUND,
    #[error("Missing required item(s)")]
    MissingItem = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient gold on character")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("Conditions are not met")]
    ConditionsNotMet = CONDITIONS_NOT_MET,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum RestError {
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
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
    InsufficientCharacterLevel = CONDITIONS_NOT_MET,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    #[error("Conditions not met")]
    ConditionsNotMet = CONDITIONS_NOT_MET,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum UnequipError {
    #[error("Insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient health")]
    InsufficientHealth = INSUFFICIENT_HEALTH,
    #[error("Slot is empty")]
    SlotEmpty = INVALID_SLOT_STATE,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
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
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum BuyNpcError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("This item cannot be bought")]
    ItemNotBuyable = ITEM_NOT_BUYABLE,
    #[error("Missing item of insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient gold on character")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Npc not found on map")]
    NpcNotFound = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum SellNpcError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("This item cannot be sold")]
    ItemNotSalable = ITEM_NOT_SALABLE,
    #[error("Missing item or insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Npc not found on map")]
    NpcNotFound = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum GiveItemError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Missing item or insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("Character not found")]
    CharacterNotFound = CHARACTER_NOT_FOUND,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum GiveGoldError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient gold on character")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("Character not found")]
    CharacterNotFound = CHARACTER_NOT_FOUND,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum GeBuyOrderError {
    #[error("Order not found")]
    OrderNotFound = ENTITY_NOT_FOUND,
    #[error("Insufficient order quantity")]
    InsufficientQuantity = INSUFFICIENT_ORDER_QUANTITY,
    #[error("Cannot trade with self")]
    CannotTradeWithSelf = CANNOT_TRADE_WITH_SELF,
    #[error("Insufficient gold")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("Insufficient inventory spcae")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No grand exchange on map")]
    NoGrandExchangeOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum GeCreateOrderError {
    #[error("Item not found")]
    ItemNotFound = ENTITY_NOT_FOUND,
    #[error("Maximum order created")]
    MaximumOrdersCreated = MAXIMUM_ORDERS_CREATED,
    #[error("Item cannot be sold")]
    ItemNotSalable = GE_ITEM_NOT_SALABLE,
    #[error("Missing item or insufficient quantity")]
    InsufficientQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
    #[error("Insufficient gold")]
    InsufficientGold = CHARACTER_GOLD_INSUFFICIENT,
    #[error("No grand exchange on map")]
    NoGrandExchangeOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

#[derive(Debug, Error, TryFrom)]
#[try_from(repr)]
#[repr(isize)]
pub enum GeCancelOrderError {
    #[error("Order not found")]
    OrderNotFound = ENTITY_NOT_FOUND,
    #[error("Order not owned")]
    OrderNotOwned = ORDER_NOT_OWNED,
    #[error("Insufficient inventory spcae")]
    InsufficientInventorySpace = INVENTORY_FULL,
    #[error("No grand exchange on map")]
    NoGrandExchangeOnMap = ENTITY_NOT_FOUND_ON_MAP,
    #[error(transparent)]
    UnhandledError(#[from] RequestError),
}

// #[derive(Debug, Error, TryFrom)]
// #[try_from(repr)]
// #[repr(isize)]
// pub enum GiftExchangeError {
//     #[error("Insufficient gift quantity")]
//     InsufficientGiftQuantity = MISSING_ITEM_OR_INSUFFICIENT_QUANTITY,
//     #[error("Insufficient inventory space")]
//     InsufficientInventorySpace = INVENTORY_FULL,
//     #[error("No Santa Claus on map")]
//     NoSantaClausOnMap = ENTITY_NOT_FOUND_ON_MAP,
//     #[error(transparent)]
//     UnhandledError(#[from] RequestError),
// }
