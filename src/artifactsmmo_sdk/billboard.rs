use log::info;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Billboard {
    pub queue: RwLock<Vec<Arc<RwLock<Request>>>>,
}

impl Billboard {
    pub fn new() -> Self {
        Billboard {
            queue: RwLock::new(vec![]),
        }
    }

    pub fn request_item(&self, author: &str, item: &str, quantity: i32) {
        let request = Request::new(author, item, quantity);
        if !self.has_similar_request(&request) {
            if let Ok(mut r) = self.queue.write() {
                info!("request added to queue {:?}.", request);
                r.push(Arc::new(RwLock::new(request)))
            }
        }
    }

    pub fn remove_request(&self, request: &Request) {
        if let Ok(mut queue) = self.queue.write() {
            queue.retain(|r| *r.read().unwrap() != *request);
            info!("request removed from queue {:?}", request)
        }
    }

    pub fn has_similar_request(&self, other: &Request) -> bool {
        match self.queue.read() {
            Ok(queue) => {
                return queue
                    .iter()
                    .any(|r| r.read().is_ok_and(|r| r.is_similar(other)));
            }
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Request {
    pub author: String,
    pub item: String,
    pub quantity: i32,
    pub worked: bool,
}

impl Request {
    pub fn new(author: &str, item: &str, quantity: i32) -> Self {
        Request {
            author: author.to_owned(),
            item: item.to_owned(),
            quantity,
            worked: false,
        }
    }

    fn is_similar(&self, other: &Request) -> bool {
        self.author == other.author && self.item == other.item
    }
}
