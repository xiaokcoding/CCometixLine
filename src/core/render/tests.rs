use super::*;
use crate::config::{
    AnsiColor, ColorConfig, Config, IconConfig, SegmentConfig, SegmentId, StyleConfig, StyleMode,
    TextStyleConfig, WidthConfig,
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
        width: WidthConfig::default(),
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
    standard_pipeline().run(&mut state);
    state
}

#[test]
fn filter_drops_disabled_segments() {
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
fn empty_frame_renders_nothing() {
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
    assert_eq!(state.separators.len(), 1);
    assert_eq!(state.separators[0], "\x1b[37m|\x1b[0m");
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
    assert!(state.separators[0].contains("\u{e0b0}"));
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
fn wrap_breaks_fragments_when_the_width_runs_out() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![
            segment(SegmentId::Model, true, "aaaa", None),
            segment(SegmentId::Git, true, "bbbb", None),
        ],
    );
    composition_pipeline().run(&mut state);

    let wide = wrap::wrap_fragments(&state, 80);
    assert_eq!(wide.len(), 1);
    assert!(wide[0].contains("\x1b[37m|\x1b[0m"));

    let narrow = wrap::wrap_fragments(&state, 8);
    assert_eq!(narrow.len(), 2);
    assert!(!narrow[0].contains('|'));
}

#[test]
fn wrap_keeps_everything_on_one_line_when_it_fits() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![segment(SegmentId::Model, true, "a", None)],
    );
    composition_pipeline().run(&mut state);
    assert_eq!(wrap::wrap_fragments(&state, 80).len(), 1);
    assert!(wrap::wrap_fragments(&state, 80)[0].contains('a'));
}

#[test]
fn width_phase_drops_fragments_that_spill_past_the_edge() {
    let segments = vec![
        segment(SegmentId::Model, true, "aaaa", None),
        segment(SegmentId::Git, true, "bbbb", None),
        segment(SegmentId::Directory, true, "cccc", None),
    ];

    let mut state =
        RenderState::new(config_with_separator("|"), segments.clone()).with_max_width(Some(14));
    standard_pipeline().run(&mut state);
    assert_eq!(state.fragments.len(), 2);
    assert!(state.line.contains("aaaa"));
    assert!(state.line.contains("bbbb"));
    assert!(!state.line.contains("cccc"));

    let mut wide = RenderState::new(config_with_separator("|"), segments).with_max_width(Some(120));
    standard_pipeline().run(&mut wide);
    assert_eq!(wide.fragments.len(), 3);
}

#[test]
fn width_phase_never_drops_the_last_fragment() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![segment(
            SegmentId::Model,
            true,
            "a very long fragment",
            None,
        )],
    )
    .with_max_width(Some(3));
    standard_pipeline().run(&mut state);
    assert_eq!(state.fragments.len(), 1);
    assert!(!state.line.is_empty());
}

#[test]
fn no_max_width_means_no_truncation() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![
            segment(SegmentId::Model, true, "aaaa", None),
            segment(SegmentId::Git, true, "bbbb", None),
        ],
    );
    standard_pipeline().run(&mut state);
    assert_eq!(state.fragments.len(), 2);
}

#[test]
fn visible_width_ignores_escape_sequences() {
    assert_eq!(palette::visible_width("\x1b[38;5;1mabc\x1b[0m"), 3);
    assert_eq!(palette::visible_width("abc"), 3);
    assert_eq!(palette::visible_width(""), 0);
}

#[test]
fn visible_width_counts_wide_characters_as_two_columns() {
    assert_eq!(palette::visible_width("你好"), 4);
    assert_eq!(palette::visible_width("\x1b[38;5;1m你好\x1b[0m"), 4);
    assert_eq!(palette::visible_width("a你b"), 4);
}

#[test]
fn width_phase_budgets_cjk_fragments_correctly() {
    // Each fragment body is "* 你好" = 1 (icon) + 1 (space) + 4 (CJK) = 6 cols;
    // separator "|" adds 1. Two fragments + separator = 13 columns.
    let segments = vec![
        segment(SegmentId::Model, true, "你好", None),
        segment(SegmentId::Git, true, "你好", None),
    ];

    let mut fits =
        RenderState::new(config_with_separator("|"), segments.clone()).with_max_width(Some(13));
    standard_pipeline().run(&mut fits);
    assert_eq!(fits.fragments.len(), 2);

    let mut tight = RenderState::new(config_with_separator("|"), segments).with_max_width(Some(12));
    standard_pipeline().run(&mut tight);
    // The second fragment no longer fits whole: it comes back truncated.
    assert!(tight.line.contains('…'));
    assert!(palette::visible_width(&tight.line) <= 12);
}

