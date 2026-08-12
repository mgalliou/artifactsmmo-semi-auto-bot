use ratatui::{
    layout::{Constraint, Size},
    prelude::{Buffer, Position, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Cell, Row, StatefulWidget, Table, TableState},
};
use sdk::{MapsClient, entities::Map, models::MapLayer::Overworld};
use std::ops::Range;
use tui_scrollview::{ScrollView, ScrollViewState};

pub const CELL_HEIGHT: u16 = 5;
pub const CELL_WIDTH: u16 = 20;

#[derive(Default)]
pub struct MapWidget;

impl MapWidget {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Builds a table containing only the map cells inside the current viewport.
    /// Table indices are zero-based; map coordinates are offset from the map's minimum bounds.
    fn construct_table(maps: &MapsClient, visible: &VisibleSpans) -> Table<'static> {
        let rows = visible.rows.clone().map(|row| {
            Self::construct_row(maps, visible.columns.clone(), maps.min_y() + row as i32)
        });
        Table::new(
            rows,
            vec![Constraint::Length(CELL_WIDTH); visible.columns.len()],
        )
        .column_spacing(0)
        .cell_highlight_style(Style::new().fg(Color::Black).bg(Color::White))
    }

    fn construct_row(maps: &MapsClient, columns: Range<usize>, y: i32) -> Row<'static> {
        Row::new(columns.map(|column| Self::construct_cell(maps, maps.min_x() + column as i32, y)))
            .height(CELL_HEIGHT)
    }

    fn construct_cell(maps: &MapsClient, x: i32, y: i32) -> Cell<'static> {
        maps.get_raw(&(Overworld, x, y))
            .map_or_else(Cell::default, |map| {
                Cell::from(Text::from(vec![
                    Line::from(""),
                    Line::from(format!("{},{}", map.x(), map.y())).right_aligned(),
                    Line::from(map.name().into_owned()).centered(),
                    Line::from(map.content().map_or(String::new(), |c| c.code.clone())).centered(),
                ]))
            })
    }
}

impl StatefulWidget for MapWidget {
    type State = MapState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let visible = state.visible_spans(area);
        let selected = visible.selected_cell_in_viewport(state.selected);
        let mut table_state = TableState::new().with_selected_cell(selected);
        let map_size = Size::new(
            cells_to_terminal_length(state.maps.width() as usize, CELL_WIDTH),
            cells_to_terminal_length(state.maps.height() as usize, CELL_HEIGHT),
        );
        let mut scroll_view = ScrollView::new(map_size);

        scroll_view.render_stateful_widget(
            Self::construct_table(&state.maps, &visible),
            visible.table_area(),
            &mut table_state,
        );
        scroll_view.render(area, buf, &mut state.scroll_view_state);
    }
}

pub struct MapState {
    maps: MapsClient,
    selected: MapCellIndex,
    scroll_view_state: ScrollViewState,
}

impl MapState {
    #[must_use]
    pub fn new(maps: MapsClient) -> Self {
        Self {
            maps,
            selected: MapCellIndex::default(),
            scroll_view_state: ScrollViewState::new(),
        }
    }

    fn visible_spans(&mut self, area: Rect) -> VisibleSpans {
        let offset = self.scroll_view_state.offset();
        let columns = visible_span(
            self.maps.width() as usize,
            self.selected.column,
            offset.x,
            area.width,
            CELL_WIDTH,
        );
        let rows = visible_span(
            self.maps.height() as usize,
            self.selected.row,
            offset.y,
            area.height,
            CELL_HEIGHT,
        );
        self.scroll_view_state.set_offset(Position::new(
            cells_to_terminal_length(columns.start, CELL_WIDTH),
            cells_to_terminal_length(rows.start, CELL_HEIGHT),
        ));

        VisibleSpans { columns, rows }
    }

    pub const fn move_left(&mut self) {
        self.selected.column = self.selected.column.saturating_sub(1);
    }

    pub const fn move_up(&mut self) {
        self.selected.row = self.selected.row.saturating_sub(1);
    }

    pub fn move_down(&mut self) {
        self.selected.row = self
            .selected
            .row
            .saturating_add(1)
            .min((self.maps.height() as usize).saturating_sub(1));
    }

    pub fn move_right(&mut self) {
        self.selected.column = self
            .selected
            .column
            .saturating_add(1)
            .min((self.maps.width() as usize).saturating_sub(1));
    }
}

struct VisibleSpans {
    columns: Range<usize>,
    rows: Range<usize>,
}

