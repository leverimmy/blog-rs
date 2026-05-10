//! Hexo-compatible tag plugins.
//!
//! Supported tags:
//! - `{% note TYPE %}...{% endnote %}` — colored note boxes (default, primary, info, success, warning, danger)
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

static NOTE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)\{%\s*note\s+(\w+)\s*%\}(.*?)\{%\s*endnote\s*%\}").unwrap()
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

/// Extract all hexo tags from markdown, replacing them with `<<PLACEHOLDER_N>>` markers.
pub fn extract(markdown: &str) -> ExtractedTags {
    let mut tags = Vec::new();
    let mut text = markdown.to_string();

    // Extract block tags first (they may contain other tags)

    // Note blocks
    text = NOTE_RE.replace_all(&text, |caps: &regex::Captures| {
        let idx = tags.len();
        tags.push(HexoTag::Note {
            note_type: caps[1].to_string(),
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

    // Self-closing tags

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
        HexoTag::Note { note_type, .. } => {
            // note_type is validated by regex to only match \w+
            format!(
                "<div class=\"note note-{note_type}\">\n{inner_html}\n</div>"
            )
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
