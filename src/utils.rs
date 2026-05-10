//! Shared utility functions.

/// Escape HTML special characters (`&`, `<`, `>`, `"`).
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
