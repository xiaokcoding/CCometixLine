use crate::config::{WidthConfig, WidthMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// Popup editor for the global `[width]` config block.
///
/// Four rows — mode, reserve, adaptive threshold, max lines — selected with
/// ↑↓ and adjusted in place with ←→. Enter applies, Esc discards.
pub struct WidthSettingsComponent {
    pub is_open: bool,
    draft: WidthConfig,
    selected_row: usize,
}

const ROWS: usize = 4;

impl Default for WidthSettingsComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl WidthSettingsComponent {
    pub fn new() -> Self {
        Self {
            is_open: false,
            draft: WidthConfig::default(),
            selected_row: 0,
        }
    }

    pub fn open(&mut self, current: &WidthConfig) {
        self.is_open = true;
        self.draft = current.clone();
        self.selected_row = 0;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// The edited config, to apply on Enter.
    pub fn get_config(&self) -> WidthConfig {
        self.draft.clone()
    }

    pub fn move_selection(&mut self, delta: i32) {
        self.selected_row = (self.selected_row as i32 + delta).rem_euclid(ROWS as i32) as usize;
    }

    /// Adjust the selected row's value: mode cycles, numbers step (threshold
    /// in fives), all floored at sensible minimums.
    pub fn adjust(&mut self, delta: i32) {
        match self.selected_row {
            0 => {
                self.draft.mode = match (self.draft.mode, delta >= 0) {
                    (WidthMode::Full, true) => WidthMode::Reserve,
                    (WidthMode::Reserve, true) => WidthMode::Adaptive,
                    (WidthMode::Adaptive, true) => WidthMode::Full,
                    (WidthMode::Full, false) => WidthMode::Adaptive,
                    (WidthMode::Reserve, false) => WidthMode::Full,
                    (WidthMode::Adaptive, false) => WidthMode::Reserve,
                };
            }
            1 => {
                self.draft.reserve = step_usize(self.draft.reserve, delta, 0, 200);
            }
            2 => {
                self.draft.adaptive_threshold =
                    (self.draft.adaptive_threshold + 5.0 * delta as f64).clamp(0.0, 100.0);
            }
            _ => {
                self.draft.max_lines = step_usize(self.draft.max_lines, delta, 1, 10);
            }
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if !self.is_open {
            return;
        }

        let popup_width = 56_u16.min(area.width.saturating_sub(4));
        let popup_height = 11_u16;
        let popup_area = Rect {
            x: (area.width.saturating_sub(popup_width)) / 2,
            y: (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, popup_area);
        let popup_block = Block::default()
            .borders(Borders::ALL)
            .title("Width Settings");
        let inner = popup_block.inner(popup_area);
        f.render_widget(popup_block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(6), Constraint::Length(3)])
            .split(inner);

        let mode = match self.draft.mode {
            WidthMode::Full => "full      — use all of COLUMNS",
            WidthMode::Reserve => "reserve   — keep `reserve` columns free",
            WidthMode::Adaptive => "adaptive  — full until the threshold",
        };
        let rows = [
            format!("Mode:               ◀ {} ▶", mode),
            format!("Reserve columns:    ◀ {} ▶", self.draft.reserve),
            format!("Adaptive threshold: ◀ {}% ▶", self.draft.adaptive_threshold),
            format!("Max lines:          ◀ {} ▶", self.draft.max_lines),
        ];
        let lines: Vec<Line> = rows
            .iter()
            .enumerate()
            .map(|(i, row)| {
                if i == self.selected_row {
                    Line::from(Span::styled(
                        format!("▶ {}", row),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::raw(format!("  {}", row)))
                }
            })
            .collect();
        f.render_widget(Paragraph::new(lines), chunks[0]);

        f.render_widget(
            Paragraph::new("[↑↓] Field  [←→] Adjust  [Enter] Apply  [Esc] Cancel")
                .block(Block::default().borders(Borders::ALL)),
            chunks[1],
        );
    }
}

fn step_usize(value: usize, delta: i32, min: usize, max: usize) -> usize {
    let next = value as i64 + delta as i64;
    next.clamp(min as i64, max as i64) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_cycles_in_both_directions() {
        let mut editor = WidthSettingsComponent::new();
        editor.open(&WidthConfig::default());
        assert_eq!(editor.get_config().mode, WidthMode::Reserve);
        editor.adjust(1);
        assert_eq!(editor.get_config().mode, WidthMode::Adaptive);
        editor.adjust(1);
        assert_eq!(editor.get_config().mode, WidthMode::Full);
        editor.adjust(-1);
        assert_eq!(editor.get_config().mode, WidthMode::Adaptive);
    }

    #[test]
    fn numeric_rows_step_within_bounds() {
        let mut editor = WidthSettingsComponent::new();
        editor.open(&WidthConfig::default());
        editor.move_selection(1); // reserve
        editor.adjust(1);
        assert_eq!(editor.get_config().reserve, 41);
        editor.move_selection(1); // threshold
        editor.adjust(-1);
        assert_eq!(editor.get_config().adaptive_threshold, 55.0);
        editor.move_selection(1); // max_lines
        editor.adjust(-5);
        assert_eq!(editor.get_config().max_lines, 1); // floored
    }

    #[test]
    fn selection_wraps_around() {
        let mut editor = WidthSettingsComponent::new();
        editor.open(&WidthConfig::default());
        editor.move_selection(-1);
        editor.adjust(1); // adjust max_lines, not a panic / wrong row
        assert_eq!(editor.get_config().max_lines, 2);
    }
}
