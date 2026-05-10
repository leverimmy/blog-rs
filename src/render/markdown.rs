//! Markdown-to-HTML conversion via pulldown-cmark.
//!
//! Intercepts parser events for specialized processing:
//! - `InlineMath` / `DisplayMath` → KaTeX rendering
//! - `CodeBlock` (fenced) → syntect highlighting or mermaid passthrough
//! - `Heading` → TOC item collection + `id` attribute injection

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::utils::html_escape;

use super::code_highlight;
use super::math;
use super::toc::{self, TocItem};

/// Options controlling which features are active during markdown rendering.
pub struct RenderOptions {
    pub enable_math: bool,
    pub enable_toc: bool,
    pub code_theme: String,
}

/// Result of rendering: the HTML output and collected TOC heading items.
pub struct RenderResult {
    pub html: String,
    pub toc: Vec<TocItem>,
}

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
