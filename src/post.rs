//! Post parsing: frontmatter deserialization, excerpt splitting, permalink computation.

use anyhow::{Context, Result};
use chrono::NaiveDateTime;
use serde::Deserialize;
use std::path::Path;

fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let s = String::deserialize(deserializer)?;
    // Try "2025-12-21 23:29:01" format first
    if let Ok(dt) = NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S") {
        return Ok(dt);
    }
    // Try "2025-12-21" date only
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        return Ok(d.and_hms_opt(0, 0, 0).expect("midnight is always valid"));
    }
    Err(D::Error::custom(format!("Cannot parse date: {s}")))
}

#[derive(Debug, Clone, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    #[serde(deserialize_with = "deserialize_date")]
    pub date: NaiveDateTime,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub mathjax: bool,
    #[serde(default)]
    pub toc: bool,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub password: Option<String>,
}

#[derive(Debug)]
pub struct Post {
    pub frontmatter: Frontmatter,
    pub body: String,
    pub excerpt: String,
    pub has_more: bool,
    pub permalink: String,
    #[allow(dead_code)]
    pub source_path: std::path::PathBuf,
}

impl Post {
    pub fn parse(source_path: &Path, permalink_pattern: &str) -> Result<Self> {
        let content = std::fs::read_to_string(source_path)
            .with_context(|| format!("Failed to read {}", source_path.display()))?;

        let (fm, body) = split_frontmatter_raw(&content)
            .with_context(|| format!("Failed to parse frontmatter in {}", source_path.display()))?;

        let (excerpt, body, has_more) = split_excerpt(&body);

        let permalink = compute_permalink(&fm, permalink_pattern);

        Ok(Post {
            frontmatter: fm,
            body,
            excerpt,
            has_more,
            permalink,
            source_path: source_path.to_path_buf(),
        })
    }
}

/// Parse YAML frontmatter from raw file content.
/// Returns `(frontmatter, body)` where body is everything after the closing `---`.
pub fn split_frontmatter_raw(content: &str) -> Result<(Frontmatter, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        anyhow::bail!("Missing frontmatter delimiter");
    }

    let rest = &trimmed[3..];
    let end = rest.find("\n---").with_context(|| "Unclosed frontmatter")?;

    let yaml_str = &rest[..end];
    let fm: Frontmatter = serde_yaml::from_str(yaml_str)
        .with_context(|| "Failed to parse YAML frontmatter")?;

    let body = rest[end + 4..].to_string();
    Ok((fm, body))
}

/// Split body into (excerpt, full_body, has_more) at the `<!--more-->` tag.
///
/// Uses the same regex as Hexo (`/<!-- ?more ?-->/i`), matching at most one
/// optional space before and after "more".
fn split_excerpt(body: &str) -> (String, String, bool) {
    let re = regex::Regex::new(r"(?i)<!-- ?more ?-->").unwrap();
    if let Some(cap) = re.find(body) {
        let excerpt = body[..cap.start()].to_string();
        let full_body = body[..cap.start()].to_string() + &body[cap.end()..];
        (excerpt, full_body, true)
    } else {
        let excerpt: String = body.chars().take(300).collect();
        (excerpt, body.to_string(), false)
    }
}

fn compute_permalink(fm: &Frontmatter, pattern: &str) -> String {
    let date = fm.date;
    let result = pattern
        .replace(":year", &format!("{:04}", date.format("%Y")))
        .replace(":month", &format!("{:02}", date.format("%m")))
        .replace(":day", &format!("{:02}", date.format("%d")))
        .replace(":id", &fm.id);
    // Ensure trailing slash
    if result.ends_with('/') {
        result
    } else {
        result + "/"
    }
}
