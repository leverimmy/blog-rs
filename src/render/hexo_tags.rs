//! Hexo-compatible tag plugins.
//!
//! Supported tags:
//! - `{% note TYPE %}...{% endnote %}` — colored note boxes
//! - `{% note TYPE [no-icon] TEXT %}...{% endnote %}` — collapsible note with summary
//! - `{% grouppicture LAYOUT %}...{% endgrouppicture %}` — grid image layout
//! - `{% video URL %}` — embedded video player
//! - `{% pdf URL [HEIGHT] %}` — embedded PDF viewer

use regex::Regex;
use std::sync::LazyLock;

/// A parsed hexo tag with its arguments and inner content.
#[derive(Debug, Clone)]
pub enum HexoTag {
    Note {
        note_type: String,
        no_icon: bool,
        summary: Option<String>,
        content: String,
    },
    GroupPicture {
        layout: String,
        content: String,
    },
    Video {
        src: String,
    },
    Pdf {
        url: String,
        height: String,
    },
}

// Block note: {% note ARGS %}...{% endnote %}
// ARGS can be: TYPE, TYPE no-icon, TYPE TEXT, TYPE no-icon TEXT
static NOTE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\{%\s*note\s+(.*?)\s*%\}(.*?)\{%\s*endnote\s*%\}").unwrap()
});

static GROUPPIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\{%\s*grouppicture\s+([\d-]+)\s*%\}(.*?)\{%\s*endgrouppicture\s*%\}").unwrap()
});

static VIDEO_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{%\s*video\s+(.+?)\s*%\}").unwrap()
});

static PDF_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{%\s*pdf\s+(\S+)(?:\s+\[([^\]]+)\])?\s*%\}").unwrap()
});

/// Result of extracting hexo tags: the tags themselves and the markdown with tags replaced by placeholders.
pub struct ExtractedTags {
    pub tags: Vec<HexoTag>,
    pub text: String,
}

const NOTE_TYPES: &[&str] = &["default", "primary", "info", "success", "warning", "danger"];

/// Parse note tag arguments: TYPE [no-icon] [summary text]
fn parse_note_args(args: &str) -> (String, bool, Option<String>) {
    let words: Vec<&str> = args.split_whitespace().collect();
    let mut note_type = "default".to_string();
    let mut no_icon = false;
    let mut summary_parts: Vec<&str> = Vec::new();

    let mut i = 0;
    if i < words.len() && NOTE_TYPES.contains(&words[i]) {
        note_type = words[i].to_string();
        i += 1;
    }
    if i < words.len() && words[i] == "no-icon" {
        no_icon = true;
        i += 1;
    }
    if i < words.len() {
        summary_parts = words[i..].to_vec();
    }

    let summary = if summary_parts.is_empty() {
        None
    } else {
        Some(summary_parts.join(" "))
    };

    (note_type, no_icon, summary)
}

/// Extract all hexo tags from markdown, replacing them with `<<PLACEHOLDER_N>>` markers.
pub fn extract(markdown: &str) -> ExtractedTags {
    let mut tags = Vec::new();
    let mut text = markdown.to_string();

    // Extract block notes (they may span multiple lines)
    text = NOTE_BLOCK_RE.replace_all(&text, |caps: &regex::Captures| {
        let (note_type, no_icon, summary) = parse_note_args(&caps[1]);
        let idx = tags.len();
        tags.push(HexoTag::Note {
            note_type,
            no_icon,
            summary,
            content: caps[2].to_string(),
        });
        format!("<<PLACEHOLDER_{idx}>>")
    }).into_owned();

    // Group pictures
    text = GROUPPIC_RE.replace_all(&text, |caps: &regex::Captures| {
        let idx = tags.len();
        tags.push(HexoTag::GroupPicture {
            layout: caps[1].to_string(),
            content: caps[2].to_string(),
        });
        format!("<<PLACEHOLDER_{idx}>>")
    }).into_owned();

    // Video
    text = VIDEO_RE.replace_all(&text, |caps: &regex::Captures| {
        let idx = tags.len();
        tags.push(HexoTag::Video {
            src: caps[1].to_string(),
        });
        format!("<<PLACEHOLDER_{idx}>>")
    }).into_owned();

    // PDF
    text = PDF_RE.replace_all(&text, |caps: &regex::Captures| {
        let idx = tags.len();
        tags.push(HexoTag::Pdf {
            url: caps[1].to_string(),
            height: caps.get(2).map(|m| m.as_str().to_string()).unwrap_or_else(|| "500px".into()),
        });
        format!("<<PLACEHOLDER_{idx}>>")
    }).into_owned();

    ExtractedTags { tags, text }
}

/// Render a hexo tag to its final HTML, with `inner_html` as the rendered content.
pub fn render_tag(tag: &HexoTag, inner_html: &str) -> String {
    match tag {
        HexoTag::Note { note_type, no_icon, summary, .. } => {
            let no_icon_class = if *no_icon { " no-icon" } else { "" };
            if let Some(summary_text) = summary {
                let escaped = crate::utils::html_escape(summary_text);
                format!(
                    "<details class=\"note note-{note_type}{no_icon_class}\"><summary>{escaped}</summary>\n{inner_html}\n</details>"
                )
            } else {
                format!(
                    "<div class=\"note note-{note_type}{no_icon_class}\">\n{inner_html}\n</div>"
                )
            }
        }
        HexoTag::GroupPicture { layout, .. } => {
            let cols = layout.split('-').next().unwrap_or("2").parse::<usize>().unwrap_or(2);
            format!(
                "<div class=\"group-picture\" style=\"display: grid; grid-template-columns: repeat({cols}, 1fr); gap: 4px;\">\n{inner_html}\n</div>"
            )
        }
        HexoTag::Video { src } => {
            if let Some(safe) = safe_url(src) {
                format!(
                    "<div class=\"video-container\"><video src=\"{safe}\" preload=\"metadata\" controls playsinline></video></div>"
                )
            } else {
                format!("<div class=\"video-container\"><!-- blocked unsafe URL: {} --></div>", crate::utils::html_escape(src))
            }
        }
        HexoTag::Pdf { url, height } => {
            if let Some(safe) = safe_url(url) {
                format!(
                    "<div class=\"pdf-container\"><iframe src=\"{safe}\" width=\"100%\" height=\"{height}\" frameborder=\"0\"></iframe></div>"
                )
            } else {
                format!("<div class=\"pdf-container\"><!-- blocked unsafe URL: {} --></div>", crate::utils::html_escape(url))
            }
        }
    }
}

/// Only allow http(s) and relative URLs. Blocks javascript:, data:, vbscript: etc.
fn safe_url(url: &str) -> Option<String> {
    let trimmed = url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with('/') {
        Some(trimmed.to_string())
    } else {
        None
    }
}