fn segment_with_priority(
    id: SegmentId,
    primary: &str,
    priority: i64,
) -> (SegmentConfig, SegmentData) {
    let (mut config, data) = segment(id, true, primary, None);
    config
        .options
        .insert("priority".to_string(), serde_json::json!(priority));
    (config, data)
}

#[test]
fn truncate_visible_appends_ellipsis_and_reset() {
    assert_eq!(palette::truncate_visible("abc", 3), "abc");
    assert_eq!(palette::truncate_visible("abcdef", 4), "abc…\x1b[0m");
    // CJK: budget 4 fits one wide char (2) + ellipsis (1).
    assert_eq!(palette::truncate_visible("你好世", 4), "你…\x1b[0m");
    // Escape sequences cost nothing and pass through.
    assert_eq!(
        palette::visible_width(&palette::truncate_visible("\x1b[31mabcdef\x1b[0m", 4)),
        4
    );
}

#[test]
fn width_phase_truncates_single_overflowing_fragment_with_ellipsis() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![segment(
            SegmentId::Model,
            true,
            "a very long fragment",
            None,
        )],
    )
    .with_max_width(Some(10));
    standard_pipeline().run(&mut state);
    assert_eq!(state.fragments.len(), 1);
    assert!(state.line.contains('…'));
    assert!(palette::visible_width(&state.line) <= 10);
}

#[test]
fn width_phase_readds_final_fragment_truncated_when_room_allows() {
    // "* aaaaaaaaaa" (12) + "|" + "* bbbbbbbbbb" (12) = 25 cols; budget 20
    // drops the second fragment leaving 8 spare columns: enough to re-add
    // it ellipsis-truncated.
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![
            segment(SegmentId::Model, true, "aaaaaaaaaa", None),
            segment(SegmentId::Git, true, "bbbbbbbbbb", None),
        ],
    )
    .with_max_width(Some(20));
    standard_pipeline().run(&mut state);
    assert_eq!(state.fragments.len(), 2);
    assert!(state.line.contains('…'));
    assert!(palette::visible_width(&state.line) <= 20);
}

#[test]
fn width_phase_drops_lowest_priority_first() {
    let mut state = RenderState::new(
        config_with_separator("|"),
        vec![
            segment_with_priority(SegmentId::Model, "aaaa", 0),
            segment_with_priority(SegmentId::Git, "bbbb", -5),
            segment_with_priority(SegmentId::Usage, "cccc", 10),
        ],
    )
    .with_max_width(Some(14));
    standard_pipeline().run(&mut state);
    // The middle fragment has the lowest priority and goes first.
    assert!(state.line.contains("aaaa"));
    assert!(!state.line.contains("bbbb"));
    assert!(state.line.contains("cccc"));
}

#[test]
fn separators_are_rebuilt_after_a_middle_fragment_is_removed() {
    let red = AnsiColor::Color256 { c256: 1 };
    let blue = AnsiColor::Color256 { c256: 4 };
    let (mut low, low_data) = segment(SegmentId::Git, true, "bbbbbbbb", None);
    low.options
        .insert("priority".to_string(), serde_json::json!(-1));

    let mut state = RenderState::new(
        config_with_separator("\u{e0b0}"),
        vec![
            segment(SegmentId::Model, true, "aaaa", Some(red)),
            (low, low_data),
            segment(SegmentId::Usage, true, "cc", Some(blue)),
        ],
    )
    .with_max_width(Some(15));
    standard_pipeline().run(&mut state);
    // The middle (no-background) fragment was removed; the remaining arrow
    // must carry the red-to-blue transition of the surviving neighbours.
    assert_eq!(state.fragments.len(), 2);
    assert_eq!(state.separators.len(), 1);
    assert!(state.separators[0].contains("\x1b[48;5;4m"));
    assert!(state.separators[0].contains("\x1b[38;5;1m"));
}
