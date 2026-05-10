//! Rendering pipeline: orchestrates markdown, code highlighting, math, hexo tags, and TOC.
//!
//! The pipeline runs in three passes:
//! 1. **Extract** hexo tags (`{% note %}`, `{% video %}`, etc.) → replace with placeholders
//! 2. **Render** markdown via pulldown-cmark (placeholders pass through as literal text),
//!    intercepting math/code/heading events for specialized processing
//! 3. **Resolve** placeholders — recursively render inner content and substitute final HTML
//!
//! A final regex pass fixes `**bold**` and `~~strikethrough~~` that pulldown-cmark
//! misses when delimiters are adjacent to CJK characters.

pub mod code_highlight;
pub mod hexo_tags;
pub mod markdown;
pub mod math;
pub mod toc;

use regex::Regex;
use std::sync::LazyLock;

use crate::post::Post;

// Fallback regexes for cases pulldown-cmark misses (e.g. CJK-adjacent)
static STRIKETHROUGH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"~~([^~]+)~~").unwrap()
});
static BOLD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\*\*([^*]+)\*\*").unwrap()
});

/// Rendered output for a single post.
pub struct PostHtml {
    pub content: String,
    pub excerpt_html: String,
    pub toc_html: String,
    pub has_math: bool,
}

/// Full rendering of a post: body, excerpt, and TOC.
///
/// `code_theme` is the syntect theme name (from `config.theme.code_theme`).
/// `default_toc` is the global TOC default (from `config.theme.toc`), used when
/// the post's frontmatter doesn't explicitly set `toc`.
pub fn render_post(post: &Post, code_theme: &str, default_toc: bool) -> PostHtml {
    let enable_toc = post.frontmatter.toc || default_toc;
    let opts = markdown::RenderOptions {
        enable_math: post.frontmatter.mathjax,
        enable_toc,
        code_theme: code_theme.to_string(),
    };

    // Full body rendering
    let content = render_with_hexo_tags(&post.body, &opts);

    // Excerpt rendering
    let excerpt = render_with_hexo_tags(&post.excerpt, &opts);

    // TOC from a separate pass (we need the headings)
    let toc_result = markdown::render_markdown(&post.body, &markdown::RenderOptions {
        enable_math: false,
        enable_toc: true,
        code_theme: code_theme.to_string(),
    });
    let toc_html = toc::generate_toc(&toc_result.toc);

    PostHtml {
        content,
        excerpt_html: excerpt,
        toc_html,
        has_math: post.frontmatter.mathjax,
    }
}

/// Render markdown with hexo tag support (the 3-pass pipeline described in the module docs).
pub fn render_with_hexo_tags(text: &str, opts: &markdown::RenderOptions) -> String {
    // Pass 1: Extract hexo tags, replace with placeholders
    let extracted = hexo_tags::extract(text);

    // Pass 2: Render markdown (with placeholders as literal text)
    let mut result = markdown::render_markdown(&extracted.text, opts);

    // Pass 3: Resolve placeholders — render inner content + wrap in tag HTML
    for (i, tag) in extracted.tags.iter().enumerate() {
        let inner_html = match tag {
            hexo_tags::HexoTag::Note { content, .. } => {
                let inner = render_with_hexo_tags(content, opts);
                format!("<p>{}</p>", inner.trim())
            }
            hexo_tags::HexoTag::GroupPicture { content, .. } => {
                render_with_hexo_tags(content, opts)
            }
            _ => String::new(),
        };

        let tag_html = hexo_tags::render_tag(tag, &inner_html);
        let placeholder = format!("&lt;&lt;PLACEHOLDER_{i}&gt;&gt;");
        result.html = result.html.replace(&placeholder, &tag_html);

        let placeholder_raw = format!("<<PLACEHOLDER_{i}>>");
        result.html = result.html.replace(&placeholder_raw, &tag_html);
    }

    // Fallback: fix unrendered markdown due to CJK-adjacent delimiters
    result.html = STRIKETHROUGH_RE
        .replace_all(&result.html, "<del>$1</del>")
        .into_owned();
    result.html = BOLD_RE
        .replace_all(&result.html, "<strong>$1</strong>")
        .into_owned();

    result.html
}
