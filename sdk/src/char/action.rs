use super::request_handler::{RequestError, ResponseSchema};
use crate::gear::Slot;
use artifactsmmo_api_wrapper::ArtifactApi;
use strum_macros::{Display, EnumIs};

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
    //ChristmasExchange,
}

impl Action<'_> {
    pub fn request(
        &self,
        name: &str,
        api: &ArtifactApi,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        match self {
            Action::Move { x, y } => api
                .my_character
                .move_to(name, *x, *y)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Fight => api
                .my_character
                .fight(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Rest => api
                .my_character
                .rest(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::UseItem { item, quantity } => api
                .my_character
                .use_item(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Gather => api
                .my_character
                .gather(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Craft { item, quantity } => api
                .my_character
                .craft(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Recycle { item, quantity } => api
                .my_character
                .recycle(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Delete { item, quantity } => api
                .my_character
                .delete(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Deposit { item, quantity } => api
                .my_character
                .deposit(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Withdraw { item, quantity } => api
                .my_character
                .withdraw(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::DepositGold { quantity } => api
                .my_character
                .deposit_gold(name, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::WithdrawGold { quantity } => api
                .my_character
                .withdraw_gold(name, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::ExpandBank => api
                .my_character
                .expand_bank(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Equip {
                item,
                slot,
                quantity,
            } => api
                .my_character
                .equip(name, item, (*slot).into(), Some(*quantity))
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Unequip { slot, quantity } => {
                api.my_character
                    .unequip(name, (*slot).into(), Some(*quantity))
            }
            .map(|r| r.into())
            .map_err(|e| e.into()),
            Action::AcceptTask => api
                .my_character
                .accept_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::TaskTrade { item, quantity } => api
                .my_character
                .trade_task(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CompleteTask => api
                .my_character
                .complete_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CancelTask => api
                .my_character
                .cancel_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::TaskExchange => api
                .my_character
                .task_exchange(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            //Action::ChristmasExchange => api
            //    .my_character
            //    .christmas_exchange(name)
            //    .map(|r| r.into())
            //    .map_err(|e| e.into()),
        }
    }
}
