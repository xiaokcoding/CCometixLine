use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use crate::utils::credentials;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Weekly usage split by model, matching Claude Code's `/usage` view.
///
/// With an OAuth login the official usage API provides exact utilization
/// percentages (all models + Opus). Without one, the segment falls back to
/// aggregating token totals per model family from the local transcripts of
/// the last seven days.
#[derive(Default)]
pub struct WeeklyUsageSegment;

impl WeeklyUsageSegment {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    seven_day: Option<ApiPeriod>,
    seven_day_opus: Option<ApiPeriod>,
}

#[derive(Debug, Deserialize)]
struct ApiPeriod {
    utilization: f64,
}

/// What ends up on the statusline, cached between refresh ticks.
#[derive(Debug, Serialize, Deserialize)]
struct WeeklyCache {
    primary: String,
    secondary: String,
    source: String,
    cached_at: String,
}

impl Segment for WeeklyUsageSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        let config = crate::config::Config::load().ok()?;
        let options = config
            .segments
            .iter()
            .find(|s| s.id == SegmentId::WeeklyUsage)
            .map(|s| &s.options);

        let api_base_url = options
            .and_then(|o| o.get("api_base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("https://api.anthropic.com")
            .to_string();
        let cache_duration = options
            .and_then(|o| o.get("cache_duration"))
            .and_then(|v| v.as_u64())
            .unwrap_or(600);
        let timeout = options
            .and_then(|o| o.get("timeout"))
            .and_then(|v| v.as_u64())
            .unwrap_or(2);

        let cached = load_cache();
        if let Some(cache) = &cached {
            if cache_age_seconds(cache).is_some_and(|age| age < cache_duration as i64) {
                return Some(to_segment_data(cache));
            }
        }

        let fresh = collect_from_api(&api_base_url, timeout)
            .or_else(collect_from_transcripts)
            .map(|(primary, secondary, source)| WeeklyCache {
                primary,
                secondary,
                source,
                cached_at: Utc::now().to_rfc3339(),
            });

        match fresh {
            Some(cache) => {
                save_cache(&cache);
                Some(to_segment_data(&cache))
            }
            // Both sources failed: serve a stale cache rather than nothing.
            None => cached.as_ref().map(to_segment_data),
        }
    }

    fn id(&self) -> SegmentId {
        SegmentId::WeeklyUsage
    }
}

fn to_segment_data(cache: &WeeklyCache) -> SegmentData {
    let mut metadata = HashMap::new();
    metadata.insert("source".to_string(), cache.source.clone());
    SegmentData {
        primary: cache.primary.clone(),
        secondary: cache.secondary.clone(),
        metadata,
    }
}

/// Exact weekly utilization from the official usage API. `None` without an
/// OAuth login or when the request fails.
fn collect_from_api(api_base_url: &str, timeout_secs: u64) -> Option<(String, String, String)> {
    let token = credentials::get_oauth_token()?;
    let url = format!("{}/api/oauth/usage", api_base_url);

    let response: ApiResponse = ureq::Agent::new_with_defaults()
        .get(&url)
        .header("Authorization", &format!("Bearer {}", token))
        .header("anthropic-beta", "oauth-2025-04-20")
        .config()
        .timeout_global(Some(std::time::Duration::from_secs(timeout_secs)))
        .build()
        .call()
        .ok()?
        .into_body()
        .read_json()
        .ok()?;

    let seven_day = response.seven_day?;
    let primary = format!("W {}%", seven_day.utilization.round() as u8);
    let secondary = match response.seven_day_opus {
        Some(opus) => format!("· O {}%", opus.utilization.round() as u8),
        None => String::new(),
    };
    Some((primary, secondary, "api".to_string()))
}

/// Token totals per model family from the last week of local transcripts.
fn collect_from_transcripts() -> Option<(String, String, String)> {
    let projects = dirs::home_dir()?.join(".claude").join("projects");
    let totals = aggregate_transcripts(&projects, Utc::now());

    if totals.is_empty() {
        return None;
    }

    let mut parts: Vec<String> = Vec::new();
    for (label, key) in [("S", "sonnet"), ("O", "opus"), ("H", "haiku")] {
        if let Some(tokens) = totals.get(key) {
            parts.push(format!("{} {}", label, format_tokens(*tokens)));
        }
    }
    if parts.is_empty() {
        // Tokens exist but none in a known family.
        let total: u64 = totals.values().sum();
        parts.push(format_tokens(total));
    }

    let primary = parts.remove(0);
    let secondary = if parts.is_empty() {
        String::new()
    } else {
        format!("· {}", parts.join(" · "))
    };
    Some((primary, secondary, "transcript".to_string()))
}

#[derive(Deserialize)]
struct WeeklyEntry {
    message: Option<WeeklyMessage>,
}

