use crate::core::render::phase::RenderPhase;
use crate::core::render::phases::seam::POWERLINE_ARROW;
use crate::core::render::state::RenderState;

/// The final breath: fragments and seams interleave into one line of text.
pub struct ExhalePhase;

impl RenderPhase for ExhalePhase {
    fn name(&self) -> &'static str {
        "exhale"
    }

    fn pass_through(&self, state: &mut RenderState) {
        if state.fragments.is_empty() {
            state.line = String::new();
            return;
        }

        let mut line = state.fragments[0].body.clone();
        for (fragment, seam) in state.fragments[1..].iter().zip(&state.seams) {
            line.push_str(seam);
            line.push_str(&fragment.body);
        }

        if state.config.style.separator == POWERLINE_ARROW && state.fragments.len() > 1 {
            line.push_str("\x1b[0m");
        }

        state.line = line;
    }
}
