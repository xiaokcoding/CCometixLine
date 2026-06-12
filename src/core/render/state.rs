use crate::config::{AnsiColor, Config, SegmentConfig};
use crate::core::segments::SegmentData;

/// A single rendered piece of the statusline, keeping the background color it
/// was rendered with (for separator color transitions) and its truncation
/// priority (higher survives longer when the terminal is narrow).
pub struct Fragment {
    pub body: String,
    pub background: Option<AnsiColor>,
    pub priority: i64,
}

/// Everything a frame knows about itself while it moves through the pipeline.
///
/// Each phase reads what earlier phases left behind and adds its own
/// contribution: `segments` feed `fragments`, fragments get `separators`
/// between them, and the whole thing is joined into `line`.
pub struct RenderState {
    pub config: Config,
    pub segments: Vec<(SegmentConfig, SegmentData)>,
    pub fragments: Vec<Fragment>,
    pub separators: Vec<String>,
    pub line: String,
    pub max_width: Option<usize>,
}

impl RenderState {
    pub fn new(config: Config, segments: Vec<(SegmentConfig, SegmentData)>) -> Self {
        Self {
            config,
            segments,
            fragments: Vec::new(),
            separators: Vec::new(),
            line: String::new(),
            max_width: None,
        }
    }

    /// Set the terminal width budget the width phase truncates to.
    pub fn with_max_width(mut self, max_width: Option<usize>) -> Self {
        self.max_width = max_width;
        self
    }
}
