//! Markdown-to-HTML conversion via pulldown-cmark.
//!
//! Intercepts parser events for specialized processing:
//! - `InlineMath` / `DisplayMath` → KaTeX rendering
//! - `CodeBlock` (fenced) → syntect highlighting or mermaid passthrough
//! - `Heading` → TOC item collection + `id` attribute injection
//! - WeChat public account links → share-card style anchors

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use regex::{Captures, Regex};
use std::sync::LazyLock;

use crate::utils::html_escape;

use super::code_highlight;
use super::math;
use super::toc::{self, TocItem};
use super::wechat::{WechatPreview, WechatPreviewStore};

/// Options controlling which features are active during markdown rendering.
pub struct RenderOptions<'a> {
    pub enable_math: bool,
    pub enable_toc: bool,
    pub code_theme: String,
    pub wechat_previews: Option<&'a WechatPreviewStore>,
}

/// Result of rendering: the HTML output and collected TOC heading items.
pub struct RenderResult {
    pub html: String,
    pub toc: Vec<TocItem>,
}

static MERMAID_INLINE_MATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)(^|[^\\$])\$([^$\n]+?)\$").unwrap());

/// Render a markdown string to HTML, intercepting math/code/heading events.
pub fn render_markdown(markdown: &str, opts: &RenderOptions) -> RenderResult {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    if opts.enable_math {
        options.insert(Options::ENABLE_MATH);
    }

    let parser = Parser::new_ext(markdown, options);
    let mut events: Vec<Event> = parser.collect();
    let mut toc_items: Vec<TocItem> = Vec::new();

    // Process events
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::InlineMath(latex) if opts.enable_math => {
                let rendered = math::render_inline(latex);
                events[i] = Event::Html(rendered.into());
            }
            Event::DisplayMath(latex) if opts.enable_math => {
                let rendered = math::render_display(latex);
                events[i] = Event::Html(rendered.into());
            }
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                let fence_info = info.to_string();
                let mut code = String::new();
                let start = i + 1;
                let mut end = start;
                for j in start..events.len() {
                    if matches!(events[j], Event::End(TagEnd::CodeBlock)) {
                        end = j;
                        break;
                    }
                    if let Event::Text(t) = &events[j] {
                        if !code.is_empty() {
                            code.push('\n');
                        }
                        code.push_str(t);
                    }
                }

                // Check for mermaid blocks — render as <pre class="mermaid">
                let lang = fence_info.split_whitespace().next().unwrap_or("");
                let highlighted = if lang == "mermaid" {
                    let code = if opts.enable_math {
                        normalize_mermaid_inline_math(&code)
                    } else {
                        code
                    };
                    format!(
                        "<div class=\"mermaid-container\"><pre class=\"mermaid\">{}</pre></div>",
                        html_escape(&code)
                    )
                } else {
                    code_highlight::highlight(&code, &fence_info, &opts.code_theme)
                };
                // Replace the code block with highlighted HTML
                events[i] = Event::Html(highlighted.into());
                for j in (i + 1)..=end {
                    if j < events.len() {
                        events[j] = Event::Html("".into());
                    }
                }
                i = end + 1;
                continue;
            }
            Event::Start(Tag::Heading { level, .. }) if opts.enable_toc => {
                let heading_level = *level as u8;
                // Collect heading text
                let mut text = String::new();
                let mut j = i + 1;
                while j < events.len() {
                    match &events[j] {
                        Event::Text(t) | Event::Code(t) => text.push_str(t),
                        Event::End(TagEnd::Heading(_)) => break,
                        _ => {}
                    }
                    j += 1;
                }
                let id = toc::slugify(&text);
                toc_items.push(TocItem {
                    level: heading_level,
                    id: id.clone(),
                    text,
                });
                // Inject id into the heading tag
                events[i] = Event::Html(
                    format!("<h{heading_level} id=\"{id}\">").into(),
                );
                // Skip to after End heading to inject closing tag properly
                // We need to replace the End event too
                if j < events.len() {
                    events[j] = Event::Html(format!("</h{heading_level}>").into());
                }
            }
            Event::Start(Tag::Link { dest_url, .. }) if is_wechat_mp_link(dest_url) => {
                if let Some(end) = find_link_end(&events, i) {
                    let card = render_wechat_card(dest_url, &events[(i + 1)..end], opts);
                    events[i] = Event::Html(card.into());
                    for j in (i + 1)..=end {
                        events[j] = Event::Html("".into());
                    }
                    i = end + 1;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let mut html_output = String::new();
    pulldown_cmark::html::push_html(&mut html_output, events.into_iter());

    RenderResult {
        html: html_output,
        toc: toc_items,
    }
}

fn normalize_mermaid_inline_math(code: &str) -> String {
    MERMAID_INLINE_MATH_RE
        .replace_all(code, |caps: &Captures| {
            format!("{}$${}$$", &caps[1], &caps[2])
        })
        .into_owned()
}

fn find_link_end(events: &[Event], start: usize) -> Option<usize> {
    let mut depth = 0usize;

    for (idx, event) in events.iter().enumerate().skip(start + 1) {
        match event {
            Event::Start(Tag::Link { .. }) => depth += 1,
            Event::End(TagEnd::Link) if depth == 0 => return Some(idx),
            Event::End(TagEnd::Link) => depth -= 1,
            _ => {}
        }
    }

    None
}

fn is_wechat_mp_link(url: &str) -> bool {
    let normalized = url.trim_start().to_ascii_lowercase();
    normalized.starts_with("https://mp.weixin.qq.com/")
        || normalized.starts_with("http://mp.weixin.qq.com/")
}

fn render_wechat_card(url: &str, inner_events: &[Event], opts: &RenderOptions) -> String {
    let plain_title = collect_plain_text(inner_events);
    let hover_title_attr = non_empty(&plain_title)
        .filter(|title| !should_use_default_wechat_title(title, url))
        .map(|title| format!(" title=\"{}\"", html_escape(title)))
        .unwrap_or_default();
    let preview = opts.wechat_previews.and_then(|store| store.get(url));
    let fallback_title = if preview.is_some() {
        "微信公众号文章"
    } else {
        "无法访问该推送"
    };
    let title_html = preview
        .as_ref()
        .and_then(|preview| non_empty(&preview.title))
        .map(html_escape)
        .unwrap_or_else(|| fallback_title.to_string());
    let description_html = preview
        .as_ref()
        .and_then(|preview| non_empty(&preview.description))
        .map(|description| {
            let description = compact_preview_text(description, 120);
            format!(
                "<span class=\"wechat-card-description\">{}</span>",
                html_escape(&description)
            )
        })
        .unwrap_or_default();
    let account_name = preview
        .as_ref()
        .and_then(|preview| non_empty(&preview.account_name))
        .unwrap_or("微信公众平台");
    let avatar_html = preview
        .as_ref()
        .and_then(|preview| non_empty(&preview.local_avatar_url))
        .map(|avatar_url| {
            format!(
                "<span class=\"wechat-card-avatar\" style=\"background-image:url('{}')\" aria-hidden=\"true\"></span>",
                html_escape(avatar_url)
            )
        })
        .unwrap_or_default();
    let media = select_wechat_media(preview.as_ref());
    let card_class = media.card_class();
    let hero_html = media.hero_html();
    let side_media_html = media.side_html();

    format!(
        "<a class=\"{card_class}\"{hover_title_attr} href=\"{}\" target=\"_blank\" rel=\"noopener\">{hero_html}<span class=\"wechat-card-body\"><span class=\"wechat-card-title\">{}</span>{description_html}<span class=\"wechat-card-meta\">{avatar_html}<span class=\"wechat-card-badge\">公众号</span><span class=\"wechat-card-source\">{}</span></span></span>{side_media_html}</a>",
        html_escape(url),
        title_html,
        html_escape(account_name)
    )
}

enum WechatCardMedia<'a> {
    Hero(&'a str),
    Side(&'a str),
    Icon,
}

#[derive(Clone, Copy)]
struct WechatImageCandidate<'a> {
    url: &'a str,
    is_square: bool,
}

impl WechatCardMedia<'_> {
    fn card_class(&self) -> &'static str {
        match self {
            Self::Hero(_) => "wechat-card has-hero-cover",
            Self::Side(_) => "wechat-card has-cover",
            Self::Icon => "wechat-card",
        }
    }

    fn hero_html(&self) -> String {
        match self {
            Self::Hero(url) => format!(
                "<img class=\"wechat-card-hero\" src=\"{}\" alt=\"\" loading=\"lazy\">",
                html_escape(url)
            ),
            Self::Side(_) | Self::Icon => String::new(),
        }
    }

    fn side_html(&self) -> String {
        match self {
            Self::Side(url) => format!(
                "<img class=\"wechat-card-cover\" src=\"{}\" alt=\"\" loading=\"lazy\">",
                html_escape(url)
            ),
            Self::Hero(_) => String::new(),
            Self::Icon => "<span class=\"wechat-card-icon\" aria-hidden=\"true\"></span>".to_string(),
        }
    }
}

