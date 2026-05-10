//! Syntax highlighting via syntect.
//!
//! Renders code blocks with per-line highlighting, aligned line numbers,
//! a copy button, and a wrap toggle. The fence info string can include an
//! optional title: ` ```python hello.py ` renders as a "hello.py" title bar.
//!
//! The highlight theme is configurable via `config.toml`'s `theme.code_theme`
//! (falls back to `InspiredGitHub` if the name is not found).

use std::sync::LazyLock;

use crate::utils::html_escape;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::html::{IncludeBackground, append_highlighted_html_for_styled_line};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(|| {
    SyntaxSet::load_defaults_newlines()
});

static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(|| {
    ThemeSet::load_defaults()
});

const DEFAULT_THEME: &str = "InspiredGitHub";

/// Highlight code with line numbers, optional title, copy button, and wrap toggle.
///
/// `info` is the fence info string, e.g. `"python quine.py"` → language="python", title="quine.py".
/// `theme_name` is the syntect theme name (e.g. `"InspiredGitHub"`, `"base16-ocean-dark"`).
pub fn highlight(code: &str, info: &str, theme_name: &str) -> String {
    let (language, title) = parse_fence_info(info);

    let syntax = SYNTAX_SET
        .find_syntax_by_token(&language)
        .or_else(|| SYNTAX_SET.find_syntax_by_extension(&language))
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());

    let theme = THEME_SET.themes.get(theme_name)
        .unwrap_or_else(|| THEME_SET.themes.get(DEFAULT_THEME)
            .expect("default theme exists"));

    let line_count = code.lines().count();
    let width = line_count.to_string().len();

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut body_html = String::new();

    for (i, line) in LinesWithEndings::from(code).enumerate() {
        let num = i + 1;
        let regions = match highlighter.highlight_line(line, &SYNTAX_SET) {
            Ok(r) => r,
            Err(_) => {
                body_html.push_str(&format!(
                    "<span class=\"cl\"><span class=\"ln\">{num:>width$}</span>{}</span>",
                    html_escape(line)
                ));
                continue;
            }
        };
        let mut line_html = String::new();
        let _ = append_highlighted_html_for_styled_line(
            &regions,
            IncludeBackground::No,
            &mut line_html,
        );
        body_html.push_str(&format!(
            "<span class=\"cl\"><span class=\"ln\">{num:>width$}</span>{line_html}</span>"
        ));
    }

    let header_right = "<span class=\"code-actions\"><button class=\"code-wrap\" onclick=\"toggleWrap(this)\" title=\"切换自动换行\">换行</button><button class=\"code-copy\" onclick=\"copyCode(this)\" title=\"复制代码\">复制</button></span>";

    let title_bar = if let Some(t) = title {
        format!("<div class=\"code-header\"><span class=\"code-title\">{t}</span>{header_right}</div>")
    } else {
        format!("<div class=\"code-header\"><span class=\"code-lang\">{language}</span>{header_right}</div>")
    };

    format!(
        "<div class=\"code-block\" data-lang=\"{language}\">{title_bar}<pre class=\"highlight\"><code>{body_html}</code></pre></div>"
    )
}

/// Parse fence info into (language, optional_title).
/// `"python quine.py"` → `("python", Some("quine.py"))`.
fn parse_fence_info(info: &str) -> (String, Option<String>) {
    let info = info.trim();
    if info.is_empty() {
        return ("text".into(), None);
    }
    let mut parts = info.splitn(2, char::is_whitespace);
    let lang = parts.next().unwrap_or("text").to_string();
    let title = parts.next().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    (lang, title)
}

/// Generate CSS for the syntect highlight theme (written to `css/highlight.css`).
/// `theme_name` is the syntect theme name; falls back to `InspiredGitHub` if not found.
pub fn generate_highlight_css(theme_name: &str) -> String {
    let theme = THEME_SET.themes.get(theme_name)
        .unwrap_or_else(|| THEME_SET.themes.get(DEFAULT_THEME)
            .expect("default theme exists"));
    syntect::html::css_for_theme_with_class_style(theme, syntect::html::ClassStyle::Spaced)
        .unwrap_or_default()
}
