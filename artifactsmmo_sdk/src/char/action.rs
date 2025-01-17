use strum_macros::{Display, EnumIs};
use crate::gear::Slot;

#[derive(Debug, EnumIs, Display)]
pub enum Action<'a> {
    Move {
        x: i32,
        y: i32,
    },
    Fight,
    Rest,
    UseItem {
        item: &'a str,
        quantity: i32,
    },
    Gather,
    Craft {
        item: &'a str,
        quantity: i32,
    },
    Recycle {
        item: &'a str,
        quantity: i32,
    },
    Delete {
        item: &'a str,
        quantity: i32,
    },
    Deposit {
        item: &'a str,
        quantity: i32,
    },
    Withdraw {
        item: &'a str,
        quantity: i32,
    },
    DepositGold {
        quantity: i32,
    },
    WithdrawGold {
        quantity: i32,
    },
    ExpandBank,
    Equip {
        item: &'a str,
        slot: Slot,
        quantity: i32,
    },
    Unequip {
        slot: Slot,
        quantity: i32,
    },
    AcceptTask,
    TaskTrade {
        item: &'a str,
        quantity: i32,
    },
    CompleteTask,
    CancelTask,
    TaskExchange,
    ChristmasExchange,
}
