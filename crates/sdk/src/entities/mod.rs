mod account_achievement;
mod active_event;
mod character;
mod event;
mod item;
mod map;
mod monster;
mod npc;
mod npc_item;
mod pending_item;
mod resource;
mod task;
mod task_reward;

pub use account_achievement::*;
pub use active_event::*;
pub(crate) use character::CharacterHandle;
pub use character::{Character, CharacterName, RawCharacter, TaskCode};
pub use event::*;
pub use item::*;
pub(crate) use map::MapHandle;
pub use map::{Map, RawMap};
pub use monster::*;
pub use npc::*;
pub use npc_item::*;
pub use pending_item::*;
pub use resource::*;
pub use task::*;
pub use task_reward::*;

pub trait EventSchemaExt {
    fn content_code(&self) -> Option<&str>;
    fn pretty(&self) -> String;
}
