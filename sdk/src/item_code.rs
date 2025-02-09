use crate::ITEMS;
use nutype::nutype;

fn is_item_code(code: &str) -> bool {
    ITEMS.get(code).is_some()
}

#[nutype(validate(predicate = |s| is_item_code(s)))]
pub struct ItemCode(String);
