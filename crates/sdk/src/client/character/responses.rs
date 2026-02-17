use downcast_rs::{Downcast, impl_downcast};
use itertools::Itertools;
use openapi::models::{
    ActionType, BankExtensionTransactionResponseSchema, BankGoldTransactionResponseSchema,
    BankItemTransactionResponseSchema, CharacterFightResponseSchema,
    CharacterMovementResponseSchema, CharacterRestResponseSchema, CharacterSchema,
    CharacterTransitionResponseSchema, DeleteItemResponseSchema, EquipmentResponseSchema,
    FightResult, GeCreateOrderTransactionResponseSchema, GeTransactionResponseSchema,
    GiveGoldResponseSchema, GiveItemResponseSchema, NpcMerchantTransactionResponseSchema,
    RecyclingResponseSchema, RewardDataResponseSchema, SkillResponseSchema,
    TaskCancelledResponseSchema, TaskResponseSchema, TaskTradeResponseSchema,
    UseItemResponseSchema,
};

use crate::{DropSchemas, SimpleItemSchemas, entities::Map};

pub trait ResponseSchema: Downcast {
    fn character(&self) -> &CharacterSchema;
    fn to_string(&self) -> String;
}
impl_downcast!(ResponseSchema);

impl ResponseSchema for CharacterMovementResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: moved to {}. {}s",
            self.data.character.name,
            Map::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterTransitionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: transitioned to {}. {}s",
            self.data.character.name,
            Map::new(*self.data.destination.clone()),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for CharacterFightResponseSchema {
    fn to_string(&self) -> String {
        let chars = &self.data.fight.characters;
        let names = chars.iter().map(|c| c.character_name.to_string()).join(",");
        let drops = chars.iter().flat_map(|c| c.drops.clone()).collect_vec();
        let xp = chars.iter().map(|c| c.xp).join("/");
        let gold = chars.iter().map(|c| c.gold).join("/");
        match self.data.fight.result {
            FightResult::Win => format!(
                "{} won a fight after {} turns ([{}], {}xp, {}g). {}s",
                names,
                self.data.fight.turns,
                DropSchemas(&drops),
                xp,
                gold,
                self.data.cooldown.remaining_seconds
            ),
            FightResult::Loss => format!(
                "{} lost a fight after {} turns. {}s",
                self.data.characters.first().unwrap().name,
                self.data.fight.turns,
                self.data.cooldown.remaining_seconds
            ),
        }
    }

    fn character(&self) -> &CharacterSchema {
        self.data.characters.first().unwrap()
    }
}

impl ResponseSchema for CharacterRestResponseSchema {
    fn to_string(&self) -> String {
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
    fn to_string(&self) -> String {
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
    fn to_string(&self) -> String {
        let reason = if self.data.cooldown.reason == ActionType::Crafting {
            "crafted"
        } else {
            "gathered"
        };
        format!(
            "{}: {reason} [{}] ({}xp). {}s",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.details.xp,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for DeleteItemResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: deleted '{}'x{}",
            self.data.character.name, self.data.item.code, self.data.item.quantity
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankItemTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::WithdrawItem {
            format!(
                "{}: withdrawed [{}] from the bank. {}s",
                self.data.character.name,
                SimpleItemSchemas(&self.data.items),
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: deposited [{}] to the bank. {}s",
                self.data.character.name,
                SimpleItemSchemas(&self.data.items),
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankGoldTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::WithdrawGold {
            format!(
                "{}: withdrawed gold from the bank. {}s",
                self.data.character.name, self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: deposited gold to the bank. {}s",
                self.data.character.name, self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for BankExtensionTransactionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: bought bank expansion for {} golds. {}s",
            self.data.character.name,
            self.data.transaction.price,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for RecyclingResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: recycled and received {}. {}s",
            self.data.character.name,
            DropSchemas(&self.data.details.items),
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for EquipmentResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::Equip {
            format!(
                "{}: equiped '{}' in the '{}' slot. {}s",
                &self.data.character.name,
                &self.data.item.code,
                &self.data.slot,
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: unequiped '{}' from the '{}' slot. {}s",
                &self.data.character.name,
                &self.data.item.code,
                &self.data.slot,
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: accepted new [{:?}] task: '{}'x{}. {}s",
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
    fn to_string(&self) -> String {
        format!(
            "{}: completed task and was rewarded with [{}] and {}g. {}s",
            self.data.character.name,
            SimpleItemSchemas(&self.data.rewards.items),
            self.data.rewards.gold,
            self.data.cooldown.remaining_seconds
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for TaskCancelledResponseSchema {
    fn to_string(&self) -> String {
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
    fn to_string(&self) -> String {
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
    fn to_string(&self) -> String {
        format!(
            "{}: traded {} {} for {} {}(s) at {} each. {}s",
            self.data.character.name,
            self.data.transaction.quantity,
            self.data.transaction.code,
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
    fn to_string(&self) -> String {
        format!(
            "{}: gave '{}' to {}. {}s",
            self.data.character.name,
            SimpleItemSchemas(&self.data.items),
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GiveGoldResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: gave {} gold to {}. {}s",
            self.data.character.name,
            self.data.quantity,
            self.data.receiver_character.name,
            self.data.cooldown.remaining_seconds,
        )
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GeTransactionResponseSchema {
    fn to_string(&self) -> String {
        if self.data.cooldown.reason == ActionType::BuyGe {
            format!(
                "{}: bought '{}'x{} for {}g from the grand exchange. {}",
                self.data.character.name,
                self.data.order.code,
                self.data.order.quantity,
                self.data.order.total_price,
                self.data.cooldown.remaining_seconds
            )
        } else {
            format!(
                "{}: canceled order '{}'x{} for {}g at the grand exchange. {}",
                self.data.character.name,
                self.data.order.code,
                self.data.order.quantity,
                self.data.order.total_price,
                self.data.cooldown.remaining_seconds
            )
        }
    }

    fn character(&self) -> &CharacterSchema {
        &self.data.character
    }
}

impl ResponseSchema for GeCreateOrderTransactionResponseSchema {
    fn to_string(&self) -> String {
        format!(
            "{}: created order '{}'x{} for {}g at the grand exchange. {}s",
            self.data.character.name,
            self.data.order.code,
            self.data.order.quantity,
            self.data.order.price,
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
