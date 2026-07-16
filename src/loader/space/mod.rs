//! Fixed-width / space-aligned format parser.
//!
//! Determines column boundaries from the header line:
//! tokens separated by 2+ consecutive spaces define column start positions.

mod detect;
mod parse;

#[cfg(test)]
mod tests;

pub use detect::looks_like_space_format;
pub use parse::load_space;

/// Column boundary definition.
#[derive(Debug, Clone)]
pub(crate) struct Column {
    pub(crate) start: usize,
    pub(crate) name: String,
}

/// Detect whether a line is a separator (e.g., "---", "===", "─────").
pub(crate) fn is_separator_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .chars()
        .all(|c| matches!(c, '-' | '=' | '─' | '┼' | '+' | '|' | ' '))
}
