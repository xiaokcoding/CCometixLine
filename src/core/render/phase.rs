use super::state::RenderState;

/// One stage of the rendering state machine.
///
/// A phase looks at the [`RenderState`], transforms the part of the frame it
/// is responsible for, and lets the state flow on to the next phase.
pub trait RenderPhase {
    fn apply(&self, state: &mut RenderState);
}

/// An ordered sequence of phases that a frame passes through.
#[derive(Default)]
pub struct RenderPipeline {
    phases: Vec<Box<dyn RenderPhase>>,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn then(mut self, phase: impl RenderPhase + 'static) -> Self {
        self.phases.push(Box::new(phase));
        self
    }

    /// Run the frame through every phase in order.
    pub fn run(&self, state: &mut RenderState) {
        for phase in &self.phases {
            phase.apply(state);
        }
    }
}
