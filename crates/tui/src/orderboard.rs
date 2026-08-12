use bot::orderboard::OrderBoard;
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Block, List, Widget},
};

pub struct OrderboardWidget {
    order_board: OrderBoard,
}

impl OrderboardWidget {
    #[must_use]
    pub const fn new(order_board: OrderBoard) -> Self {
        Self { order_board }
    }
}

impl Widget for OrderboardWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let orders = self.order_board.orders_by_priority();
        let block = Block::bordered().title(format!("Orderboard ({})", orders.len()));
        let inner = block.inner(area);
        block.render(area, buf);
        List::new(orders.iter().map(ToString::to_string)).render(inner, buf);
    }
}
