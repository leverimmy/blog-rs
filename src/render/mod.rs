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
pub mod wechat;

use regex::Regex;
use std::sync::LazyLock;

use crate::post::Post;

// Wrap <img> with alt text into <figure> with <figcaption>
static IMG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"<img\s+([^>]*)alt="([^"]+)"([^>]*)>"#).unwrap()
});
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
pub fn render_post(
    post: &Post,
    code_theme: &str,
    default_toc: bool,
    wechat_previews: Option<&wechat::WechatPreviewStore>,
) -> PostHtml {
    let enable_toc = post.frontmatter.toc || default_toc;
    let opts = markdown::RenderOptions {
        enable_math: post.frontmatter.mathjax,
        enable_toc,
        code_theme: code_theme.to_string(),
        wechat_previews,
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
        wechat_previews: None,
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

        // Replace escaped placeholder (from HTML encoding in markdown)
        let placeholder = format!("&lt;&lt;PLACEHOLDER_{i}&gt;&gt;");
        result.html = result.html.replace(&placeholder, &tag_html);

        // Replace raw placeholder and strip wrapping <p> if it creates invalid nesting
        let placeholder_raw = format!("<<PLACEHOLDER_{i}>>");
        let wrapped = format!("<p>{placeholder_raw}</p>");
        if result.html.contains(&wrapped) {
            result.html = result.html.replace(&wrapped, &tag_html);
        } else {
            result.html = result.html.replace(&placeholder_raw, &tag_html);
        }
    }

    // Wrap images with alt text in <figure> (skip those already in a <figure>)
    // First, temporarily replace existing <figure>...</figure> blocks with placeholders
    static FIGURE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<figure>.*?</figure>").unwrap()
    });
    let mut saved_figures: Vec<String> = Vec::new();
    let protected = FIGURE_RE.replace_all(&result.html, |caps: &regex::Captures| {
        let idx = saved_figures.len();
        saved_figures.push(caps[0].to_string());
        format!("<<SAVEDFIG{idx}>>")
    }).into_owned();

    // Now wrap remaining bare <img> tags with alt text
    let wrapped = IMG_RE
        .replace_all(&protected, |caps: &regex::Captures| {
            let before = &caps[1];
            let alt = &caps[2];
            let after = &caps[3];
            format!("<figure><img {before}alt=\"{alt}\"{after}><figcaption>{alt}</figcaption></figure>")
        })
        .into_owned();

    // Restore saved figures
    let mut html = wrapped;
    for (i, fig) in saved_figures.iter().enumerate() {
        html = html.replace(&format!("<<SAVEDFIG{i}>>"), fig);
    }
    result.html = html;

    // Fix invalid <p> wrapping block elements (from placeholder replacement)
    // Run twice to handle nested cases (outer <p> wrapping a <div> that contains <figure>)
    static P_WRAP_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"<p>\s*(<(?:div|details|figure)\b[^>]*>[\s\S]*?</(?:div|details|figure)>)\s*</p>").unwrap()
    });
    for _ in 0..2 {
        result.html = P_WRAP_BLOCK_RE
            .replace_all(&result.html, "$1")
            .into_owned();
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
