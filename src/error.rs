use artifactsmmo_sdk::{
    char::{
        Skill,
        error::{
            BankExpansionError, BuyNpcError, CraftError, DeleteError, DepositError, EquipError,
            FightError, GatherError, GoldDepositError, GoldWithdrawError, MoveError, RecycleError,
            RestError, SellNpcError, TaskAcceptationError, TaskCancellationError,
            TaskCompletionError, TaskTradeError, TasksCoinExchangeError, UnequipError, UseError,
            WithdrawError,
        },
    },
    models::SimpleItemSchema,
};
use thiserror::Error;

use crate::orderboard::OrderError;

#[derive(Debug, Error)]
pub enum KillMonsterCommandError {
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("No map with monster found")]
    MapNotFound,
    #[error("Unable to check bank for available gear")]
    BankUnavailable,
    #[error("No gear powerfull enough available to kill '{monster_code}'")]
    GearTooWeak { monster_code: String },
    #[error("Failed to deposit before gathering: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to move: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to rest: {0}")]
    RestError(#[from] RestError),
    #[error("Failed to request fight: {0}")]
    ClientError(#[from] FightError),
}

#[derive(Debug, Error)]
pub enum GatherCommandError {
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient '{0}' level")]
    InsufficientSkillLevel(Skill),
    #[error("Insufficient inventory space")]
    MapNotFound,
    #[error("Failed to deposit before gathering: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to move: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request gather: {0}")]
    ClientError(#[from] GatherError),
}

#[derive(Debug, Error)]
pub enum CraftCommandError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not craftable")]
    ItemNotCraftable,
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient '{0}' level: {1}")]
    InsufficientSkillLevel(Skill, u32),
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Invalid quantity")]
    InvalidQuantity,
    #[error("Insufficient materials quantity available: {0:?}")]
    InsufficientMaterials(Vec<SimpleItemSchema>),
    #[error("Failed to deposit items: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to withdraw mats: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to workbench: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request craft: {0}")]
    ClientError(#[from] CraftError),
}

#[derive(Debug, Error)]
pub enum RecycleCommandError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not recyclable")]
    ItemNotRecyclable,
    #[error("'{0}' skill is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient '{0}' level: {1}")]
    InsufficientSkillLevel(Skill, u32),
    #[error("Invalid quantity")]
    InvalidQuantity,
    #[error("Insufficient item quantity available")]
    InsufficientQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to workbench: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request item recycle: {0}")]
    ClientError(#[from] RecycleError),
}
#[derive(Debug, Error)]
pub enum DeleteCommandError {
    #[error("Invalid quantity")]
    InvalidQuantity,
    #[error("Insufficient item quantity available")]
    InsufficientQuantity,
    #[error("Failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to request item deletion: {0}")]
    ClientError(#[from] DeleteError),
}

#[derive(Debug, Error)]
pub enum TaskTradeCommandError {
    #[error("No current task")]
    NoTask,
    #[error("Invalid task type")]
    InvalidTaskType,
    #[error("Task already completed")]
    TaskAlreadyCompleted,
    #[error("Missing item(s): '{item}'x{quantity}")]
    MissingItems { item: String, quantity: u32 },
    #[error("Failed to withdraw items: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request task trade: {0}")]
    ClientError(#[from] TaskTradeError),
}

#[derive(Debug, Error)]
pub enum TaskAcceptationCommandError {
    #[error("Task already in progress")]
    TaskAlreadyInProgress,
    #[error("Failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request task acceptation: {0}")]
    TaskAcceptationError(#[from] TaskAcceptationError),
}

#[derive(Debug, Error)]
pub enum TaskCompletionCommandError {
    #[error("No current task")]
    NoTask,
    #[error("Task no finished")]
    TaskNotFinished,
    #[error("Failed to deposit items before completing task: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request task completion: {0}")]
    ClientError(#[from] TaskCompletionError),
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeCommandError {
    #[error("Missing coin quantity: {0}")]
    MissingCoins(u32),
    #[error("Failed to withdraw coins required: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to tasks master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request tasks coins exchange: {0}")]
    ClientError(#[from] TasksCoinExchangeError),
}

#[derive(Debug, Error)]
pub enum TaskCancellationCommandError {
    #[error("Missing coin quantity")]
    MissingCoins,
    #[error("Failed to withdraw coins required: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to task master: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request task cancellation: {0}")]
    ClientError(#[from] TaskCancellationError),
}

#[derive(Debug, Error)]
pub enum BankExpansionCommandError {
    #[error("Bank is unavailable")]
    BankUnavailable,
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to withdraw required gold: {0}")]
    GoldWithdrawCommandError(#[from] GoldWithdrawCommandError),
    #[error("Failed to request bank expansion: {0}")]
    ClientError(#[from] BankExpansionError),
}

#[derive(Debug, Error)]
pub enum GoldWithdrawCommandError {
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request gold withdrawal: {0}")]
    ClientError(#[from] GoldWithdrawError),
}

#[derive(Debug, Error)]
pub enum GoldDepositCommandError {
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request gold deposit: {0}")]
    ClientError(#[from] GoldDepositError),
}

#[derive(Debug, Error)]
pub enum MoveCommandError {
    #[error("Failed to find target map")]
    MapNotFound,
    #[error("Failed to request movement: {0}")]
    MoveError(#[from] MoveError),
}

#[derive(Debug, Error)]
pub enum WithdrawItemCommandError {
    #[error("Insufficient item quantity available")]
    InsufficientQuantity,
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Failed to deposit item before withdrawing: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request item withdrawal: {0}")]
    ClientError(#[from] WithdrawError),
}

#[derive(Debug, Error)]
pub enum DepositItemCommandError {
    #[error("Insufficient item quantity available")]
    InsufficientQuantity,
    #[error("Failed to move to bank: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Insufficient bank space")]
    InsufficientBankSpace,
    #[error("Failed to request item deposit: {0}")]
    ClientError(#[from] DepositError),
}

#[derive(Debug, Error)]
pub enum EquipCommandError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Conditions not met")]
    ConditionsNotMet,
    #[error("Failed to deposit all before equiping item: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to unequip equiped item before equiping item: {0}")]
    UnequipCommandError(#[from] UnequipCommandError),
    #[error("Failed to request equip item: {0}")]
    ClientError(#[from] EquipError),
}

#[derive(Debug, Error)]
pub enum UnequipCommandError {
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Failed to rest: {0}")]
    RestError(#[from] RestError),
    #[error("Failed to unequip item: {0}")]
    ClientError(#[from] UnequipError),
}

#[derive(Debug, Error)]
pub enum UseItemCommandError {
    #[error("Failed to request item use: {0}")]
    ClientError(#[from] UseError),
}

#[derive(Debug, Error)]
pub enum BuyNpcCommandError {
    #[error("Item not found: {0}")]
    ItemNotFound(String),
    #[error("Item not purchasable")]
    ItemNotPurchasable,
    #[error("No NPC found on map to purchase item")]
    NpcNotFound,
    #[error("Insufficient currency: '{currency}'x{quantity}")]
    InsufficientCurrency { currency: String, quantity: u32 },
    #[error("Failed to deposit all before withdrawing currency: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to withdraw gold from bank: {0}")]
    GoldWithdrawCommandError(#[from] GoldWithdrawCommandError),
    #[error("Failed to withdraw currency from bank: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to NPC: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request npc purchase : {0}")]
    ClientError(#[from] BuyNpcError),
}

#[derive(Debug, Error)]
pub enum SellNpcCommandError {
    #[error("Character is not allowed")]
    NotAllowed,
    #[error("Item not found: {0}")]
    ItemNotFound(String),
    #[error("Item not sellable")]
    ItemNotSellable,
    #[error("No NPC found on map to sell item")]
    NpcNotFound,
    #[error("Insufficient item quantity available")]
    InsufficientQuantity { quantity: u32 },
    #[error("Failed to deposit all before selling item: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to withdraw item to sell: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to NPC: {0}")]
    MoveCommandError(#[from] MoveCommandError),
    #[error("Failed to request npc purchase : {0}")]
    ClientError(#[from] SellNpcError),
}

#[derive(Debug, Error)]
pub enum OrderProgressionError {
    #[error("No item missing")]
    NoItemMissing,
    #[error("No item source found to progress order")]
    NoSourceForItem,
    #[error("Failed to progress resource order: {0}")]
    GatherCommandError(#[from] GatherCommandError),
    #[error("Failed to progress monster order: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("Failed to progress crafting order: {0}")]
    CraftOrderProgressionError(#[from] CraftOrderProgressionError),
    #[error("Failed to progress tasks coin exchange order: {0}")]
    TasksCoinExchangeOrderProgressionError(#[from] TasksCoinExchangeOrderProgressionError),
    #[error("Failed to progress tasks progression order: {0}")]
    TaskProgressionError(#[from] TaskProgressionError),
    #[error("Failed to progress npc purchase order: {0}")]
    BuyNpcOrderProgressionError(#[from] BuyNpcOrderProgressionError),
}

#[derive(Debug, Error)]
pub enum CraftOrderProgressionError {
    #[error("Failed to craft items: {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("Failed to order missing mats: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum TaskProgressionError {
    #[error("Failed to accept task: {0}")]
    TaskAcceptationCommandError(#[from] TaskAcceptationCommandError),
    #[error("Failed to complete task: {0}")]
    TaskCompletionCommandError(#[from] TaskCompletionCommandError),
    #[error("Failed to trade task: {0}")]
    TaskTradeCommandError(#[from] TaskTradeCommandError),
    #[error("Failed to cancel task: {0}")]
    TaskCancellationCommandError(#[from] TaskCancellationCommandError),
    #[error("Failed to fight: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("Order error missing items: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum BuyNpcOrderProgressionError {
    #[error("Failed to command npc buy: {0}")]
    BuyNpcCommandError(#[from] BuyNpcCommandError),
    #[error("Failed to order missing currency: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeOrderProgressionError {
    #[error("Failed to command tasks coin exchange: {0}")]
    TasksCoinExchangeCommandError(#[from] TasksCoinExchangeCommandError),
    #[error("Failed to order missing items: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum SkillLevelingError {
    #[error("Skill is already at max level")]
    SkillAlreadyMaxed,
    #[error("Failed to kill monster to level combat: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("Failed to level combat: {0}")]
    CombatLevelingError(#[from] CombatLevelingError),
    #[error("Failed to level skill by crafting: {0}")]
    CraftSkillLevelingError(#[from] CraftSkillLevelingError),
    #[error("Failed to gather for leveling skill: {0}")]
    GatherCommandError(#[from] GatherCommandError),
}

#[derive(Debug, Error)]
pub enum CombatLevelingError {
    #[error("Failed to progress task : {0}")]
    TaskProgressionError(#[from] TaskProgressionError),
    #[error("No monster killable providing xp found")]
    NoMonsterFound,
    #[error("Failed to kill monster to level combat: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
}

#[derive(Debug, Error)]
pub enum CraftSkillLevelingError {
    #[error("No craftable item found to level skill:")]
    ItemNotFound,
    #[error("Failed to craft to level skill: {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("Failed to order missing mats: {0}")]
    OrderError(#[from] OrderError),
}

#[derive(Debug, Error)]
pub enum OrderDepositError {
    #[error("No item to deposit in inventory")]
    NoItemToDeposit,
    #[error("Failed to deposit order items: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
}

#[derive(Debug, Error)]
pub enum BankCleanupError {
    #[error("Failed to sell item from bank: {0}")]
    SellNpcCommandError(#[from] SellNpcCommandError),
    #[error("No item to handle")]
    NoItemToHandle,
}
