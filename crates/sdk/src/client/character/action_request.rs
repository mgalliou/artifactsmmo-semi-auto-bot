use crate::{
    character::responses::ResponseSchema, client::character::error::RequestError, gear::Slot,
};
use api::ArtifactApi;
use openapi::models::SimpleItemSchema;
use strum_macros::{Display, EnumIs};

#[derive(Debug, EnumIs, Display)]
pub enum ActionRequest<'a> {
    Move {
        x: i32,
        y: i32,
    },
    Transition,
    Fight {
        participants: Option<&'a [String; 2]>,
    },
    Rest,
    Gather,
    Craft {
        item_code: &'a str,
        quantity: u32,
    },
    Recycle {
        item_code: &'a str,
        quantity: u32,
    },
    Delete {
        item_code: &'a str,
        quantity: u32,
    },
    DepositItem {
        items: &'a [SimpleItemSchema],
    },
    WithdrawItem {
        items: &'a [SimpleItemSchema],
    },
    DepositGold {
        quantity: u32,
    },
    WithdrawGold {
        quantity: u32,
    },
    ExpandBank,
    Equip {
        item_code: &'a str,
        slot: Slot,
        quantity: u32,
    },
    Unequip {
        slot: Slot,
        quantity: u32,
    },
    UseItem {
        item_code: &'a str,
        quantity: u32,
    },
    AcceptTask,
    CancelTask,
    TradeTaskItem {
        item_code: &'a str,
        quantity: u32,
    },
    CompleteTask,
    ExchangeTasksCoins,
    NpcBuy {
        item_code: &'a str,
        quantity: u32,
    },
    NpcSell {
        item_code: &'a str,
        quantity: u32,
    },
    GiveItem {
        items: &'a [SimpleItemSchema],
        character: &'a str,
    },
    GiveGold {
        quantity: u32,
        character: &'a str,
    },
    GeBuyOrder {
        id: &'a str,
        quantity: u32,
    },
    GeCreateOrder {
        item_code: &'a str,
        quantity: u32,
        price: u32,
    },
    GeCancelOrder {
        id: &'a str,
    },
}

impl ActionRequest<'_> {
    pub fn send(
        &self,
        name: &str,
        api: &ArtifactApi,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        match self {
            ActionRequest::Move { x, y } => api
                .my_character
                .r#move(name, *x, *y)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Transition => api
                .my_character
                .transition(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Fight { participants } => api
                .my_character
                .fight(name, *participants)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Rest => api
                .my_character
                .rest(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Gather => api
                .my_character
                .gather(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Craft {
                item_code: item,
                quantity,
            } => api
                .my_character
                .craft(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Recycle {
                item_code: item,
                quantity,
            } => api
                .my_character
                .recycle(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Delete {
                item_code: item,
                quantity,
            } => api
                .my_character
                .delete(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::DepositItem { items } => api
                .my_character
                .deposit_item(name, items)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::WithdrawItem { items } => api
                .my_character
                .withdraw_item(name, items)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::DepositGold { quantity } => api
                .my_character
                .deposit_gold(name, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::WithdrawGold { quantity } => api
                .my_character
                .withdraw_gold(name, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::ExpandBank => api
                .my_character
                .expand_bank(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Equip {
                item_code: item,
                slot,
                quantity,
            } => api
                .my_character
                .equip(name, item, (*slot).into(), Some(*quantity))
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::Unequip { slot, quantity } => {
                api.my_character
                    .unequip(name, (*slot).into(), Some(*quantity))
            }
            .map(|r| r.into())
            .map_err(|e| e.into()),
            ActionRequest::UseItem {
                item_code: item,
                quantity,
            } => api
                .my_character
                .use_item(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::AcceptTask => api
                .my_character
                .accept_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::CancelTask => api
                .my_character
                .cancel_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::TradeTaskItem {
                item_code: item,
                quantity,
            } => api
                .my_character
                .trade_task_item(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::CompleteTask => api
                .my_character
                .complete_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::ExchangeTasksCoins => api
                .my_character
                .exchange_tasks_coins(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::NpcBuy {
                item_code: item,
                quantity,
            } => api
                .my_character
                .npc_buy(name, item.to_string(), *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::NpcSell {
                item_code: item,
                quantity,
            } => api
                .my_character
                .npc_sell(name, item.to_string(), *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::GiveItem { items, character } => api
                .my_character
                .give_item(name, items, character)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::GiveGold {
                quantity,
                character,
            } => api
                .my_character
                .give_gold(name, *quantity, character)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::GeBuyOrder { id, quantity } => api
                .my_character
                .ge_buy_order(name, id, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::GeCreateOrder {
                item_code: item,
                quantity,
                price,
            } => api
                .my_character
                .ge_create_order(name, item, *quantity, *price)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            ActionRequest::GeCancelOrder { id } => api
                .my_character
                .ge_cancel_order(name, id)
                .map(|r| r.into())
                .map_err(|e| e.into()),
        }
    }
}