#[derive(Deserialize)]
struct WeeklyMessage {
    model: Option<String>,
    usage: Option<WeeklyTokens>,
}

#[derive(Deserialize)]
struct WeeklyTokens {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
}

/// Sum input+output tokens per model family across every transcript modified
/// within the last seven days.
fn aggregate_transcripts(projects_dir: &Path, now: chrono::DateTime<Utc>) -> HashMap<String, u64> {
    let mut totals = HashMap::new();
    for path in recent_transcripts(projects_dir, now) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                // Cheap prefilter: skip lines without usage data before serde.
                if !line.contains("\"usage\"") {
                    continue;
                }
                let Ok(entry) = serde_json::from_str::<WeeklyEntry>(line) else {
                    continue;
                };
                let Some(message) = entry.message else {
                    continue;
                };
                let (Some(model), Some(usage)) = (message.model, message.usage) else {
                    continue;
                };
                let tokens = usage.input_tokens.unwrap_or(0) + usage.output_tokens.unwrap_or(0);
                if tokens > 0 {
                    *totals.entry(model_family(&model)).or_insert(0) += tokens;
                }
            }
        }
    }
    totals
}

/// All `.jsonl` transcripts under the projects dir modified in the last week.
fn recent_transcripts(projects_dir: &Path, now: chrono::DateTime<Utc>) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(projects) = std::fs::read_dir(projects_dir) else {
        return found;
    };
    for project in projects.flatten() {
        let Ok(sessions) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for session in sessions.flatten() {
            let path = session.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let Ok(modified) = session.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            let modified: chrono::DateTime<Utc> = modified.into();
            if (now - modified).num_days() < 7 {
                found.push(path);
            }
        }
    }
    found
}

/// Bucket a model id into a family: `claude-opus-4-8` → `opus`.
fn model_family(model: &str) -> String {
    let lower = model.to_lowercase();
    for family in ["sonnet", "opus", "haiku", "fable"] {
        if lower.contains(family) {
            return family.to_string();
        }
    }
    "other".to_string()
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

fn cache_path() -> Option<PathBuf> {
    Some(
        dirs::home_dir()?
            .join(".claude")
            .join("ccline")
            .join(".weekly_usage_cache.json"),
    )
}

fn load_cache() -> Option<WeeklyCache> {
    let content = std::fs::read_to_string(cache_path()?).ok()?;
    serde_json::from_str(&content).ok()
}

fn save_cache(cache: &WeeklyCache) {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(cache) {
            let _ = std::fs::write(&path, json);
        }
    }
}

fn cache_age_seconds(cache: &WeeklyCache) -> Option<i64> {
    let cached_at = chrono::DateTime::parse_from_rfc3339(&cache.cached_at).ok()?;
    Some((Utc::now() - cached_at.with_timezone(&Utc)).num_seconds())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_api_response_with_and_without_opus_period() {
        let with: ApiResponse = serde_json::from_str(
            r#"{"five_hour":{"utilization":10.0},"seven_day":{"utilization":42.4},"seven_day_opus":{"utilization":13.0,"resets_at":null}}"#,
        )
        .unwrap();
        assert_eq!(with.seven_day.unwrap().utilization, 42.4);
        assert_eq!(with.seven_day_opus.unwrap().utilization, 13.0);

        let without: ApiResponse =
            serde_json::from_str(r#"{"seven_day":{"utilization":5.0}}"#).unwrap();
        assert!(without.seven_day_opus.is_none());
    }

    #[test]
    fn buckets_model_ids_into_families() {
        assert_eq!(model_family("claude-sonnet-4-6"), "sonnet");
        assert_eq!(model_family("claude-opus-4-8"), "opus");
        assert_eq!(model_family("claude-fable-5"), "fable");
        assert_eq!(model_family("gpt-x"), "other");
    }

    #[test]
    fn aggregates_recent_transcripts_per_family() {
        let dir = std::env::temp_dir().join(format!("ccline-weekly-test-{}", std::process::id()));
        let project = dir.join("proj");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(
            project.join("session.jsonl"),
            concat!(
                r#"{"type":"assistant","message":{"model":"claude-sonnet-4-6","usage":{"input_tokens":100,"output_tokens":50}}}"#,
                "\n",
                r#"{"type":"assistant","message":{"model":"claude-opus-4-8","usage":{"output_tokens":30}}}"#,
                "\n",
                r#"{"type":"user","message":{}}"#,
                "\n",
                "garbage\n",
            ),
        )
        .unwrap();

        let totals = aggregate_transcripts(&dir, Utc::now());
        assert_eq!(totals.get("sonnet"), Some(&150));
        assert_eq!(totals.get("opus"), Some(&30));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn token_formatting() {
        assert_eq!(format_tokens(980), "980");
        assert_eq!(format_tokens(15_300), "15.3k");
        assert_eq!(format_tokens(1_200_000), "1.2M");
    }
}
