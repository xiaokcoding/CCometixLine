//! A phase-driven rendering engine for the statusline.
//!
//! Rendering is modeled as a state machine: a [`RenderState`] flows through an
//! ordered sequence of [`RenderPhase`]s, each transforming one aspect of the
//! frame — collecting segments, composing fragments, weaving seams, and
//! finally exhaling a single line of text.

pub mod loom;
pub mod palette;
pub mod phase;
pub mod phases;
pub mod state;

#[cfg(test)]
mod tests;

pub use phase::{RenderPhase, RenderPipeline};
pub use state::{Fragment, RenderState};

pub use phases::terminal_horizon;

use phases::{AwakeningPhase, CompositionPhase, ExhalePhase, HorizonPhase, SeamPhase};

/// The canonical pipeline: awaken, compose, weave seams, honor the horizon,
/// exhale.
pub fn standard_pipeline() -> RenderPipeline {
    composition_pipeline().then(HorizonPhase).then(ExhalePhase)
}

/// The pipeline up to (but not including) the final exhale, for callers that
/// want to lay fragments out themselves — e.g. wrapping across lines.
pub fn composition_pipeline() -> RenderPipeline {
    RenderPipeline::new()
        .then(AwakeningPhase)
        .then(CompositionPhase)
        .then(SeamPhase)
}
