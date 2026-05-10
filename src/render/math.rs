//! KaTeX math rendering (inline and display mode).
//!
//! Requires `katex.min.css` and font files to be served for proper browser rendering.
//! On error, falls back to a `<code class="math-error">` display.

use crate::utils::html_escape;

pub fn render_inline(latex: &str) -> String {
    match katex::render(latex) {
        Ok(html) => html,
        Err(e) => {
            log::warn!("KaTeX inline render error: {e}");
            format!("<code class=\"math-error\">{}</code>", html_escape(latex))
        }
    }
}

pub fn render_display(latex: &str) -> String {
    let opts = katex::Opts::builder()
        .display_mode(true)
        .build()
        .expect("KaTeX opts are valid");

    match katex::render_with_opts(latex, &opts) {
        Ok(html) => html,
        Err(e) => {
            log::warn!("KaTeX display render error: {e}");
            format!("<code class=\"math-error\">{}</code>", html_escape(latex))
        }
    }
}
