use openapi::models::SimpleItemSchema;
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum SdkEvent {
    ItemDeposited {
        character: String,
        items: Vec<SimpleItemSchema>,
    },
    ItemWithdrawn {
        character: String,
        items: Vec<SimpleItemSchema>,
    },
    GoldDeposited {
        character: String,
        amount: u32,
    },
    GoldWithdrawn {
        character: String,
        amount: u32,
    },
}

#[derive(Clone, Default)]
pub struct EventBus {
    sender: Option<mpsc::Sender<SdkEvent>>,
}

impl EventBus {
    #[must_use]
    pub fn new() -> (Self, mpsc::Receiver<SdkEvent>) {
        let (tx, rx) = mpsc::channel();
        (Self { sender: Some(tx) }, rx)
    }

    pub fn emit(&self, event: SdkEvent) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(event);
        }
    }
}
