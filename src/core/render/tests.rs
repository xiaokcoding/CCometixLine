use super::*;
use crate::config::{
    AnsiColor, ColorConfig, Config, IconConfig, SegmentConfig, SegmentId, StyleConfig, StyleMode,
    TextStyleConfig,
};
use crate::core::segments::SegmentData;
use std::collections::HashMap;

fn config_with_separator(separator: &str) -> Config {
    Config {
        style: StyleConfig {
            mode: StyleMode::Plain,
            separator: separator.to_string(),
        },
        segments: vec![],
        theme: "default".to_string(),
    }
}

fn segment(
    id: SegmentId,
    enabled: bool,
    primary: &str,
    background: Option<AnsiColor>,
) -> (SegmentConfig, SegmentData) {
    (
        SegmentConfig {
            id,
            enabled,
            icon: IconConfig {
                plain: "*".to_string(),
                nerd_font: "*".to_string(),
            },
            colors: ColorConfig {
                icon: None,
                text: None,
                background,
            },
            styles: TextStyleConfig::default(),
            options: HashMap::new(),
        },
        SegmentData {
            primary: primary.to_string(),
            secondary: String::new(),
            metadata: HashMap::new(),
        },
    )
}

fn run(config: Config, segments: Vec<(SegmentConfig, SegmentData)>) -> RenderState {
    let mut state = RenderState::new(config, segments);
    standard_pipeline().breathe_between_frames(&mut state);
    state
}

#[test]
fn awakening_drops_disabled_segments() {
    let state = run(
        config_with_separator("|"),
        vec![
            segment(SegmentId::Model, true, "sonnet", None),
            segment(SegmentId::Git, false, "main", None),
        ],
    );
    assert_eq!(state.fragments.len(), 1);
    assert!(state.line.contains("sonnet"));
    assert!(!state.line.contains("main"));
}

#[test]
fn empty_frame_exhales_nothing() {
    let state = run(config_with_separator("|"), vec![]);
    assert_eq!(state.line, "");
}

#[test]
fn plain_separator_is_painted_white() {
    let state = run(
        config_with_separator("|"),
        vec![
            segment(SegmentId::Model, true, "a", None),
            segment(SegmentId::Git, true, "b", None),
        ],
    );
    assert_eq!(state.seams.len(), 1);
    assert_eq!(state.seams[0], "\x1b[37m|\x1b[0m");
    assert!(!state.line.ends_with("\x1b[0m"));
}

#[test]
fn powerline_arrow_carries_color_transition() {
    let prev = AnsiColor::Color256 { c256: 1 };
    let curr = AnsiColor::Color256 { c256: 2 };
    let arrow = phases::powerline_arrow(Some(&prev), Some(&curr));
    assert_eq!(arrow, "\x1b[48;5;2m\x1b[38;5;1m\u{e0b0}\x1b[0m");
    assert_eq!(phases::powerline_arrow(None, None), "\u{e0b0}");
}

#[test]
fn powerline_frame_resets_at_the_end() {
    let state = run(
        config_with_separator("\u{e0b0}"),
        vec![
            segment(
                SegmentId::Model,
                true,
                "a",
                Some(AnsiColor::Color256 { c256: 1 }),
            ),
            segment(
                SegmentId::Git,
                true,
                "b",
                Some(AnsiColor::Color256 { c256: 2 }),
            ),
        ],
    );
    assert!(state.line.ends_with("\x1b[0m"));
    assert!(state.seams[0].contains("\u{e0b0}"));
}

#[test]
fn background_wraps_the_whole_fragment() {
    let state = run(
        config_with_separator("|"),
        vec![segment(
            SegmentId::Model,
            true,
            "a",
            Some(AnsiColor::Color256 { c256: 4 }),
        )],
    );
    assert!(state.line.starts_with("\x1b[48;5;4m"));
    assert!(state.line.ends_with("\x1b[49m"));
}

#[test]
fn visible_width_ignores_escape_sequences() {
    assert_eq!(palette::visible_width("\x1b[38;5;1mabc\x1b[0m"), 3);
    assert_eq!(palette::visible_width("abc"), 3);
    assert_eq!(palette::visible_width(""), 0);
}
