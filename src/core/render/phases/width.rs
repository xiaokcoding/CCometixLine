use crate::core::render::palette::visible_width;
use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

/// Keeps the frame within the terminal width.
///
/// When the state knows how wide the terminal is, fragments that would not
/// fit are dropped from the end — segments earlier in the configuration
/// survive longest.
pub struct WidthPhase;

impl RenderPhase for WidthPhase {
    fn apply(&self, state: &mut RenderState) {
        if let Some(max_width) = state.max_width {
            truncate_to_width(state, max_width);
        }
    }
}

/// Pop fragments (and the separators that led to them) off the end of the
/// frame until what remains fits within `max_width`. At least one fragment
/// always survives so the statusline never disappears entirely.
pub fn truncate_to_width(state: &mut RenderState, max_width: usize) {
    while state.fragments.len() > 1 && frame_width(state) > max_width {
        state.fragments.pop();
        state.separators.pop();
    }
}

fn frame_width(state: &RenderState) -> usize {
    let fragments: usize = state.fragments.iter().map(|f| visible_width(&f.body)).sum();
    let separators: usize = state.separators.iter().map(|s| visible_width(s)).sum();
    fragments + separators
}

/// The terminal width Claude Code hands to statusline commands via the
/// `COLUMNS` environment variable, when present and meaningful.
pub fn terminal_width() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|w| *w > 0)
}
