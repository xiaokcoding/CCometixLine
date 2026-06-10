use crate::core::render::phase::RenderPhase;
use crate::core::render::state::RenderState;

/// Wakes the frame up: only enabled segments get to participate in rendering.
pub struct AwakeningPhase;

impl RenderPhase for AwakeningPhase {
    fn name(&self) -> &'static str {
        "awakening"
    }

    fn pass_through(&self, state: &mut RenderState) {
        state.segments.retain(|(config, _)| config.enabled);
    }
}
