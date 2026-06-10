use crate::config::{Config, SegmentConfig};
use crate::core::segments::SegmentData;

/// A single rendered piece of the statusline, still aware of the segment
/// configuration it was born from so later phases can reason about color
/// transitions and widths.
pub struct Fragment {
    pub body: String,
    pub config: SegmentConfig,
}

/// Everything a frame knows about itself while it moves through the pipeline.
///
/// Each phase reads what earlier phases left behind and settles its own
/// contribution into place: `segments` feed `fragments`, fragments grow
/// `seams` between them, and the whole thing condenses into `line`.
pub struct RenderState {
    pub config: Config,
    pub segments: Vec<(SegmentConfig, SegmentData)>,
    pub fragments: Vec<Fragment>,
    pub seams: Vec<String>,
    pub line: String,
    pub horizon: Option<usize>,
}

impl RenderState {
    pub fn new(config: Config, segments: Vec<(SegmentConfig, SegmentData)>) -> Self {
        Self {
            config,
            segments,
            fragments: Vec::new(),
            seams: Vec::new(),
            line: String::new(),
            horizon: None,
        }
    }

    /// Give the frame an awareness of how wide the terminal is, so the
    /// horizon phase can dissolve whatever would not fit.
    pub fn with_horizon(mut self, horizon: Option<usize>) -> Self {
        self.horizon = horizon;
        self
    }
}
