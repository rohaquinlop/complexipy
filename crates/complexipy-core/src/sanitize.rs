use std::borrow::Cow;

/// Replaces every control character except tab, and every bidirectional
/// formatting character, with a visible `\u{..}` escape. Text that comes
/// from a repository (a path, a source line, a config value) goes through
/// this before it reaches a terminal or an editor log, so it cannot move the
/// cursor, rewrite earlier lines, set the window title, write the clipboard,
/// or plant a hyperlink.
pub fn escape_controls(text: &str) -> Cow<'_, str> {
    if !text.chars().any(needs_escape) {
        return Cow::Borrowed(text);
    }
    let mut escaped = String::with_capacity(text.len() + 8);
    for character in text.chars() {
        if needs_escape(character) {
            escaped.extend(character.escape_unicode());
        } else {
            escaped.push(character);
        }
    }
    Cow::Owned(escaped)
}

fn needs_escape(character: char) -> bool {
    (character.is_control() && character != '\t')
        || matches!(
            character,
            '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
        )
}

/// Prefixes a CSV cell that a spreadsheet would run as a formula (one that
/// starts with `=`, `+`, `-`, `@`, a tab, or a carriage return) with `'`, so
/// the cell stays text.
pub fn neutralize_csv_cell(cell: &str) -> Cow<'_, str> {
    if cell.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        Cow::Owned(format!("'{}", cell))
    } else {
        Cow::Borrowed(cell)
    }
}

#[cfg(test)]
mod tests;
