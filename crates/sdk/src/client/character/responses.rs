use crate::{ItemList, entities::RawMap};
use downcast_rs::{Downcast, impl_downcast};
use itertools::Itertools;
use openapi::models::{
    ActionType, BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    BankItemTransactionResponseSchema, CharacterFightResponseSchema,
    CharacterMovementResponseSchema, CharacterRestResponseSchema, CharacterSchema,
    CharacterTransitionResponseSchema, ClaimPendingItemResponseSchema, DeleteItemResponseSchema,
    EquipmentResponseSchema, FightResult, GeCreateOrderTransactionResponseSchema,
    GeTransactionResponseSchema, GiveGoldResponseSchema, GiveItemResponseSchema,
    NpcMerchantTransactionResponseSchema, PendingItemSchema, RecyclingResponseSchema,
    RewardDataResponseSchema, SimpleItemSchema, SkillResponseSchema, TaskCancelledResponseSchema,
    TaskResponseSchema, TaskTradeResponseSchema, UseItemResponseSchema,
};
use std::fmt::{self, Display, Formatter};

pub trait ResponseSchema: Downcast {
    fn pretty(&self) -> String;
    fn character(&self) -> &CharacterSchema;

    fn characters(&self) -> Vec<&CharacterSchema> {
        vec![self.character()]
    }

    fn bank_content(&self) -> Option<&Vec<SimpleItemSchema>> {
        None
    }

    fn bank_gold(&self) -> Option<u32> {
        None
    }

    fn extension_price(&self) -> Option<u32> {
        None
    }

    fn claimed_pending_item(&self) -> Option<&PendingItemSchema> {
        None
    }
}

impl_downcast!(ResponseSchema);

