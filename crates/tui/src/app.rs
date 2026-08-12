use crate::{
    bank::BankWidget,
    chars_info::CharsInfoWidget,
    inventory::InventoryWidget,
    log::{LogBuffer, LogWidget},
    map::{MapState, MapWidget},
    orderboard::OrderboardWidget,
    skills_widget::SkillsWidget,
};
use bot::orderboard::OrderBoard;
use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Layout},
    prelude::{Buffer, Rect},
    widgets::{Block, ListState, StatefulWidget, Tabs, Widget},
};
use sdk::{CharacterClient, Client};
use std::time::Duration;

pub struct App {
    running: bool,
    current_char: Option<CharacterClient>,
    client: Client,
    selected_tab: usize,
    map_state: MapState,
    inventory_state: ListState,
    logs: LogBuffer,
    order_board: OrderBoard,
}

impl App {
    #[must_use]
    pub fn new(client: Client, logs: LogBuffer, order_board: OrderBoard) -> Self {
        let maps = client.maps.clone();
        Self {
            running: false,
            current_char: client.account.characters().first().cloned(),
            client,
            selected_tab: 1,
            map_state: MapState::new(maps),
            inventory_state: ListState::default(),
            logs,
            order_board,
        }
    }

    /// Run the application's main loop.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame.area(), frame.buffer_mut()))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    fn handle_crossterm_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
                _ => {}
            }
        }
        Ok(())
    }

    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c' | 'C')) => self.quit(),
            (_, KeyCode::Char('1')) => self.select_character(0),
            (_, KeyCode::Char('2')) => self.select_character(1),
            (_, KeyCode::Char('3')) => self.select_character(2),
            (_, KeyCode::Char('4')) => self.select_character(3),
            (_, KeyCode::Char('5')) => self.select_character(4),
            (_, KeyCode::Tab) => self.selected_tab = (self.selected_tab + 1) % 4,
            (_, KeyCode::Char('h')) if self.selected_tab == 1 => self.map_state.move_left(),
            (_, KeyCode::Char('k')) if self.selected_tab == 1 => self.map_state.move_up(),
            (_, KeyCode::Char('j')) if self.selected_tab == 1 => self.map_state.move_down(),
            (_, KeyCode::Char('l')) if self.selected_tab == 1 => self.map_state.move_right(),
            _ => {}
        }
    }

    fn select_character(&mut self, id: usize) {
        self.current_char = self.client.account.characters().get(id).cloned();
    }

    const fn quit(&mut self) {
        self.running = false;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::vertical([Constraint::Max(11), Constraint::Min(0)]);
        let [top_area, middle_area] = layout.areas(area);
        let layout = Layout::horizontal([Constraint::Min(100), Constraint::Max(50)]);
        let [char_info_area, skills_area] = layout.areas(top_area);
        let [bank_area, inventory_area] = layout.areas(middle_area);
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]);
        let [tabs_area, content_area] = layout.areas(bank_area);

        CharsInfoWidget::new(
            self.client.account.characters(),
            self.current_char.as_ref().map_or(0, |c| c.id),
        )
        .render(char_info_area, buf);
        SkillsWidget::new(self.current_char.clone()).render(skills_area, buf);
        Tabs::new(["Bank", "Map", "Log", "Orderboard"])
            .block(Block::bordered())
            .select(self.selected_tab)
            .render(tabs_area, buf);
        match self.selected_tab {
            0 => BankWidget::new(self.client.account.bank()).render(content_area, buf),
            1 => MapWidget::new().render(content_area, buf, &mut self.map_state),
            2 => LogWidget::new(self.logs.clone()).render(content_area, buf),
            3 => OrderboardWidget::new(self.order_board.clone()).render(content_area, buf),
            _ => {}
        }
        InventoryWidget::new(self.current_char.clone()).render(
            inventory_area,
            buf,
            &mut self.inventory_state,
        );
    }
}
