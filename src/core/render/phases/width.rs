use crate::config::{WidthConfig, WidthMode};
use crate::core::render::palette::{truncate_visible, visible_width};
use crate::core::render::phase::RenderPhase;
use crate::core::render::phases::separator::{build_separators, render_separator};
use crate::core::render::state::{Fragment, RenderState};

/// Smallest useful number of visible columns for an ellipsis-truncated
/// fragment; anything narrower is dropped instead.
const MIN_TRUNCATED_WIDTH: usize = 4;

/// Keeps the frame within the terminal width.
///
/// When the state knows how wide the terminal is, the lowest-priority
/// fragments are dropped until the line fits (ties drop from the end). The
/// last casualty gets a second chance: if it was the final fragment and
/// enough columns remain, it comes back ellipsis-truncated instead of
/// disappearing.
pub struct WidthPhase;

impl RenderPhase for WidthPhase {
    fn apply(&self, state: &mut RenderState) {
        if let Some(max_width) = state.max_width {
            truncate_to_width(state, max_width);
        }
        expand_flex(state);
    }
}

/// Give every flex fragment its share of the slack between the frame and the
/// width budget. Without a budget (or without slack) a flex gap degrades to a
/// single space so neighbouring segments never touch.
pub fn expand_flex(state: &mut RenderState) {
    let flex_count = state.fragments.iter().filter(|f| f.flex).count();
    if flex_count == 0 {
        return;
    }
    let slack = state
        .max_width
        .map_or(0, |w| w.saturating_sub(frame_width(state)));
    let base = slack / flex_count;
    let mut remainder = slack % flex_count;
    for fragment in state.fragments.iter_mut().filter(|f| f.flex) {
        let mut gap = base;
        if remainder > 0 {
            remainder -= 1;
            gap += 1;
        }
        fragment.body = " ".repeat(gap.max(1));
    }
}

/// Remove fragments until what remains fits within `max_width`. At least one
/// fragment always survives so the statusline never disappears entirely.
pub fn truncate_to_width(state: &mut RenderState, max_width: usize) {
    let mut last_removed_was_final = false;
    let mut last_removed: Option<Fragment> = None;

    while state.fragments.len() > 1 && frame_width(state) > max_width {
        let victim = pick_victim(state);
        last_removed_was_final = victim == state.fragments.len() - 1;
        last_removed = Some(state.fragments.remove(victim));
        build_separators(state);
    }

    let used = frame_width(state);

    if used > max_width {
        // A single fragment that still overflows: truncate it in place.
        if let Some(last) = state.fragments.last_mut() {
            let budget = max_width.saturating_sub(used - visible_width(&last.body));
            last.body = truncate_visible(&last.body, budget.max(1));
        }
    } else if last_removed_was_final {
        // The final fragment was dropped but there is spare room: bring it
        // back truncated rather than losing it entirely.
        if let (Some(frag), Some(prev)) = (last_removed, state.fragments.last()) {
            let separator = render_separator(
                &state.config.style.separator,
                prev.background.as_ref(),
                frag.background.as_ref(),
            );
            let spare = max_width - used;
            let budget = spare.saturating_sub(visible_width(&separator));
            if budget >= MIN_TRUNCATED_WIDTH {
                state.separators.push(separator);
                state.fragments.push(Fragment {
                    body: truncate_visible(&frag.body, budget),
                    ..frag
                });
            }
        }
    }
}

/// Index of the fragment to drop next: lowest priority first, and among
/// equals the one closest to the end of the line. A flex gap is never
/// sacrificed while real content remains.
fn pick_victim(state: &RenderState) -> usize {
    let mut victim = state.fragments.len() - 1;
    if state.fragments[victim].flex {
        if let Some(i) = state.fragments.iter().rposition(|f| !f.flex) {
            victim = i;
        }
    }
    for (i, fragment) in state.fragments.iter().enumerate() {
        if !fragment.flex && fragment.priority < state.fragments[victim].priority {
            victim = i;
        }
    }
    victim
}

fn frame_width(state: &RenderState) -> usize {
    let fragments: usize = state.fragments.iter().map(|f| visible_width(&f.body)).sum();
    let separators: usize = state.separators.iter().map(|s| visible_width(s)).sum();
    fragments + separators
}

/// The width budget for this render, from the environment Claude Code sets up:
/// `CCLINE_WIDTH` is an exact override; otherwise `COLUMNS` interpreted
/// through the configured width mode. `context_pct` (current context usage
/// in percent) drives the `adaptive` mode.
pub fn terminal_width(config: &WidthConfig, context_pct: Option<f64>) -> Option<usize> {
    resolve_width(
        env_usize("CCLINE_WIDTH"),
        env_usize("COLUMNS"),
        config,
        context_pct,
    )
}

/// The terminal height from the `LINES` env var, when present.
pub fn terminal_lines() -> Option<usize> {
    env_usize("LINES")
}

fn resolve_width(
    ccline_width: Option<usize>,
    columns: Option<usize>,
    config: &WidthConfig,
    context_pct: Option<f64>,
) -> Option<usize> {
    if ccline_width.is_some() {
        return ccline_width;
    }
    let columns = columns?;
    let full = match config.mode {
        WidthMode::Full => true,
        WidthMode::Reserve => false,
        // Unknown usage could compact at any moment: keep the room reserved.
        WidthMode::Adaptive => context_pct.is_some_and(|pct| pct < config.adaptive_threshold),
    };
    if full {
        Some(columns)
    } else {
        // Floored so a small terminal still gets a usable line.
        Some(columns.saturating_sub(config.reserve).max(columns.min(20)))
    }
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()?
        .trim()
        .parse::<usize>()
        .ok()
        .filter(|v| *v > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ccline_width_overrides_columns_and_reserve() {
        let cfg = WidthConfig::default();
        assert_eq!(resolve_width(Some(55), Some(200), &cfg, None), Some(55));
    }

    #[test]
    fn columns_minus_reserve_with_floor() {
        let cfg = WidthConfig::default();
        assert_eq!(resolve_width(None, Some(120), &cfg, None), Some(80));
        // Reserve would leave nothing: floor at 20 columns.
        assert_eq!(resolve_width(None, Some(45), &cfg, None), Some(20));
        // Terminal narrower than the floor: use what is there.
        assert_eq!(resolve_width(None, Some(15), &cfg, None), Some(15));
        assert_eq!(resolve_width(None, None, &cfg, None), None);
    }

    #[test]
    fn full_mode_uses_all_columns() {
        let cfg = WidthConfig {
            mode: WidthMode::Full,
            ..Default::default()
        };
        assert_eq!(resolve_width(None, Some(120), &cfg, None), Some(120));
        // The exact override still wins.
        assert_eq!(resolve_width(Some(55), Some(120), &cfg, None), Some(55));
    }

    #[test]
    fn adaptive_mode_switches_on_the_context_threshold() {
        let cfg = WidthConfig {
            mode: WidthMode::Adaptive,
            ..Default::default()
        };
        // Far from auto-compact: full width.
        assert_eq!(resolve_width(None, Some(120), &cfg, Some(30.0)), Some(120));
        // Past the threshold: reserve kicks in.
        assert_eq!(resolve_width(None, Some(120), &cfg, Some(75.0)), Some(80));
        // Unknown usage: stay safe, reserve.
        assert_eq!(resolve_width(None, Some(120), &cfg, None), Some(80));
    }
}
