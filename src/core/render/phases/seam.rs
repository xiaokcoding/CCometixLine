use crate::config::AnsiColor;
use crate::core::render::palette;
use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

pub const POWERLINE_ARROW: &str = "\u{e0b0}";

/// Weaves a seam between every pair of neighbouring fragments — either a
/// powerline arrow carrying the color transition, or a plain separator.
pub struct SeamPhase;

impl RenderPhase for SeamPhase {
    fn name(&self) -> &'static str {
        "seam"
    }

    fn pass_through(&self, state: &mut RenderState) {
        let separator = state.config.style.separator.clone();

        for i in 0..state.fragments.len().saturating_sub(1) {
            let seam = if separator == POWERLINE_ARROW {
                let prev_bg = state.fragments[i].config.colors.background.as_ref();
                let curr_bg = state.fragments[i + 1].config.colors.background.as_ref();
                powerline_arrow(prev_bg, curr_bg)
            } else {
                format!("\x1b[37m{}\x1b[0m", separator)
            };
            state.seams.push(seam);
        }
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
