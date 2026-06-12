//! ANSI color and style primitives shared by all rendering phases.

use crate::config::AnsiColor;

/// Escape code that paints subsequent text in the given foreground color.
pub fn foreground(color: &AnsiColor) -> String {
    match color {
        AnsiColor::Color16 { c16 } => {
            let code = if *c16 < 8 { 30 + c16 } else { 90 + (c16 - 8) };
            format!("\x1b[{}m", code)
        }
        AnsiColor::Color256 { c256 } => format!("\x1b[38;5;{}m", c256),
        AnsiColor::Rgb { r, g, b } => format!("\x1b[38;2;{};{};{}m", r, g, b),
    }
}

/// Escape code that paints subsequent text on the given background color.
pub fn background(color: &AnsiColor) -> String {
    match color {
        AnsiColor::Color16 { c16 } => {
            let code = if *c16 < 8 { 40 + c16 } else { 100 + (c16 - 8) };
            format!("\x1b[{}m", code)
        }
        AnsiColor::Color256 { c256 } => format!("\x1b[48;5;{}m", c256),
        AnsiColor::Rgb { r, g, b } => format!("\x1b[48;2;{};{};{}m", r, g, b),
    }
}

/// Wrap text in a foreground color, resetting afterwards.
pub fn tinted(text: &str, color: Option<&AnsiColor>) -> String {
    match color {
        Some(color) => format!("{}{}\x1b[0m", foreground(color), text),
        None => text.to_string(),
    }
}

/// Wrap text in an optional color and bold style, resetting afterwards.
pub fn styled(text: &str, color: Option<&AnsiColor>, bold: bool) -> String {
    let mut codes: Vec<String> = Vec::new();

    if bold {
        codes.push("1".to_string());
    }

    match color {
        Some(AnsiColor::Color16 { c16 }) => {
            let code = if *c16 < 8 { 30 + c16 } else { 90 + (c16 - 8) };
            codes.push(code.to_string());
        }
        Some(AnsiColor::Color256 { c256 }) => {
            codes.push("38".to_string());
            codes.push("5".to_string());
            codes.push(c256.to_string());
        }
        Some(AnsiColor::Rgb { r, g, b }) => {
            codes.push("38".to_string());
            codes.push("2".to_string());
            codes.push(r.to_string());
            codes.push(g.to_string());
            codes.push(b.to_string());
        }
        None => {}
    }

    if codes.is_empty() {
        text.to_string()
    } else {
        format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text)
    }
}

/// Escape-sequence scanner state shared by [`visible_width`] and
/// [`truncate_visible`]: CSI (`ESC [ ... letter`) and OSC (`ESC ] ... BEL`
/// or `ESC ] ... ESC \`) sequences occupy no terminal cells.
#[derive(PartialEq)]
enum EscState {
    Text,
    Intro,
    Csi,
    Osc,
    OscEsc,
}

impl EscState {
    fn step(&mut self, ch: char) -> bool {
        match self {
            EscState::Text => {
                if ch == '\x1b' {
                    *self = EscState::Intro;
                    return false;
                }
                return true;
            }
            EscState::Intro => {
                *self = match ch {
                    '[' => EscState::Csi,
                    ']' => EscState::Osc,
                    // Two-character sequence like ESC \ or ESC c: done.
                    _ => EscState::Text,
                };
            }
            EscState::Csi => {
                if ch.is_alphabetic() {
                    *self = EscState::Text;
                }
            }
            EscState::Osc => match ch {
                '\x07' => *self = EscState::Text,
                '\x1b' => *self = EscState::OscEsc,
                _ => {}
            },
            EscState::OscEsc => {
                *self = if ch == '\\' {
                    EscState::Text
                } else {
                    EscState::Osc
                };
            }
        }
        false
    }
}

/// Number of terminal cells the text occupies once escape sequences vanish.
///
/// Uses Unicode display width, so CJK characters and other wide glyphs count
/// as two columns instead of one.
pub fn visible_width(text: &str) -> usize {
    use unicode_width::UnicodeWidthChar;

    let mut width = 0usize;
    let mut state = EscState::Text;
    for ch in text.chars() {
        if state.step(ch) {
            width += ch.width().unwrap_or(0);
        }
    }
    width
}

/// Cut text down to at most `max_width` visible columns, appending `…` and a
/// reset when anything was removed. Escape sequences pass through unchanged
/// and cost nothing.
pub fn truncate_visible(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if visible_width(text) <= max_width {
        return text.to_string();
    }

    let budget = max_width.saturating_sub(1); // room for the ellipsis
    let mut out = String::new();
    let mut width = 0usize;
    let mut state = EscState::Text;

    for ch in text.chars() {
        if state.step(ch) {
            let w = ch.width().unwrap_or(0);
            if width + w > budget {
                break;
            }
            width += w;
        }
        out.push(ch);
    }

    out.push('…');
    out.push_str("\x1b[0m");
    out
}
