//! A phase-driven rendering engine for the statusline.
//!
//! Rendering is modeled as a state machine: a [`RenderState`] flows through an
//! ordered sequence of [`RenderPhase`]s, each transforming one aspect of the
//! frame — filtering segments, composing fragments, building separators, and
//! finally joining everything into a single line of text.

pub mod palette;
pub mod phase;
pub mod phases;
pub mod state;
pub mod wrap;

#[cfg(test)]
mod tests;

pub use phase::{RenderPhase, RenderPipeline};
pub use state::{Fragment, RenderState};

pub use phases::{terminal_lines, terminal_width};

use phases::{CompositionPhase, FilterPhase, JoinPhase, SeparatorPhase, WidthPhase};

/// The canonical pipeline: filter, compose, separate, fit the width, join.
pub fn standard_pipeline() -> RenderPipeline {
    composition_pipeline().then(WidthPhase).then(JoinPhase)
}

/// The pipeline up to (but not including) the final join, for callers that
/// want to lay fragments out themselves — e.g. wrapping across lines.
pub fn composition_pipeline() -> RenderPipeline {
    RenderPipeline::new()
        .then(FilterPhase)
        .then(CompositionPhase)
        .then(SeparatorPhase)
}
