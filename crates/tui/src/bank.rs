use itertools::Itertools;
use ratatui::{
    prelude::{Buffer, Rect},
    widgets::{Block, List, Widget},
};
use sdk::{BankClient, ItemContainer, bank::Bank};

pub struct BankWidget {
    bank: BankClient,
}

impl BankWidget {
    #[must_use]
    pub const fn new(bank: BankClient) -> Self {
        Self { bank }
    }
}

impl Widget for BankWidget {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        let slot_nbr = self.bank.details().slots;
        let used = self.bank.content().len();

        let block = Block::bordered().title(format!("Bank ({used}/{slot_nbr})"));
        let inner = block.inner(area);
        block.render(area, buf);
        let content = self
            .bank
            .content()
            .iter()
            .map(|i| format!("{} {}", i.code, i.quantity))
            .collect_vec();
        List::new(content).render(inner, buf);
    }
}
