use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// How many bytes of the transcript tail to scan for samples.
const TAIL_BYTES: u64 = 256 * 1024;

/// Output-token rate over a sliding window, computed from the transcript's
/// per-entry timestamps and usage — stateless across invocations.
#[derive(Default)]
pub struct TokenRateSegment {
    window_seconds: u64,
}

impl TokenRateSegment {
    pub fn new() -> Self {
        Self { window_seconds: 60 }
    }

    pub fn with_window(mut self, window_seconds: u64) -> Self {
        self.window_seconds = window_seconds.max(1);
        self
    }
}

#[derive(Deserialize)]
struct TailEntry {
    timestamp: Option<String>,
    message: Option<TailMessage>,
}

#[derive(Deserialize)]
struct TailMessage {
    usage: Option<TailUsage>,
}

#[derive(Deserialize)]
struct TailUsage {
    output_tokens: Option<u64>,
}

/// One transcript sample: when it happened and how many output tokens it added.
#[derive(Debug, PartialEq)]
struct Sample {
    at: DateTime<Utc>,
    output_tokens: u64,
}

impl Segment for TokenRateSegment {
    fn collect(&self, input: &InputData) -> Option<SegmentData> {
        let tail = read_tail(Path::new(&input.transcript_path), TAIL_BYTES)?;
        let samples = parse_samples(&tail);
        let rate = window_rate(&samples, Utc::now(), self.window_seconds)?;

        let mut metadata = HashMap::new();
        metadata.insert("tokens_per_second".to_string(), format!("{:.2}", rate));

        Some(SegmentData {
            primary: format_rate(rate),
            secondary: String::new(),
            metadata,
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::TokenRate
    }
}

/// Read at most `max_bytes` from the end of the file, aligned to the first
/// complete line.
fn read_tail(path: &Path, max_bytes: u64) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start)).ok()?;

    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;

    if start > 0 {
        // Drop the (probably partial) first line.
        if let Some(pos) = buf.find('\n') {
            buf.drain(..=pos);
        }
    }
    Some(buf)
}

/// Collect `(timestamp, output_tokens)` samples from transcript JSONL text.
fn parse_samples(jsonl: &str) -> Vec<Sample> {
    jsonl
        .lines()
        .filter_map(|line| {
            let entry: TailEntry = serde_json::from_str(line).ok()?;
            let at = DateTime::parse_from_rfc3339(entry.timestamp.as_deref()?)
                .ok()?
                .with_timezone(&Utc);
            let output_tokens = entry.message?.usage?.output_tokens?;
            Some(Sample { at, output_tokens })
        })
        .collect()
}

/// Tokens per second across the samples that fall inside the window ending at
/// `now`. `None` when the window has no samples (segment hides while idle).
fn window_rate(samples: &[Sample], now: DateTime<Utc>, window_seconds: u64) -> Option<f64> {
    let cutoff = now - chrono::Duration::seconds(window_seconds as i64);
    let recent: Vec<&Sample> = samples.iter().filter(|s| s.at >= cutoff).collect();

    let first = recent.first()?;
    let tokens: u64 = recent.iter().map(|s| s.output_tokens).sum();
    let elapsed = (now - first.at).num_seconds().max(1) as f64;
    Some(tokens as f64 / elapsed)
}

/// `9.5 tok/s` below 10, `42 tok/s` above.
fn format_rate(rate: f64) -> String {
    if rate < 10.0 {
        format!("{:.1} tok/s", rate)
    } else {
        format!("{:.0} tok/s", rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(secs_ago: i64, now: DateTime<Utc>) -> DateTime<Utc> {
        now - chrono::Duration::seconds(secs_ago)
    }

    #[test]
    fn parses_samples_from_transcript_jsonl() {
        let jsonl = concat!(
            r#"{"type":"assistant","timestamp":"2026-06-12T08:00:00Z","message":{"usage":{"input_tokens":10,"output_tokens":120}}}"#,
            "\n",
            r#"{"type":"user","timestamp":"2026-06-12T08:00:05Z","message":{}}"#,
            "\n",
            "not json\n",
            r#"{"type":"assistant","timestamp":"2026-06-12T08:00:10Z","message":{"usage":{"output_tokens":80}}}"#,
        );
        let samples = parse_samples(jsonl);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].output_tokens, 120);
        assert_eq!(samples[1].output_tokens, 80);
    }

    #[test]
    fn rate_uses_only_samples_inside_the_window() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 8, 1, 0).unwrap();
        let samples = vec![
            Sample {
                at: at(300, now), // outside a 60s window
                output_tokens: 9999,
            },
            Sample {
                at: at(50, now),
                output_tokens: 600,
            },
            Sample {
                at: at(10, now),
                output_tokens: 400,
            },
        ];
        let rate = window_rate(&samples, now, 60).unwrap();
        assert!((rate - 1000.0 / 50.0).abs() < 1e-9);
    }

    #[test]
    fn empty_window_hides_the_segment() {
        let now = Utc.with_ymd_and_hms(2026, 6, 12, 8, 0, 0).unwrap();
        assert_eq!(window_rate(&[], now, 60), None);
        let stale = vec![Sample {
            at: at(120, now),
            output_tokens: 100,
        }];
        assert_eq!(window_rate(&stale, now, 60), None);
    }

    #[test]
    fn rate_formatting() {
        assert_eq!(format_rate(9.54), "9.5 tok/s");
        assert_eq!(format_rate(42.4), "42 tok/s");
    }
}