impl ResponseSchema for CharacterMovementResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: moved to {}. {}s",
            self.data.character.name,
            RawMap::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterTransitionResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: transitioned to {}. {}s",
            self.data.character.name,
            RawMap::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterFightResponseSchema {
    fn pretty(&self) -> String {
        let chars = &self.data.fight.characters;
        let names = chars.iter().map(|c| c.character_name.clone()).join(",");
        let drops = chars.iter().flat_map(|c| c.drops.clone()).collect_vec();
        let xp = chars.iter().map(|c| c.xp).join("/");
        let gold = chars.iter().map(|c| c.gold).join("/");
        let (verb, loot) = match self.data.fight.result {
            FightResult::Win => ("won", format!(" ([{}], {xp}xp, {gold}g)", ItemList(&drops))),
            FightResult::Loss => ("lost", String::new()),
        };
        format!(
            "{names}: {verb} a {} turns fight against {}{loot}. {}s",
            self.data.fight.turns, self.data.fight.opponent, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        self.data.characters.first().unwrap()
    }

    fn characters(&self) -> Vec<&CharacterSchema> {
        self.data.characters.iter().collect_vec()
    }
}

impl ResponseSchema for CharacterRestResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: rested and restored {}hp. {}s",
            self.data.character.name, self.data.hp_restored, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for UseItemResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: used '{}'. {}s",
            self.data.character.name, self.data.item.code, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for SkillResponseSchema {
    fn pretty(&self) -> String {
        let verb = if self.data.cooldown.reason == ActionType::Crafting {
            "crafted"
        } else {
            "gathered"
        };
        format!(
            "{}: {verb} [{}] ({}xp). {}s",
            self.data.character.name,
            ItemList(&self.data.details.items),
            self.data.details.xp,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for DeleteItemResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: deleted '{}'x{}. {}s",
            self.data.character.name,
            self.data.item.code,
            self.data.item.quantity,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankItemTransactionResponseSchema {
    fn pretty(&self) -> String {
        let (action, prep) = if self.data.cooldown.reason == ActionType::WithdrawItem {
            ("withdrew", "from")
        } else {
            ("deposited", "to")
        };
        format!(
            "{}: {action} [{}] {prep} the bank. {}s",
            self.data.character.name,
            ItemList(&self.data.items),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn bank_content(&self) -> Option<&Vec<SimpleItemSchema>> {
        Some(&self.data.bank)
    }
}

impl ResponseSchema for BankGoldTransactionResponseSchema {
    fn pretty(&self) -> String {
        let (action, prep) = if self.data.cooldown.reason == ActionType::WithdrawGold {
            ("withdrew", "from")
        } else {
            ("deposited", "to")
        };
        format!(
            "{}: {action} {}g {prep} the bank. {}s",
            self.data.character.name, self.data.bank.quantity, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn bank_gold(&self) -> Option<u32> {
        Some(self.data.bank.quantity)
    }
}

impl ResponseSchema for BankExtensionTransactionResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: bought bank expansion for {}g. {}s",
            self.data.character.name,
            self.data.transaction.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn extension_price(&self) -> Option<u32> {
        Some(self.data.transaction.price)
    }
}

impl ResponseSchema for RecyclingResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: recycled and received [{}]. {}s",
            self.data.character.name,
            ItemList(&self.data.details.items),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for EquipmentResponseSchema {
    fn pretty(&self) -> String {
        let action = if self.data.cooldown.reason == ActionType::Equip {
            "equipped"
        } else {
            "unequipped"
        };
        let item_codes = self
            .data
            .items
            .iter()
            .map(|i| format!("'{}'", i.item.code))
            .join(", ");
        format!(
            "{}: {action} [{item_codes}]. {}s",
            self.data.character.name, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: accepted new '{:?}' task: '{}'x{}. {}s",
            self.data.character.name,
            self.data.task.r#type,
            self.data.task.code,
            self.data.task.total,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for RewardDataResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: completed task and was rewarded with [{}] and {}g. {}s",
            self.data.character.name,
            ItemList(&self.data.rewards.items),
            self.data.rewards.gold,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: cancelled current task. {}s",
            self.data.character.name, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskTradeResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: traded '{}'x{} with the taskmaster. {}s",
            self.data.character.name,
            self.data.trade.code,
            self.data.trade.quantity,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for NpcMerchantTransactionResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: traded '{}'x{} for {} {} at {} each. {}s",
            self.data.character.name,
            self.data.transaction.code,
            self.data.transaction.quantity,
            self.data.transaction.total_price,
            self.data.transaction.currency,
            self.data.transaction.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GiveItemResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: gave '{}' to {}. {}s",
            self.data.character.name,
            ItemList(&self.data.items),
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn characters(&self) -> Vec<&CharacterSchema> {
        vec![&self.character(), &self.data.receiver_character]
    }
}

impl ResponseSchema for GiveGoldResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: gave {}g to {}. {}s",
            self.data.character.name,
            self.data.quantity,
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn characters(&self) -> Vec<&CharacterSchema> {
        vec![&self.character(), &self.data.receiver_character]
    }
}

impl ResponseSchema for ClaimPendingItemResponseSchema {
    fn pretty(&self) -> String {
        let what = match (&self.data.item.items, self.data.item.gold) {
            (Some(items), Some(gold)) => format!("[{}] and {gold}g", ItemList(items)),
            (Some(items), None) => format!("[{}]", ItemList(items)),
            (None, Some(gold)) => format!("{gold}g"),
            (None, None) => "pending item".to_string(),
        };
        format!(
            "{}: claimed {what}. {}s",
            self.data.character.name, self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }

    fn characters(&self) -> Vec<&CharacterSchema> {
        vec![&self.character()]
    }

    fn claimed_pending_item(&self) -> Option<&PendingItemSchema> {
        Some(&self.data.item)
    }
}

impl ResponseSchema for GeTransactionResponseSchema {
    fn pretty(&self) -> String {
        let (action, prep) = if self.data.cooldown.reason == ActionType::BuyGe {
            ("bought", "from")
        } else {
            ("canceled", "at")
        };
        format!(
            "{}: {action} an order ['{}'x{} ({}g)] {prep} the grand exchange. {}s",
            self.data.character.name,
            self.data.order.code,
            self.data.order.quantity,
            self.data.order.total_price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GeCreateOrderTransactionResponseSchema {
    fn pretty(&self) -> String {
        format!(
            "{}: created order ['{}'x{} ({}g)] at the grand exchange. {}s",
            self.data.character.name,
            self.data.order.code,
            self.data.order.quantity,
            self.data.order.total_price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl<T: ResponseSchema + 'static> From<T> for Box<dyn ResponseSchema> {
    fn from(value: T) -> Self {
        Box::new(value)
    }
}

impl Display for dyn ResponseSchema {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pretty())
    }
}
