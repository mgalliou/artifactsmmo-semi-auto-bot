use crate::{
    character::responses::ResponseSchema, client::character::error::RequestError, gear::Slot,
};
use api::ArtifactApi;
use openapi::models::SimpleItemSchema;
use strum_macros::{Display, EnumIs};

#[derive(Debug, EnumIs, Display)]
pub enum Action<'a> {
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

impl Action<'_> {
    pub fn request(
        &self,
        name: &str,
        api: &ArtifactApi,
    ) -> Result<Box<dyn ResponseSchema>, RequestError> {
        match self {
            Action::Move { x, y } => api
                .my_character
                .r#move(name, *x, *y)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Transition => api
                .my_character
                .transition(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Fight { participants } => api
                .my_character
                .fight(name, *participants)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Rest => api
                .my_character
                .rest(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Gather => api
                .my_character
                .gather(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Craft {
                item_code: item,
                quantity,
            } => api
                .my_character
                .craft(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Recycle {
                item_code: item,
                quantity,
            } => api
                .my_character
                .recycle(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::Delete {
                item_code: item,
                quantity,
            } => api
                .my_character
                .delete(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::DepositItem { items } => api
                .my_character
                .deposit_item(name, items)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::WithdrawItem { items } => api
                .my_character
                .withdraw_item(name, items)
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
                item_code: item,
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
            Action::UseItem {
                item_code: item,
                quantity,
            } => api
                .my_character
                .use_item(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::AcceptTask => api
                .my_character
                .accept_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CancelTask => api
                .my_character
                .cancel_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::TradeTaskItem {
                item_code: item,
                quantity,
            } => api
                .my_character
                .trade_task_item(name, item, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::CompleteTask => api
                .my_character
                .complete_task(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::ExchangeTasksCoins => api
                .my_character
                .exchange_tasks_coins(name)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::NpcBuy {
                item_code: item,
                quantity,
            } => api
                .my_character
                .npc_buy(name, item.to_string(), *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::NpcSell {
                item_code: item,
                quantity,
            } => api
                .my_character
                .npc_sell(name, item.to_string(), *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::GiveItem { items, character } => api
                .my_character
                .give_item(name, items, character)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::GiveGold {
                quantity,
                character,
            } => api
                .my_character
                .give_gold(name, *quantity, character)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::GeBuyOrder { id, quantity } => api
                .my_character
                .ge_buy_order(name, id, *quantity)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::GeCreateOrder {
                item_code: item,
                quantity,
                price,
            } => api
                .my_character
                .ge_create_order(name, item, *quantity, *price)
                .map(|r| r.into())
                .map_err(|e| e.into()),
            Action::GeCancelOrder { id } => api
                .my_character
                .ge_cancel_order(name, id)
                .map(|r| r.into())
                .map_err(|e| e.into()),
        }
    }
}
//
// trait CharacterAction {
//     type Result;
//     type Error;
//
//     fn execute(
//         &self,
//         request_handler: &CharacterRequestHandler,
//     ) -> Result<Self::Result, Self::Error>;
//     fn can_execute(&self, request_handler: &CharacterRequestHandler) -> Result<(), Self::Error>;
// }
//
// struct MoveCharacter {
//     x: i32,
//     y: i32,
//     maps: Arc<MapsClient>,
// }
//
// impl CharacterAction for MoveCharacter {
//     type Result = Map;
//     type Error = MoveError;
//
//     fn execute(&self, handler: &CharacterRequestHandler) -> Result<Self::Result, Self::Error> {
//         self.can_execute(handler)?;
//         Ok(handler.request_move(self.x, self.y)?)
//     }
//
//     fn can_execute(&self, handler: &CharacterRequestHandler) -> Result<(), Self::Error> {
//         if handler.position() == (handler.position().0, self.x, self.y) {
//             return Err(MoveError::AlreadyOnMap);
//         }
//         let Some(map) = self.maps.get(handler.position().0, self.x, self.y) else {
//             return Err(MoveError::MapNotFound);
//         };
//         if map.is_blocked() || !handler.meets_conditions_for(map.access()) {
//             return Err(MoveError::ConditionsNotMet);
//         }
//         Ok(())
//     }
// }
