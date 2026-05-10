//! Site configuration loading from `config.toml`.
//!
//! Supports: basic site info, permalink pattern, pagination, avatar/social links,
//! motto, ICP/gongan filing, theme settings (toc, code_theme, accent_color),
//! and Giscus comment configuration.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct SiteConfig {
    pub title: String,
    #[serde(default)]
    pub subtitle: String,
    pub author: String,
    #[allow(dead_code)]
    pub url: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub avatar: String,
    #[serde(default)]
    pub github: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub motto: String,
    #[serde(default = "default_since")]
    pub since: u32,
    #[serde(default)]
    pub icp: String,
    #[serde(default)]
    pub gongan: String,
    #[serde(default)]
    pub gongan_id: String,
    #[serde(default = "default_permalink")]
    pub permalink: String,
    #[serde(default = "default_per_page")]
    pub per_page: usize,
    #[serde(default = "default_source_dir")]
    pub source_dir: PathBuf,
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub giscus: GiscusConfig,
}

#[derive(Debug, Default, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_true")]
    pub toc: bool,
    #[serde(default = "default_code_theme")]
    pub code_theme: String,
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
}

#[derive(Debug, Default, Deserialize)]
pub struct GiscusConfig {
    #[serde(default)]
    pub repo: String,
    #[serde(default)]
    pub repo_id: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub category_id: String,
}

impl GiscusConfig {
    pub fn is_enabled(&self) -> bool {
        !self.repo.is_empty() && !self.repo_id.is_empty()
    }
}

fn default_language() -> String { "zh-CN".into() }
fn default_since() -> u32 { 2022 }
fn default_permalink() -> String { ":year/:month/:day/:id/".into() }
fn default_per_page() -> usize { 10 }
fn default_source_dir() -> PathBuf { "content".into() }
fn default_output_dir() -> PathBuf { "public".into() }
fn default_true() -> bool { true }
fn default_code_theme() -> String { "InspiredGitHub".into() }
fn default_accent_color() -> String { "#428bca".into() }

impl SiteConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| "Failed to parse config.toml")?;
        Ok(config)
    }

    pub fn posts_dir(&self) -> PathBuf {
        self.source_dir.join("_posts")
    }

    pub fn gallery_dir(&self) -> PathBuf {
        self.source_dir.join("gallery")
    }
}