impl VisibleSpans {
    fn table_area(&self) -> Rect {
        Rect::new(
            cells_to_terminal_length(self.columns.start, CELL_WIDTH),
            cells_to_terminal_length(self.rows.start, CELL_HEIGHT),
            cells_to_terminal_length(self.columns.len(), CELL_WIDTH),
            cells_to_terminal_length(self.rows.len(), CELL_HEIGHT),
        )
    }

    /// Converts a world-map selection into indices relative to the visible table.
    fn selected_cell_in_viewport(&self, selected: MapCellIndex) -> Option<(usize, usize)> {
        (self.rows.contains(&selected.row) && self.columns.contains(&selected.column)).then_some((
            selected.row - self.rows.start,
            selected.column - self.columns.start,
        ))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Zero-based row and column indices within the map bounds, not world coordinates.
struct MapCellIndex {
    row: usize,
    column: usize,
}

/// Returns the range of map-cell indices to render along one viewport axis.
///
/// # Arguments
///
/// * `total` - Total number of map cells along this axis.
/// * `selected` - Index of the selected map cell along this axis.
/// * `offset` - Current scroll offset in terminal columns or rows.
/// * `viewport` - Available viewport length in terminal columns or rows.
/// * `cell_size` - Length of one map cell in terminal columns or rows.
fn visible_span(
    total: usize,
    selected: usize,
    offset: u16,
    viewport: u16,
    cell_size: u16,
) -> Range<usize> {
    let visible = usize::from(viewport / cell_size).max(1);
    let current = usize::from(offset / cell_size);
    let start = first_visible_cell(current, selected, total, visible);
    start..start.saturating_add(visible).min(total)
}

/// Returns the first visible cell index while keeping the selection in view.
///
/// # Arguments
///
/// * `current_first` - Index of the cell currently at the viewport's leading edge.
/// * `selected` - Index of the cell that must remain visible.
/// * `total_cells` - Total number of cells along this map axis.
/// * `visible_cells` - Number of complete cells that fit along this viewport axis.
fn first_visible_cell(
    current_first: usize,
    selected: usize,
    total_cells: usize,
    visible_cells: usize,
) -> usize {
    let max_first = total_cells.saturating_sub(visible_cells);
    let current_first = current_first.min(max_first);

    if selected < current_first {
        selected
    } else if selected >= current_first.saturating_add(visible_cells) {
        selected
            .saturating_add(1)
            .saturating_sub(visible_cells)
            .min(max_first)
    } else {
        current_first
    }
}

/// Converts a cell count to terminal columns or rows, clamped to Ratatui's `u16` limit.
fn cells_to_terminal_length(cell_count: usize, cell_length: u16) -> u16 {
    cell_count
        .saturating_mul(usize::from(cell_length))
        .min(usize::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::{CELL_HEIGHT, CELL_WIDTH, MapCellIndex, MapState};
    use ratatui::prelude::{Position, Rect};
    use sdk::test_utils::MAPS;

    #[test]
    fn movement_stops_at_map_boundaries() {
        let mut state = MapState::new(MAPS.clone());

        state.move_left();
        state.move_up();
        assert_eq!(state.selected, MapCellIndex { row: 0, column: 0 });

        for _ in 0..=state.maps.width() {
            state.move_right();
        }
        for _ in 0..=state.maps.height() {
            state.move_down();
        }

        assert_eq!(state.selected.column, state.maps.width() as usize - 1);
        assert_eq!(state.selected.row, state.maps.height() as usize - 1);
    }

    #[test]
    fn viewport_follows_selection_and_backfills_at_map_edge() {
        let mut state = MapState::new(MAPS.clone());
        state.selected = MapCellIndex {
            row: state.maps.height() as usize - 1,
            column: state.maps.width() as usize - 1,
        };

        let visible = state.visible_spans(Rect::new(0, 0, CELL_WIDTH * 2, CELL_HEIGHT * 2));

        assert_eq!(visible.columns.end, state.maps.width() as usize);
        assert_eq!(visible.rows.end, state.maps.height() as usize);
        assert!(visible.columns.contains(&state.selected.column));
        assert!(visible.rows.contains(&state.selected.row));
    }

    #[test]
    fn viewport_resets_when_entire_map_fits() {
        let mut state = MapState::new(MAPS.clone());
        state
            .scroll_view_state
            .set_offset(Position::new(CELL_WIDTH * 2, CELL_HEIGHT * 2));

        state.visible_spans(Rect::new(0, 0, u16::MAX, u16::MAX));

        assert_eq!(state.scroll_view_state.offset(), Position::ORIGIN);
    }
}
