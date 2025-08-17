use artifactsmmo_sdk::char::{
    Skill,
    character::{
        BankExpansionError, CraftError, DeleteError, DepositError, EquipError, FightError,
        GatherError, GoldDepositError, GoldWithdrawError, MoveError, RecycleError, RestError,
        TaskAcceptationError, TaskCancellationError, TaskCompletionError, TaskTradeError,
        TasksCoinExchangeError, UnequipError, WithdrawError,
    },
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KillMonsterCommandError {
    #[error("{0} skill is disabled")]
    SkillDisabled(Skill),
    #[error("No map with monster found")]
    MapNotFound,
    #[error("Unable to check bank for available gear")]
    BankUnavailable,
    #[error("No gear powerfull enough available to kill monster")]
    GearTooWeak { monster_code: String },
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Failed to move to monster")]
    MoveError(#[from] MoveError),
    #[error("Failed to fight")]
    FightError(#[from] FightError),
}

#[derive(Debug, Error)]
pub enum GatherCommandError {
    #[error("{0} skill is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient skill ({0}) level")]
    InsufficientSkillLevel(Skill),
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("No map with resource found")]
    MapNotFound,
    #[error("Failed to move to resource: {0}")]
    MoveError(#[from] MoveError),
    #[error("Failed to request gather: {0}")]
    ClientError(#[from] GatherError),
}

#[derive(Debug, Error)]
pub enum CraftCommandError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not craftable")]
    ItemNotCraftable,
    #[error("Skill ({0}) is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient skill ({0}) level")]
    InsuffisientSkillLevel(Skill, i32),
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Not enough materials available")]
    InsuffisientMaterials,
    #[error("Failed to deposit items: {0}")]
    DepositItemCommandError(#[from] DepositItemCommandError),
    #[error("Failed to withdraw mats: {0}")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to workbench: {0}")]
    MoveError(#[from] MoveError),
    #[error("Failed to request craft: {0}")]
    ClientError(#[from] CraftError),
}

#[derive(Debug, Error)]
pub enum RecycleCommandError {
    #[error("Item not found")]
    ItemNotFound,
    #[error("Item not craftable")]
    ItemNotCraftable,
    #[error("{0} skill is disabled")]
    SkillDisabled(Skill),
    #[error("Insufficient skill level")]
    InsuffisientSkillLevel(Skill, i32),
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
    #[error("Not enough item available")]
    InsufficientQuantity,
    #[error("Failed to withdraw mats")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to workbench")]
    MoveError(#[from] MoveError),
    #[error("Failed to craft item")]
    RecycleError(#[from] RecycleError),
}
#[derive(Debug, Error)]
pub enum DeleteCommandError {
    #[error("Not enough item available")]
    InsufficientQuantity,
    #[error("Failed to withdraw item")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to craft item")]
    DeleteError(#[from] DeleteError),
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
    #[error("Order error")]
    OrderError,
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
    MissingItems { item: String, quantity: i32 },
    #[error("Failed to move to tasks master")]
    MoveError(#[from] MoveError),
    #[error("Failed to exchange task coins")]
    TradeTaskCommandError(#[from] TaskTradeError),
}

#[derive(Debug, Error)]
pub enum TaskAcceptationCommandError {
    #[error("Failed to move to tasks master")]
    MoveError(#[from] MoveError),
    #[error("Failed to exchange task coins")]
    TaskAcceptationError(#[from] TaskAcceptationError),
}

#[derive(Debug, Error)]
pub enum TaskCompletionCommandError {
    #[error("No current task")]
    NoTask,
    #[error("Task no finished")]
    TaskNotFinished,
    #[error("Failed to move to tasks master")]
    MoveError(#[from] MoveError),
    #[error("Failed to complete task")]
    TaskCompletionError(#[from] TaskCompletionError),
}

#[derive(Debug, Error)]
pub enum TasksCoinExchangeCommandError {
    #[error("Not enough coins")]
    NotEnoughCoins,
    #[error("Failed to withdraw coins required")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to exchange task coins")]
    TasksCoinExchangeError(#[from] TasksCoinExchangeError),
    #[error("Order error")]
    OrderError,
}

#[derive(Debug, Error)]
pub enum TaskCancellationCommandError {
    #[error("Not enough coins")]
    NotEnoughCoins,
    #[error("Failed to withdraw coins required")]
    WithdrawItemCommandError(#[from] WithdrawItemCommandError),
    #[error("Failed to move to task master")]
    MoveError(#[from] MoveError),
    #[error("Failed to cancel task")]
    TaskCancellationError(#[from] TaskCancellationError),
}

#[derive(Debug, Error)]
pub enum BankExpansionCommandError {
    #[error("Bank is unavailable")]
    BankUnavailable,
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank")]
    MoveError(#[from] MoveError),
    #[error("Failed to withdraw required gold")]
    GoldWithdrawCommandError(#[from] GoldWithdrawCommandError),
    #[error("Failed to withdraw gold: {0}")]
    BankExpansionError(#[from] BankExpansionError),
}

#[derive(Debug, Error)]
pub enum GoldWithdrawCommandError {
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank")]
    MoveError(#[from] MoveError),
    #[error("Failed to withdraw gold: {0}")]
    GoldWithdrawError(#[from] GoldWithdrawError),
}

#[derive(Debug, Error)]
pub enum GoldDepositCommandError {
    #[error("Insufficient gold")]
    InsufficientGold,
    #[error("Failed to move to bank")]
    MoveError(#[from] MoveError),
    #[error("Failed to deposit gold: {0}")]
    GoldDepositError(#[from] GoldDepositError),
}

#[derive(Debug, Error)]
pub enum MoveCommandError {
    #[error(transparent)]
    MoveError(#[from] MoveError),
}

#[derive(Debug, Error)]
pub enum WithdrawItemCommandError {
    #[error("Missing item quantity")]
    InsufficientQuantity,
    #[error("Failed to move to bank: {0}")]
    MoveError(#[from] MoveError),
    #[error("Failed to request item withdrawal: {0}")]
    ClientError(#[from] WithdrawError),
}

#[derive(Debug, Error)]
pub enum DepositItemCommandError {
    #[error("Missing item quantity")]
    MissingQuantity,
    #[error("Failed to move to bank: {0}")]
    MoveError(#[from] MoveError),
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
    #[error("Failed to unequip equiped item before equiping item")]
    UnequipCommandError(#[from] UnequipCommandError),
    #[error("Failed to equip item")]
    EquipError(#[from] EquipError),
}

#[derive(Debug, Error)]
pub enum UnequipCommandError {
    #[error("Failed to rest")]
    RestError(#[from] RestError),
    #[error("Failed to unequip item")]
    UnequipError(#[from] UnequipError),
    #[error("Insufficient inventory space")]
    InsufficientInventorySpace,
}

#[derive(Debug, Error)]
pub enum OrderProgresssionError {
    #[error("Failed to progress resource order {0}")]
    GatherCommandError(#[from] GatherCommandError),
    #[error("Failed to progress monster order {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("Failed to progress crafting order {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("Failed to progress tasks coin order {0}")]
    TasksCoinExchangeCommandError(#[from] TasksCoinExchangeCommandError),
    #[error("Failed to progress tasks progression order {0}")]
    TaskProgressionError(#[from] TaskProgressionError),
    #[error("No item source found to progress order")]
    NoSourceForItem,
    #[error("No item missin")]
    NoItemMissing,
}

#[derive(Debug, Error)]
pub enum SkillLevelingError {
    #[error("Skill is already at max level")]
    SkillAlreadyMaxed,
    #[error("Failed to kill monster to level combat: {0}")]
    KillMonsterCommandError(#[from] KillMonsterCommandError),
    #[error("Failed to craft to level skill: {0}")]
    CraftCommandError(#[from] CraftCommandError),
    #[error("Failed to gather to level skill: {0}")]
    GatherCommandError(#[from] GatherCommandError),
}

#[derive(Debug, Error)]
pub enum OrderDepositError {
    #[error("No item to deposit in inventory")]
    NoItemToDeposit,
    #[error("Failed to deposit order items")]
    DepositItemCommandError(#[from] DepositItemCommandError),
}
