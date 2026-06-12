use super::{Segment, SegmentData};
use crate::config::{InputData, SegmentId};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// User-defined content: a static `text` option, or the trimmed stdout of a
/// `command` (which wins when both are set). Several `custom` entries may
/// coexist in the segment list, each with its own options.
pub struct CustomSegment {
    text: Option<String>,
    command: Option<String>,
    timeout: Duration,
    cache_duration: u64,
}

impl CustomSegment {
    pub fn from_options(options: &HashMap<String, serde_json::Value>) -> Self {
        let get_str = |key: &str| {
            options
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        Self {
            text: get_str("text"),
            command: get_str("command"),
            timeout: Duration::from_secs(
                options.get("timeout").and_then(|v| v.as_u64()).unwrap_or(1),
            ),
            cache_duration: options
                .get("cache_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
        }
    }

    fn content(&self) -> Option<String> {
        if let Some(command) = &self.command {
            return self.command_output(command);
        }
        self.text.clone()
    }

    fn command_output(&self, command: &str) -> Option<String> {
        if self.cache_duration > 0 {
            if let Some(cached) = read_cache(command, self.cache_duration) {
                return non_empty(cached);
            }
        }
        let output = run_with_timeout(command, self.timeout)?;
        if self.cache_duration > 0 {
            write_cache(command, &output);
        }
        non_empty(output)
    }
}

impl Segment for CustomSegment {
    fn collect(&self, _input: &InputData) -> Option<SegmentData> {
        Some(SegmentData {
            primary: self.content()?,
            secondary: String::new(),
            metadata: HashMap::new(),
        })
    }

    fn id(&self) -> SegmentId {
        SegmentId::Custom
    }
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// First line of the command's stdout, or `None` on failure or timeout. The
/// statusline must never block on a slow command, so the child is polled and
/// killed once the timeout passes.
fn run_with_timeout(command: &str, timeout: Duration) -> Option<String> {
    let mut child = shell_command(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return None,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => return None,
        }
    }

    let mut stdout = String::new();
    use std::io::Read;
    child.stdout.take()?.read_to_string(&mut stdout).ok()?;
    Some(stdout.lines().next().unwrap_or("").trim().to_string())
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

/// One cache file per distinct command under `~/.claude/ccline/`.
fn cache_path(command: &str) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    command.hash(&mut hasher);
    Some(
        dirs::home_dir()?
            .join(".claude")
            .join("ccline")
            .join(format!(".custom_cache_{:016x}.txt", hasher.finish())),
    )
}

fn read_cache(command: &str, max_age_secs: u64) -> Option<String> {
    let path = cache_path(command)?;
    let age = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()?
        .elapsed()
        .ok()?;
    if age.as_secs() >= max_age_secs {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn write_cache(command: &str, output: &str) {
    if let Some(path) = cache_path(command) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input() -> InputData {
        serde_json::from_str(
            r#"{"model":{"id":"claude-sonnet-4-6","display_name":"Sonnet"},
                "workspace":{"current_dir":"/tmp"},
                "transcript_path":"/tmp/nope.jsonl"}"#,
        )
        .unwrap()
    }

    #[test]
    fn static_text_becomes_the_primary() {
        let options = HashMap::from([("text".to_string(), json!("hello"))]);
        let segment = CustomSegment::from_options(&options);
        assert_eq!(segment.collect(&input()).unwrap().primary, "hello");
    }

    #[test]
    fn empty_options_hide_the_segment() {
        let segment = CustomSegment::from_options(&HashMap::new());
        assert!(segment.collect(&input()).is_none());
    }

    #[test]
    fn command_output_first_line_trimmed() {
        let command = if cfg!(windows) {
            "echo hi there"
        } else {
            "printf 'hi there\\nsecond'"
        };
        let options = HashMap::from([
            ("command".to_string(), json!(command)),
            ("timeout".to_string(), json!(5)),
        ]);
        let segment = CustomSegment::from_options(&options);
        assert_eq!(segment.collect(&input()).unwrap().primary, "hi there");
    }

    #[test]
    fn failing_command_hides_the_segment() {
        let options = HashMap::from([("command".to_string(), json!("exit 3"))]);
        let segment = CustomSegment::from_options(&options);
        assert!(segment.collect(&input()).is_none());
    }
}
