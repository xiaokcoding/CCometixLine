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

/// Number of terminal cells the text occupies once escape sequences vanish.
///
/// Uses Unicode display width, so CJK characters and other wide glyphs count
/// as two columns instead of one.
pub fn visible_width(text: &str) -> usize {
    use unicode_width::UnicodeWidthChar;

    let mut width = 0usize;
    let mut in_escape = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            in_escape = true;
            if chars.peek() == Some(&'[') {
                chars.next();
            }
        } else if in_escape {
            if ch.is_alphabetic() {
                in_escape = false;
            }
        } else {
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
    let mut in_escape = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            in_escape = true;
            out.push(ch);
            if chars.peek() == Some(&'[') {
                out.push(chars.next().unwrap());
            }
        } else if in_escape {
            out.push(ch);
            if ch.is_alphabetic() {
                in_escape = false;
            }
        } else {
            let w = ch.width().unwrap_or(0);
            if width + w > budget {
                break;
            }
            width += w;
            out.push(ch);
        }
    }

    out.push('…');
    out.push_str("\x1b[0m");
    out
}
