//! Table of contents generation from heading data.
//!
//! Produces a nested `<ul>` structure and generates URL-friendly heading IDs
//! via `slugify` (preserves CJK characters).

use regex::Regex;
use std::fmt::Write;
use std::sync::LazyLock;

use crate::utils::html_escape;

use super::math;

#[derive(Debug, Clone)]
pub struct TocItem {
    pub level: u8,
    pub id: String,
    pub text: String,
}

static INLINE_MATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(^|[^\\$])\$([^$\n]+?)\$").unwrap());

/// Generate a nested HTML `<nav>` TOC from heading items.
pub fn generate_toc(items: &[TocItem], render_math: bool) -> String {
    if items.is_empty() {
        return String::new();
    }

    let mut html = String::from("<nav class=\"toc\">\n<h3>目录</h3>\n<ul>\n");
    let mut current_level = 0u8;

    for item in items {
        while current_level < item.level {
            html.push_str("<ul>\n");
            current_level += 1;
        }
        while current_level > item.level {
            html.push_str("</ul>\n");
            current_level -= 1;
        }
        let text = render_toc_text(&item.text, render_math);
        let _ = write!(html, "<li><a href=\"#{}\">{}</a></li>\n", item.id, text);
    }

    while current_level > 0 {
        html.push_str("</ul>\n");
        current_level -= 1;
    }

    html.push_str("</ul>\n</nav>");
    html
}

fn render_toc_text(text: &str, render_math: bool) -> String {
    if !render_math {
        return html_escape(text);
    }

    let mut html = String::new();
    let mut last = 0;

    for caps in INLINE_MATH_RE.captures_iter(text) {
        let whole = caps.get(0).expect("whole match exists");
        let prefix = caps.get(1).expect("prefix capture exists");
        let latex = caps.get(2).expect("latex capture exists");

        html.push_str(&html_escape(&text[last..prefix.start()]));
        html.push_str(&html_escape(prefix.as_str()));
        html.push_str(&math::render_inline(latex.as_str()));
        last = whole.end();
    }

    html.push_str(&html_escape(&text[last..]));
    html
}

/// Convert heading text to a URL-friendly anchor ID.
/// CJK characters are preserved as-is; other non-alphanumeric chars become `-`.
pub fn slugify(text: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for ch in text.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' => {
                slug.extend(ch.to_lowercase());
                prev_dash = false;
            }
            _ if ch as u32 > 0x4E00 => {
                // Keep CJK characters as-is
                slug.push(ch);
                prev_dash = false;
            }
            _ => {
                if !prev_dash && !slug.is_empty() {
                    slug.push('-');
                    prev_dash = true;
                }
            }
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}
