use crate::config::{Config, SegmentConfig};
use crate::core::render::{composition_pipeline, standard_pipeline, wrap, RenderState};
use crate::core::segments::SegmentData;

/// A thin facade over the phase-driven render pipeline in [`crate::core::render`].
pub struct StatusLineGenerator {
    config: Config,
}

impl StatusLineGenerator {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate(&self, segments: Vec<(SegmentConfig, SegmentData)>) -> String {
        self.generate_with_width(segments, None)
    }

    /// Render the statusline within an optional terminal width: segments that
    /// would not fit are dropped from the end.
    pub fn generate_with_width(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: Option<usize>,
    ) -> String {
        let mut state = RenderState::new(self.config.clone(), segments).with_max_width(max_width);
        standard_pipeline().run(&mut state);
        state.line
    }

    /// Render the statusline wrapped across up to `max_lines` lines of at
    /// most `max_width` columns each, instead of truncating.
    pub fn generate_multiline(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: usize,
        max_lines: usize,
    ) -> String {
        let mut state = RenderState::new(self.config.clone(), segments);
        composition_pipeline().run(&mut state);

        let lines = wrap::wrap_fragments(&state, max_width);
        lines
            .into_iter()
            .take(max_lines.max(1))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Generate statusline for TUI preview with proper width calculation
    /// This method handles ANSI escape sequences properly for ratatui rendering
    pub fn generate_for_tui(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
    ) -> ratatui::text::Line<'static> {
        use ansi_to_tui::IntoText;
        use ratatui::text::{Line, Span};

        let full_output = self.generate(segments);

        if let Ok(text) = full_output.into_text() {
            if let Some(line) = text.lines.into_iter().next() {
                return line;
            }
        }

        Line::from(vec![Span::raw(full_output)])
    }

    /// Generate TUI-optimized text with intelligent wrapping by segment for preview
    pub fn generate_for_tui_preview(
        &self,
        segments: Vec<(SegmentConfig, SegmentData)>,
        max_width: u16,
    ) -> ratatui::text::Text<'_> {
        use ansi_to_tui::IntoText;
        use ratatui::text::{Line, Span, Text};

        let mut state = RenderState::new(self.config.clone(), segments);
        composition_pipeline().run(&mut state);

        let lines = wrap::wrap_fragments(&state, max_width as usize);

        let mut tui_lines = Vec::new();
        for line in lines {
            match line.as_str().into_text() {
                Ok(text) => tui_lines.extend(text.lines),
                Err(_) => tui_lines.push(Line::from(vec![Span::raw(line)])),
            }
        }

        if tui_lines.is_empty() {
            tui_lines.push(Line::default());
        }

        Text::from(tui_lines)
    }
}

pub fn collect_all_segments(
    config: &Config,
    input: &crate::config::InputData,
) -> Vec<(SegmentConfig, SegmentData)> {
    use crate::core::segments::*;

    let mut results = Vec::new();

    for segment_config in &config.segments {
        // Skip disabled segments to avoid unnecessary API requests
        if !segment_config.enabled {
            continue;
        }

        let segment_data = match segment_config.id {
            crate::config::SegmentId::Model => {
                let segment = ModelSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Directory => {
                let segment = DirectorySegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Git => {
                let show_sha = segment_config
                    .options
                    .get("show_sha")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let segment = GitSegment::new().with_sha(show_sha);
                segment.collect(input)
            }
            crate::config::SegmentId::ContextWindow => {
                let segment = ContextWindowSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Usage => {
                let segment = UsageSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Cost => {
                let segment = CostSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Session => {
                let segment = SessionSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::OutputStyle => {
                let segment = OutputStyleSegment::new();
                segment.collect(input)
            }
            crate::config::SegmentId::Update => {
                let segment = UpdateSegment::new();
                segment.collect(input)
            }
        };

        if let Some(data) = segment_data {
            results.push((segment_config.clone(), data));
        }
    }

    results
}
