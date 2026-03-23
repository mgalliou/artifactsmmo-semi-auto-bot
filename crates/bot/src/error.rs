use sdk::{
    client::character::error::{
        BankExpansionError, BuyNpcError, CraftError, DeleteError, DepositError, EquipError,
        FightError, GatherError, GoldDepositError, GoldWithdrawError, MoveError, RecycleError,
        RestError, SellNpcError, TaskAcceptationError, TaskCancellationError, TaskCompletionError,
        TaskTradeError, TasksCoinExchangeError, UnequipError, UseError, WithdrawError,
    },
    models::SimpleItemSchema,
    skill::Skill,
};
use thiserror::Error;

use crate::{bank::BankReservationError, orderboard::OrderError};

#[derive(Debug, Error)]
pub enum KillMonsterCommandError {
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("no map with monster found")]
    MapNotFound,
    #[error("failed to accept task: {0}")]
    TaskAcceptationCommandError(#[from] TaskAcceptationCommandError),
    #[error("no gear powerful enough available to kill '{monster_code}'")]
    GearTooWeak { monster_code: String },
    #[error("failed to equip gear: {0}")]
    EquipGearCommandError(#[from] EquipGearCommandError),
    #[error("failed to deposit before gathering: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to move: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to rest: {0}")]
    RestError(#[from] RestError),
    #[error("failed to request fight: {0}")]
    ClientError(#[from] FightError),
}

#[derive(Debug, Error)]
pub enum GatherCommandError {
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("insufficient '{0}' level")]
    InsufficientSkillLevel(Skill),
    #[error("insufficient inventory space")]
    MapNotFound,
    #[error("failed to equip gear: {0}")]
    EquipGearCommandError(#[from] EquipGearCommandError),
    #[error("failed to deposit before gathering: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to move: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request gather: {0}")]
    ClientError(#[from] GatherError),
}

#[derive(Debug, Error)]
pub enum CraftCommandError {
    #[error("item not found")]
    ItemNotFound,
    #[error("item not craftable")]
    ItemNotCraftable,
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("insufficient '{0}' level: {1}")]
    InsufficientSkillLevel(Skill, u32),
    #[error("insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("invalid quantity")]
    InvalidQuantity,
    #[error("insufficient materials quantity available: {0:?}")]
    InsufficientMaterials(Vec<SimpleItemSchema>),
    #[error("failed to reserve mats before crafting: {0}")]
    ReservationError(#[from] BankReservationError),
    #[error("failed to equip gear: {0}")]
    EquipGearCommandError(#[from] EquipGearCommandError),
    #[error("failed to deposit items: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to withdraw mats: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to workbench: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request craft: {0}")]
    ClientError(#[from] CraftError),
}

#[derive(Debug, Error)]
pub enum RecycleCommandError {
    #[error("item not found")]
    ItemNotFound,
    #[error("item not recyclable")]
    ItemNotRecyclable,
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("insufficient '{0}' level: {1}")]
    InsufficientSkillLevel(Skill, u32),
    #[error("invalid quantity")]
    InvalidQuantity,
    #[error("insufficient item quantity available")]
    InsufficientQuantity,
    #[error("insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to workbench: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request item recycle: {0}")]
    ClientError(#[from] RecycleError),
}
#[derive(Debug, Error)]
pub enum DeleteCommandError {
    #[error("invalid quantity")]
    InvalidQuantity,
    #[error("insufficient item quantity available")]
    InsufficientQuantity,
    #[error("failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to request item deletion: {0}")]
    ClientError(#[from] DeleteError),
}

#[derive(Debug, Error)]
pub enum TaskTradeCommandError {
    #[error("no current task")]
    NoTask,
    #[error("invalid task type")]
    InvalidTaskType,
    #[error("task already completed")]
    TaskAlreadyCompleted,
    #[error("missing item(s): '{item}'x{quantity}")]
    MissingItems { item: String, quantity: u32 },
    #[error("failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request task trade: {0}")]
    ClientError(#[from] TaskTradeError),
}

#[derive(Debug, Error)]
pub enum TaskAcceptationCommandError {
    #[error("task already in progress")]
    TaskAlreadyInProgress,
    #[error("failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request task acceptation: {0}")]
    TaskAcceptationError(#[from] TaskAcceptationError),
}

#[derive(Debug, Error)]
pub enum TaskCompletionCommandError {
    #[error("no current task")]
    NoTask,
    #[error("task not finished")]
    TaskNotFinished,
    #[error("failed to deposit items before completing task: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request task completion: {0}")]
    ClientError(#[from] TaskCompletionError),
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeCommandError {
    #[error("missing coin quantity: {0}")]
    MissingCoins(u32),
    #[error("failed to withdraw coins required: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request tasks coins exchange: {0}")]
    ClientError(#[from] TasksCoinExchangeError),
}

#[derive(Debug, Error)]
pub enum TaskCancellationCommandError {
    #[error("missing coin quantity")]
    MissingCoins,
    #[error("failed to withdraw coins required: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to task master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request task cancellation: {0}")]
    ClientError(#[from] TaskCancellationError),
}

#[derive(Debug, Error)]
pub enum BankExpansionCommandError {
    #[error("bank is unavailable")]
    BankUnavailable,
    #[error("insufficient gold")]
    InsufficientGold,
    #[error("failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to withdraw required gold: {0}")]
    GoldWithdrawCommandError(#[from] GoldWithdrawCommandError),
    #[error("failed to request bank expansion: {0}")]
    ClientError(#[from] BankExpansionError),
}

#[derive(Debug, Error)]
pub enum GoldWithdrawCommandError {
    #[error("insufficient gold")]
    InsufficientGold,
    #[error("failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request gold withdrawal: {0}")]
    ClientError(#[from] GoldWithdrawError),
}

#[derive(Debug, Error)]
pub enum GoldDepositCommandError {
    #[error("insufficient gold")]
    InsufficientGold,
    #[error("failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request gold deposit: {0}")]
    ClientError(#[from] GoldDepositError),
}

#[derive(Debug, Error)]
pub enum MoveCommandError {
    #[error("failed to find target map")]
    MapNotFound,
    #[error("failed to request movement: {0}")]
    MoveError(#[from] MoveError),
}

#[derive(Debug, Error)]
pub enum WithdrawItemCommandError {
    #[error("insufficient item quantity available")]
    InsufficientQuantity,
    #[error("insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("failed to reserve item before withdrawing: {0}")]
    ReservationError(#[from] BankReservationError),
    #[error("failed to deposit item before withdrawing: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request item withdrawal: {0}")]
    ClientError(#[from] WithdrawError),
}

#[derive(Debug, Error)]
pub enum DepositItemCommandError {
    #[error("insufficient item quantity available")]
    InsufficientQuantity,
    #[error("failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("insufficient bank space")]
    InsufficientBankSpace,
    #[error("failed to request item deposit: {0}")]
    ClientError(#[from] DepositError),
}

#[derive(Debug, Error)]
pub enum EquipGearCommandError {
    #[error("failed to reserve item from bank: {0}")]
    BankReservationError(#[from] BankReservationError),
    #[error("failed to equip an item: {0}")]
    EquipCommandError(#[from] EquipCommandError),
    #[error("failed to unequip an item: {0}")]
    UnequipCommandError(#[from] UnequipCommandError),
}

#[derive(Debug, Error)]
pub enum EquipCommandError {
    #[error("item not found")]
    ItemNotFound,
    #[error("conditions not met")]
    ConditionsNotMet,
    #[error("failed to withdraw item before equipping: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to deposit all before equipping item: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to unequip equipped item before equipping item: {0}")]
    UnequipCommandError(#[from] UnequipCommandError),
    #[error("failed to request equip item: {0}")]
    ClientError(#[from] EquipError),
}

#[derive(Debug, Error)]
pub enum UnequipCommandError {
    #[error("invalid quantity: {0}")]
    InvalidQuantity(u32),
    #[error("failed to deposit all before unequipping item: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to rest: {0}")]
    RestError(#[from] RestError),
    #[error("failed to unequip item: {0}")]
    ClientError(#[from] UnequipError),
}

#[derive(Debug, Error)]
pub enum UseItemCommandError {
    #[error("insufficient item quantity in inventory")]
    InsufficientQuantity,
    #[error("failed to request item use: {0}")]
    ClientError(#[from] UseError),
}

#[derive(Debug, Error)]
pub enum BuyNpcCommandError {
    #[error("item not found: {0}")]
    ItemNotFound(String),
    #[error("item not purchasable")]
    ItemNotPurchasable,
    #[error("no NPC found on map to purchase item")]
    NpcNotFound,
    #[error("insufficient currency: '{currency}'x{quantity}")]
    InsufficientCurrency { currency: String, quantity: u32 },
    #[error("insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("failed to deposit all before withdrawing currency: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to withdraw gold from bank: {0}")]
    GoldWithdrawCommandError(#[from] GoldWithdrawCommandError),
    #[error("failed to withdraw currency from bank: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to NPC: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request NPC purchase: {0}")]
    ClientError(#[from] BuyNpcError),
}

#[derive(Debug, Error)]
pub enum SellNpcCommandError {
    #[error("character is not allowed")]
    NotAllowed,
    #[error("item not found: {0}")]
    ItemNotFound(String),
    #[error("item not sellable")]
    ItemNotSellable,
    #[error("no NPC found on map to sell item")]
    NpcNotFound,
    #[error("insufficient item quantity available")]
    InsufficientQuantity { quantity: u32 },
    #[error("insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("failed to deposit all before selling item: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("failed to withdraw item to sell: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("failed to move to NPC: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("failed to request NPC sell: {0}")]
    ClientError(#[from] SellNpcError),
}

#[derive(Debug, Error)]
pub enum OrderProgressionError {
    #[error("no item missing")]
    NoItemMissing,
    #[error("no item source found to progress order")]
    NoSourceForItem,
    #[error("failed to progress resource order: {0}")]
    GatherCommandError(#[from] GatherCommandError),
    #[error("failed to progress monster order: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("failed to progress crafting order: {0}")]
    CraftOrderProgressionError(#[from] CraftOrderProgressionError),
    #[error("failed to progress tasks coin exchange order: {0}")]
    TasksCoinExchangeOrderProgressionError(#[from] TasksCoinExchangeOrderProgressionError),
    #[error("failed to progress tasks progression order: {0}")]
    TaskProgressionError(#[from] TaskProgressionError),
    #[error("failed to progress NPC purchase order: {0}")]
    BuyNpcOrderProgressionError(#[from] BuyNpcOrderProgressionError),
}

#[derive(Debug, Error)]
pub enum FoodOrderingError {
    #[error("combat skill is disabled")]
    CombatSkillDisabled,
    #[error("no food can be farmed")]
    NoFoodFarmable,
    #[error("failed to order food: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum CraftOrderProgressionError {
    #[error("failed to craft items: {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("failed to order missing mats: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum TaskProgressionError {
    #[error("failed to accept task: {0}")]
    TaskAcceptationCommandError(#[from] TaskAcceptationCommandError),
    #[error("failed to complete task: {0}")]
    TaskCompletionCommandError(#[from] TaskCompletionCommandError),
    #[error("failed to trade task: {0}")]
    TaskTradeCommandError(#[from] TaskTradeCommandError),
    #[error("failed to cancel task: {0}")]
    TaskCancellationCommandError(#[from] TaskCancellationCommandError),
    #[error("failed to fight: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("order error missing items: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum BuyNpcOrderProgressionError {
    #[error("failed to command NPC buy: {0}")]
    BuyNpcCommandError(#[from] BuyNpcCommandError),
    #[error("failed to order missing currency: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeOrderProgressionError {
    #[error("failed to command tasks coin exchange: {0}")]
    TasksCoinExchangeCommandError(#[from] TasksCoinExchangeCommandError),
    #[error("failed to order missing items: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum SkillLevelingError {
    #[error("skill is already at max level")]
    SkillAlreadyMaxed,
    #[error("failed to kill monster to level combat: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("failed to level combat: {0}")]
    CombatLevelingError(#[from] CombatLevelingError),
    #[error("failed to level skill by crafting: {0}")]
    CraftSkillLevelingError(#[from] CraftSkillLevelingError),
    #[error("failed to gather for leveling skill: {0}")]
    GatherCommandError(#[from] GatherCommandError),
}

#[derive(Debug, Error)]
pub enum CombatLevelingError {
    #[error("failed to progress task: {0}")]
    TaskProgressionError(#[from] TaskProgressionError),
    #[error("no monster killable providing xp found")]
    NoMonsterFound,
    #[error("failed to kill monster to level combat: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
}

#[derive(Debug, Error)]
pub enum CraftSkillLevelingError {
    #[error("no craftable item found to level skill")]
    ItemNotFound,
    #[error("failed to craft to level skill: {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("failed to order missing mats: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum OrderDepositError {
    #[error("no item to deposit in inventory")]
    NoItemToDeposit,
    #[error("failed to deposit order items: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
}

#[derive(Debug, Error)]
pub enum BankCleanupError {
    #[error("failed to sell item from bank: {0}")]
    SellNpcCommandError(#[from] SellNpcCommandError),
    #[error("no item to handle")]
    NoItemToHandle,
}
