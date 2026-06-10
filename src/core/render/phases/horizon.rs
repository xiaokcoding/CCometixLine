use crate::core::render::palette::visible_width;
use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

/// Keeps the frame within the terminal's horizon.
///
/// When the state knows how wide the terminal is, fragments that would spill
/// past the edge dissolve, least important last — segments earlier in the
/// configuration survive longest.
pub struct HorizonPhase;

impl RenderPhase for HorizonPhase {
    fn name(&self) -> &'static str {
        "horizon"
    }

    fn pass_through(&self, state: &mut RenderState) {
        if let Some(horizon) = state.horizon {
            dissolve_stale_fragments(state, horizon);
        }
    }
}

/// Pop fragments (and the seams that led to them) off the end of the frame
/// until what remains fits within the horizon. At least one fragment always
/// survives so the statusline never goes dark entirely.
pub fn dissolve_stale_fragments(state: &mut RenderState, horizon: usize) {
    while state.fragments.len() > 1 && frame_width(state) > horizon {
        state.fragments.pop();
        state.seams.pop();
    }
}

fn frame_width(state: &RenderState) -> usize {
    let fragments: usize = state.fragments.iter().map(|f| visible_width(&f.body)).sum();
    let seams: usize = state.seams.iter().map(|s| visible_width(s)).sum();
    fragments + seams
}

/// The terminal width Claude Code hands to statusline commands via the
/// `COLUMNS` environment variable, when present and meaningful.
pub fn terminal_horizon() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|w| *w > 0)
}
