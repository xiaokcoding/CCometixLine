use crate::core::render::phase::RenderPhase;
use crate::core::render::phases::separator::POWERLINE_ARROW;
use crate::core::render::state::RenderState;

/// Joins fragments and separators into one line of text.
pub struct JoinPhase;

impl RenderPhase for JoinPhase {
    fn apply(&self, state: &mut RenderState) {
        if state.fragments.is_empty() {
            state.line = String::new();
            return;
        }

        let mut line = state.fragments[0].body.clone();
        for (fragment, separator) in state.fragments[1..].iter().zip(&state.separators) {
            line.push_str(separator);
            line.push_str(&fragment.body);
        }

        if state.config.style.separator == POWERLINE_ARROW && state.fragments.len() > 1 {
            line.push_str("\x1b[0m");
        }

        state.line = line;
    }
}
