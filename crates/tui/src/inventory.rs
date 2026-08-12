use itertools::Itertools;
use ratatui::{
    prelude::{Buffer, Rect},
    style::{Color, Style},
    widgets::{Block, List, ListState, StatefulWidget, Widget},
};
use sdk::{CharacterClient, ItemContainer, SpaceLimited};

pub struct InventoryWidget {
    char: Option<CharacterClient>,
}

impl InventoryWidget {
    #[must_use]
    pub const fn new(char: Option<CharacterClient>) -> Self {
        Self { char }
    }
}

impl StatefulWidget for InventoryWidget {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let Some(char) = self.char else {
            Block::bordered().title("No Character").render(area, buf);
            return;
        };
        let used = char.inventory().total_items();
        let max = char.inventory().max_items();
        let block = Block::bordered().title(format!("Inventory ({used}/{max})"));
        let inner = block.inner(area);
        block.render(area, buf);
        let inventory = char
            .inventory()
            .content()
            .iter()
            .map(|i| format!("{}: {} ({})", i.slot, i.code, i.quantity))
            .collect_vec();
        StatefulWidget::render(
            List::new(inventory)
                .highlight_style(Style::default().fg(Color::Black).bg(Color::White))
                .style(Style::default().fg(Color::White).bg(Color::Black)),
            inner,
            buf,
            state,
        );
    }
}
