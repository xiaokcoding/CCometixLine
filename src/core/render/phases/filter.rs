use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

/// Drops disabled segments so only enabled ones are rendered.
pub struct FilterPhase;

impl RenderPhase for FilterPhase {
    fn apply(&self, state: &mut RenderState) {
        state.segments.retain(|(config, _)| config.enabled);
    }
}
