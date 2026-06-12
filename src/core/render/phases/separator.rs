use crate::config::AnsiColor;
use crate::core::render::palette;
use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

pub const POWERLINE_ARROW: &str = "\u{e0b0}";

/// Builds a separator between every pair of neighbouring fragments — either a
/// powerline arrow carrying the color transition, or a plain separator.
pub struct SeparatorPhase;

impl RenderPhase for SeparatorPhase {
    fn apply(&self, state: &mut RenderState) {
        build_separators(state);
    }
}

/// (Re)build the separator list from the current fragments. Called by the
/// phase and again after the width phase removes fragments, so powerline
/// color transitions stay correct around removed neighbours.
pub fn build_separators(state: &mut RenderState) {
    state.separators.clear();
    for i in 0..state.fragments.len().saturating_sub(1) {
        let rendered = render_separator(
            &state.config.style.separator,
            state.fragments[i].background.as_ref(),
            state.fragments[i + 1].background.as_ref(),
        );
        state.separators.push(rendered);
    }
}

/// Render the separator between two fragments given their background colors.
pub fn render_separator(
    separator: &str,
    prev_bg: Option<&AnsiColor>,
    curr_bg: Option<&AnsiColor>,
) -> String {
    if separator == POWERLINE_ARROW {
        powerline_arrow(prev_bg, curr_bg)
    } else {
        format!("\x1b[37m{}\x1b[0m", separator)
    }
}

/// An arrow whose foreground inherits the previous fragment's background and
/// whose background takes on the next fragment's, so the colors flow into
/// each other.
pub fn powerline_arrow(prev_bg: Option<&AnsiColor>, curr_bg: Option<&AnsiColor>) -> String {
    match (prev_bg, curr_bg) {
        (Some(prev), Some(curr)) => format!(
            "{}{}{}\x1b[0m",
            palette::background(curr),
            palette::foreground(prev),
            POWERLINE_ARROW
        ),
        (Some(prev), None) => format!("{}{}\x1b[0m", palette::foreground(prev), POWERLINE_ARROW),
        (None, Some(curr)) => format!("{}{}\x1b[0m", palette::background(curr), POWERLINE_ARROW),
        (None, None) => POWERLINE_ARROW.to_string(),
    }
}
