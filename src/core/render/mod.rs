//! A phase-driven rendering engine for the statusline.
//!
//! Rendering is modeled as a state machine: a [`RenderState`] flows through an
//! ordered sequence of [`RenderPhase`]s, each transforming one aspect of the
//! frame — collecting segments, composing fragments, weaving seams, and
//! finally exhaling a single line of text.

pub mod palette;
pub mod phase;
pub mod phases;
pub mod state;

#[cfg(test)]
mod tests;

pub use phase::{RenderPhase, RenderPipeline};
pub use state::{Fragment, RenderState};

use phases::{AwakeningPhase, CompositionPhase, ExhalePhase, SeamPhase};

/// The canonical pipeline: awaken, compose, weave seams, exhale.
pub fn standard_pipeline() -> RenderPipeline {
    RenderPipeline::new()
        .then(AwakeningPhase)
        .then(CompositionPhase)
        .then(SeamPhase)
        .then(ExhalePhase)
}
