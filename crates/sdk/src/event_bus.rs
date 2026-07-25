use crate::entities::CharacterName;
use bus::{Bus, BusReader};
use log::warn;
use openapi::models::SimpleItemSchema;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum SdkEvent {
    ItemDeposited {
        character: CharacterName,
        items: Vec<SimpleItemSchema>,
    },
    ItemWithdrawn {
        character: CharacterName,
        items: Vec<SimpleItemSchema>,
    },
    GoldDeposited {
        character: CharacterName,
        amount: u32,
    },
    GoldWithdrawn {
        character: CharacterName,
        amount: u32,
    },
}

#[derive(Clone)]
pub struct EventBus {
    bus: Arc<Mutex<Bus<SdkEvent>>>,
}

impl EventBus {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            bus: Arc::new(Mutex::new(Bus::new(capacity))),
        }
    }

    #[must_use]
    pub fn subscribe(&self) -> BusReader<SdkEvent> {
        self.bus.lock().unwrap().add_rx()
    }

    pub fn emit(&self, event: SdkEvent) {
        if self.bus.lock().unwrap().try_broadcast(event).is_err() {
            warn!("event bus: buffer full, event dropped");
        }
    }
}