fn select_wechat_media(preview: Option<&WechatPreview>) -> WechatCardMedia<'_> {
    let Some(preview) = preview else {
        return WechatCardMedia::Icon;
    };

    let cover = wechat_image_candidate(&preview.local_cover_url, preview.cover_is_square);
    let thumbnail =
        wechat_image_candidate(&preview.local_thumbnail_url, preview.thumbnail_is_square);

    if let Some(candidate) = cover.filter(|candidate| !candidate.is_square) {
        return WechatCardMedia::Hero(candidate.url);
    }

    if let Some(candidate) = thumbnail.filter(|candidate| candidate.is_square) {
        return WechatCardMedia::Side(candidate.url);
    }

    if let Some(candidate) = cover.filter(|candidate| candidate.is_square) {
        return WechatCardMedia::Side(candidate.url);
    }

    if let Some(candidate) = thumbnail.filter(|candidate| !candidate.is_square) {
        return WechatCardMedia::Hero(candidate.url);
    }

    WechatCardMedia::Icon
}

fn wechat_image_candidate(url: &str, is_square: bool) -> Option<WechatImageCandidate<'_>> {
    non_empty(url).map(|url| WechatImageCandidate { url, is_square })
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn compact_preview_text(value: &str, max_chars: usize) -> String {
    let compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compacted.chars();
    let shortened: String = chars.by_ref().take(max_chars).collect();

    if chars.next().is_some() {
        format!("{shortened}...")
    } else {
        shortened
    }
}

fn should_use_default_wechat_title(title: &str, url: &str) -> bool {
    let title = title.trim();
    title.is_empty() || title.eq_ignore_ascii_case(url.trim()) || is_wechat_mp_link(title)
}

fn collect_plain_text(events: &[Event]) -> String {
    let mut text = String::new();

    for event in events {
        match event {
            Event::Text(t) | Event::Code(t) | Event::InlineMath(t) | Event::DisplayMath(t) => {
                text.push_str(t);
            }
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }

    text
}
