//! Golden snapshots of the main render path: every built-in theme at a
//! matrix of width budgets, against fixed mock segment data.
//!
//! On mismatch, inspect the diff and refresh with:
//! `UPDATE_SNAPSHOTS=1 cargo test --test render_snapshots`

use ccometixline::config::{Config, SegmentConfig, SegmentId};
use ccometixline::core::segments::SegmentData;
use ccometixline::core::StatusLineGenerator;
use ccometixline::ui::themes::ThemePresets;
use std::collections::HashMap;
use std::path::PathBuf;

const WIDTHS: [Option<usize>; 4] = [None, Some(80), Some(40), Some(20)];

fn themes() -> Vec<(&'static str, Config)> {
    vec![
        ("default", ThemePresets::get_default()),
        ("cometix", ThemePresets::get_cometix()),
        ("minimal", ThemePresets::get_minimal()),
        ("gruvbox", ThemePresets::get_gruvbox()),
        ("nord", ThemePresets::get_nord()),
        ("powerline-dark", ThemePresets::get_powerline_dark()),
        ("powerline-light", ThemePresets::get_powerline_light()),
        (
            "powerline-rose-pine",
            ThemePresets::get_powerline_rose_pine(),
        ),
        (
            "powerline-tokyo-night",
            ThemePresets::get_powerline_tokyo_night(),
        ),
    ]
}

/// Deterministic stand-in data per segment kind.
fn mock_data(id: SegmentId) -> SegmentData {
    let (primary, secondary) = match id {
        SegmentId::Model => ("Sonnet 4.6", ""),
        SegmentId::Directory => ("CCometixLine", ""),
        SegmentId::Git => ("master", "✓"),
        SegmentId::ContextWindow => ("78.2% · 156.4k tokens", ""),
        SegmentId::Usage => ("24%", "· 10-7-2"),
        SegmentId::Cost => ("$0.02", ""),
        SegmentId::Session => ("3m45s", "+156 -23"),
        SegmentId::OutputStyle => ("default", ""),
        SegmentId::Update => ("v1.0.0", ""),
        SegmentId::TokenRate => ("42 tok/s", ""),
        SegmentId::WeeklyUsage => ("W 42%", "· O 13%"),
        SegmentId::Flex | SegmentId::Custom => ("", ""),
    };
    SegmentData {
        primary: primary.to_string(),
        secondary: secondary.to_string(),
        metadata: HashMap::new(),
    }
}

fn mock_segments(config: &Config) -> Vec<(SegmentConfig, SegmentData)> {
    config
        .segments
        .iter()
        .filter(|s| s.enabled)
        .map(|s| (s.clone(), mock_data(s.id)))
        .collect()
}

fn snapshot_path(theme: &str, width: Option<usize>) -> PathBuf {
    let width = width.map_or("none".to_string(), |w| w.to_string());
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
        .join(format!("{}_w{}.snap", theme, width))
}

#[test]
fn themes_render_identically_across_the_width_matrix() {
    let update = std::env::var("UPDATE_SNAPSHOTS").is_ok();
    let mut failures = Vec::new();

    for (name, config) in themes() {
        for width in WIDTHS {
            let generator = StatusLineGenerator::new(config.clone());
            let line = generator.generate_with_width(mock_segments(&config), width);
            // Debug-escaped so the ANSI bytes survive diffs and editors.
            let rendered = format!("{:?}\n", line);

            let path = snapshot_path(name, width);
            if update {
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(&path, &rendered).unwrap();
                continue;
            }

            match std::fs::read_to_string(&path) {
                Ok(expected) if expected == rendered => {}
                Ok(expected) => failures.push(format!(
                    "{}:\n  expected: {}  actual:   {}",
                    path.display(),
                    expected,
                    rendered
                )),
                Err(_) => failures.push(format!("{}: snapshot missing", path.display())),
            }
        }
    }

    assert!(
        failures.is_empty(),
        "snapshot mismatches (refresh with UPDATE_SNAPSHOTS=1):\n{}",
        failures.join("\n")
    );
}
