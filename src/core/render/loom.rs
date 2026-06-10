//! Folds a frame's fragments into multiple lines for constrained widths.

use super::palette::visible_width;
use super::state::RenderState;

/// Weave fragments and seams into as many lines as the width demands.
///
/// A fragment that would overflow the current line starts a fresh one; a seam
/// is only woven in when both it and the next fragment still fit, otherwise
/// the line breaks and the seam is left behind.
pub fn fold_into_lines(state: &RenderState, max_width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0usize;

    for (i, fragment) in state.fragments.iter().enumerate() {
        let fragment_width = visible_width(&fragment.body);

        if current_width > 0 && current_width + fragment_width > max_width {
            lines.push(std::mem::take(&mut current_line));
            current_width = 0;
        }

        current_line.push_str(&fragment.body);
        current_width += fragment_width;

        if let Some(seam) = state.seams.get(i) {
            let seam_width = visible_width(seam);

            if let Some(next) = state.fragments.get(i + 1) {
                let next_width = visible_width(&next.body);

                if current_width + seam_width + next_width <= max_width {
                    current_line.push_str(seam);
                    current_width += seam_width;
                } else {
                    lines.push(std::mem::take(&mut current_line));
                    current_width = 0;
                }
            }
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}
