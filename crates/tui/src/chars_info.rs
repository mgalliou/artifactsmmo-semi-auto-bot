use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Styled},
    text::Line,
    widgets::{Block, Widget},
};
use sdk::{
    CharacterClient, Skill,
    entities::{Character, Map},
};

use crate::{cd_gauge::CdGauge, health_gauge::HealthGauge, xp_gauge::XpGauge};

#[derive(Default)]
pub struct CharsInfoWidget {
    chars: Vec<CharacterClient>,
    selected_id: usize,
}

impl CharsInfoWidget {
    #[must_use]
    pub const fn new(chars: Vec<CharacterClient>, selected_id: usize) -> Self {
        Self { chars, selected_id }
    }
}

impl Widget for CharsInfoWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let layout = Layout::horizontal([Constraint::Min(20); 5]);
        let [area1, area2, area3, area4, area5] = layout.areas(area);

        CharacterInfoWidget::new(self.chars.first().cloned(), self.selected_id).render(area1, buf);
        CharacterInfoWidget::new(self.chars.get(1).cloned(), self.selected_id).render(area2, buf);
        CharacterInfoWidget::new(self.chars.get(2).cloned(), self.selected_id).render(area3, buf);
        CharacterInfoWidget::new(self.chars.get(3).cloned(), self.selected_id).render(area4, buf);
        CharacterInfoWidget::new(self.chars.get(4).cloned(), self.selected_id).render(area5, buf);
    }
}

pub struct CharacterInfoWidget {
    char: Option<CharacterClient>,
    selected: usize,
}

impl CharacterInfoWidget {
    #[must_use]
    pub const fn new(char: Option<CharacterClient>, selected: usize) -> Self {
        Self { char, selected }
    }
}

impl Widget for CharacterInfoWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Some(char) = self.char else {
            Block::bordered().title("No Character").render(area, buf);
            return;
        };
        let snapshot = char.snapshot();
        let level = snapshot.skill_level(Skill::Combat);
        let block = Block::bordered()
            .title_top(Line::from(snapshot.name().to_string()).left_aligned())
            .title_top(Line::from(format!("Level {level}")).right_aligned())
            .set_style(if char.id == self.selected {
                Color::Green
            } else {
                Color::White
            });
        let inner = block.inner(area);
        let layout = Layout::vertical([Constraint::Length(1); 4]);
        let [health_area, xp_area, position_area, cd_area] = layout.areas(inner);
        block.render(area, buf);
        HealthGauge::new(snapshot.clone()).render(health_area, buf);
        XpGauge::new(snapshot).render(xp_area, buf);
        render_position(&char, position_area, buf);
        CdGauge::new(char.clone()).render(cd_area, buf);
    }
}

fn render_position(char: &CharacterClient, position_area: Rect, buf: &mut Buffer) {
    let binding = char.current_map();
    let map_name = binding.name().into_owned();
    let (layer, x, y) = binding.position();
    let map_content_code = char
        .current_map()
        .content()
        .as_ref()
        .map_or_else(|| "None".to_string(), |c| c.code.clone());
    Line::raw(format!(
        "{map_name} ({layer}: {x},{y}) [{map_content_code}]"
    ))
    .centered()
    .render(position_area, buf);
}
