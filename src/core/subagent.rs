//! Rendering for Claude Code's `subagentStatusLine` setting.
//!
//! Claude Code runs the configured command once per refresh tick with one
//! JSON object on stdin: the base hook fields plus `columns` (the usable row
//! width) and a `tasks` array. For every row we want to override we print one
//! JSON line of the form `{"id": "<task id>", "content": "<row body>"}`;
//! `content` is rendered as-is, ANSI escapes included.

use crate::core::render::palette::{truncate_visible, visible_width};
use serde::Deserialize;

/// The payload Claude Code pipes to a `subagentStatusLine` command.
#[derive(Debug, Deserialize)]
pub struct SubagentInput {
    /// Usable row width in terminal columns.
    #[serde(default)]
    pub columns: Option<usize>,
    #[serde(default)]
    pub tasks: Vec<SubagentTask>,
}

/// One visible subagent row.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SubagentTask {
    pub id: String,
    pub name: Option<String>,
    pub label: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub token_count: Option<u64>,
}

/// Render every task into a `{"id","content"}` JSON line, joined by newlines.
pub fn render_rows(input: &SubagentInput) -> String {
    input
        .tasks
        .iter()
        .map(|task| {
            let row = render_row(task, input.columns);
            // serde_json string serialization guarantees valid JSONL output
            // even when names contain quotes or control characters.
            format!(
                "{{\"id\":{},\"content\":{}}}",
                serde_json::to_string(&task.id).unwrap_or_else(|_| "\"\"".to_string()),
                serde_json::to_string(&row).unwrap_or_else(|_| "\"\"".to_string()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One row: status icon + title, dim description, token count — truncated to
/// the usable width.
fn render_row(task: &SubagentTask, columns: Option<usize>) -> String {
    let (icon, color) = status_style(task.status.as_deref());

    let title = task
        .name
        .as_deref()
        .or(task.label.as_deref())
        .or(task.description.as_deref())
        .unwrap_or("subagent");

    let mut row = format!("{}{}\x1b[0m \x1b[1m{}\x1b[0m", color, icon, title);

    if let Some(description) = task.description.as_deref() {
        if task.name.is_some() || task.label.is_some() {
            row.push_str(&format!(" \x1b[2m{}\x1b[0m", description));
        }
    }

    if let Some(tokens) = task.token_count {
        let tokens = format_tokens(tokens);
        if let Some(columns) = columns {
            // Right-align the token count when the width is known.
            let suffix_width = visible_width(&tokens);
            let body_budget = columns.saturating_sub(suffix_width + 1);
            let body = truncate_visible(&row, body_budget);
            let pad = columns.saturating_sub(visible_width(&body) + suffix_width);
            return format!("{}{}\x1b[2m{}\x1b[0m", body, " ".repeat(pad), tokens);
        }
        row.push_str(&format!(" \x1b[2m{}\x1b[0m", tokens));
        return row;
    }

    match columns {
        Some(columns) => truncate_visible(&row, columns),
        None => row,
    }
}

/// Icon and ANSI color for a task status. Status values are not enumerated in
/// the docs, so unknown ones get a neutral style.
fn status_style(status: Option<&str>) -> (&'static str, &'static str) {
    match status.unwrap_or("") {
        s if s.eq_ignore_ascii_case("running") || s.eq_ignore_ascii_case("in_progress") => {
            ("●", "\x1b[36m")
        }
        s if s.eq_ignore_ascii_case("completed") || s.eq_ignore_ascii_case("done") => {
            ("✓", "\x1b[32m")
        }
        s if s.eq_ignore_ascii_case("failed") || s.eq_ignore_ascii_case("error") => {
            ("✗", "\x1b[31m")
        }
        s if s.eq_ignore_ascii_case("pending") || s.eq_ignore_ascii_case("queued") => {
            ("○", "\x1b[33m")
        }
        _ => ("◌", "\x1b[37m"),
    }
}

/// Compact token count: `980`, `12.3k`, `1.2M`.
fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(name: &str, status: &str, tokens: u64) -> SubagentTask {
        SubagentTask {
            id: "task-1".to_string(),
            name: Some(name.to_string()),
            status: Some(status.to_string()),
            token_count: Some(tokens),
            ..Default::default()
        }
    }

    #[test]
    fn parses_camel_case_payload_with_missing_fields() {
        let input: SubagentInput = serde_json::from_str(
            r#"{
                "session_id": "abc",
                "columns": 60,
                "tasks": [
                    {"id": "t1", "name": "Explore", "status": "running",
                     "tokenCount": 15300, "startTime": 1, "tokenSamples": []},
                    {"id": "t2"}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(input.columns, Some(60));
        assert_eq!(input.tasks.len(), 2);
        assert_eq!(input.tasks[0].token_count, Some(15300));
        assert_eq!(input.tasks[1].name, None);
    }

    #[test]
    fn renders_one_json_line_per_task() {
        let input = SubagentInput {
            columns: Some(40),
            tasks: vec![
                task("Explore", "running", 15300),
                task("Fix", "completed", 980),
            ],
        };
        let out = render_rows(&input);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(v["id"].is_string());
            assert!(v["content"].is_string());
        }
        assert!(lines[0].contains("15.3k"));
        assert!(lines[1].contains("980"));
    }

    #[test]
    fn rows_fit_within_columns_with_right_aligned_tokens() {
        let input = SubagentInput {
            columns: Some(30),
            tasks: vec![task(
                "a very long subagent task name that overflows",
                "running",
                15300,
            )],
        };
        let out = render_rows(&input);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let content = v["content"].as_str().unwrap();
        assert_eq!(visible_width(content), 30);
        assert!(content.contains('…'));
        assert!(content.ends_with("15.3k\x1b[0m"));
    }

    #[test]
    fn unknown_status_gets_neutral_style_and_fallback_title() {
        let input = SubagentInput {
            columns: None,
            tasks: vec![SubagentTask {
                id: "t".to_string(),
                ..Default::default()
            }],
        };
        let out = render_rows(&input);
        assert!(out.contains("subagent"));
        assert!(out.contains("◌"));
    }

    #[test]
    fn token_formatting() {
        assert_eq!(format_tokens(980), "980");
        assert_eq!(format_tokens(15_300), "15.3k");
        assert_eq!(format_tokens(1_200_000), "1.2M");
    }
}
