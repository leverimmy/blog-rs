//! Build-time metadata fetching for WeChat public account article cards.

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use crate::utils::html_unescape;

const CARD_ASSET_URL_PREFIX: &str = "/wechat-previews/";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WechatPreview {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cover_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub local_cover_url: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub cover_is_square: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub thumbnail_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub local_thumbnail_url: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub thumbnail_is_square: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account_avatar_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub local_avatar_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fetched_at: String,
}

pub struct WechatPreviewStore {
    cache_path: PathBuf,
    asset_cache_dir: PathBuf,
    output_asset_dir: PathBuf,
    entries: RefCell<HashMap<String, WechatPreview>>,
    failed_urls: RefCell<HashSet<String>>,
    dirty: Cell<bool>,
}

impl WechatPreviewStore {
    pub fn load(cache_path: impl Into<PathBuf>, output_dir: &Path) -> Self {
        let cache_path = cache_path.into();
        let entries = fs::read_to_string(&cache_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        Self {
            asset_cache_dir: cache_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("wechat-assets"),
            output_asset_dir: output_dir.join(CARD_ASSET_URL_PREFIX.trim_start_matches('/')),
            cache_path,
            entries: RefCell::new(entries),
            failed_urls: RefCell::new(HashSet::new()),
            dirty: Cell::new(false),
        }
    }

    pub fn get(&self, url: &str) -> Option<WechatPreview> {
        let key = normalize_wechat_url(url);
        let cached = self.entries.borrow().get(&key).cloned();

        if let Some(preview) = cached.as_ref() {
            if preview.needs_refresh() {
                log::info!("Refreshing incomplete WeChat metadata for {key}");
            } else {
                self.ensure_output_images(preview);
                return Some(preview.clone());
            }
        }

        if self.failed_urls.borrow().contains(&key) {
            if let Some(preview) = cached {
                self.ensure_output_images(&preview);
                return Some(preview);
            }
            return None;
        }

        match self.fetch_preview(&key) {
            Ok(preview) if preview.has_content() => {
                self.ensure_output_images(&preview);
                self.entries.borrow_mut().insert(key, preview.clone());
                self.dirty.set(true);
                Some(preview)
            }
            Ok(_) => {
                log::warn!("No WeChat metadata found for {key}");
                self.failed_urls.borrow_mut().insert(key);
                cached.inspect(|preview| self.ensure_output_images(preview))
            }
            Err(err) => {
                log::warn!("Failed to fetch WeChat metadata for {key}: {err:#}");
                self.failed_urls.borrow_mut().insert(key);
                cached.inspect(|preview| self.ensure_output_images(preview))
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        if !self.dirty.get() {
            return Ok(());
        }

        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&*self.entries.borrow())?;
        fs::write(&self.cache_path, json)?;
        Ok(())
    }

    fn fetch_preview(&self, url: &str) -> Result<WechatPreview> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36")
            .build()?;

        let html = client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .context("request failed")?
            .error_for_status()
            .context("request returned an error status")?
            .text()
            .context("failed to read response body")?;

        let mut preview = parse_wechat_html(&html);
        preview.fetched_at = chrono::Local::now().to_rfc3339();

        if !preview.cover_url.is_empty() {
            match self.cache_image(&client, url, &preview.cover_url) {
                Ok(local_url) => {
                    preview.cover_is_square = self
                        .cached_image_dimensions(&local_url)
                        .is_some_and(|(width, height)| is_squareish(width, height));
                    preview.local_cover_url = local_url;
                }
                Err(err) => log::warn!("Failed to cache WeChat cover image for {url}: {err:#}"),
            }
        }
        if !preview.thumbnail_url.is_empty() {
            match self.cache_image(&client, url, &preview.thumbnail_url) {
                Ok(local_url) => {
                    preview.thumbnail_is_square = self
                        .cached_image_dimensions(&local_url)
                        .is_some_and(|(width, height)| is_squareish(width, height));
                    preview.local_thumbnail_url = local_url;
                }
                Err(err) => {
                    log::warn!("Failed to cache WeChat thumbnail image for {url}: {err:#}")
                }
            }
        }
        if !preview.account_avatar_url.is_empty() {
            match self.cache_image(&client, url, &preview.account_avatar_url) {
                Ok(local_url) => preview.local_avatar_url = local_url,
                Err(err) => log::warn!("Failed to cache WeChat avatar image for {url}: {err:#}"),
            }
        }

        Ok(preview)
    }

    fn cache_image(
        &self,
        client: &reqwest::blocking::Client,
        article_url: &str,
        image_url: &str,
    ) -> Result<String> {
        let image_url = absolutize_url(image_url);
        let filename = image_filename(&image_url);
        let cache_path = self.asset_cache_dir.join(&filename);

        if !cache_path.exists() {
            fs::create_dir_all(&self.asset_cache_dir)?;
            let bytes = client
                .get(&image_url)
                .header(reqwest::header::REFERER, article_url)
                .send()
                .context("image request failed")?
                .error_for_status()
                .context("image request returned an error status")?
                .bytes()
                .context("failed to read image bytes")?;
            fs::write(&cache_path, &bytes)?;
        }

        fs::create_dir_all(&self.output_asset_dir)?;
        fs::copy(&cache_path, self.output_asset_dir.join(&filename))?;
        Ok(format!("{CARD_ASSET_URL_PREFIX}{filename}"))
    }

    fn ensure_output_images(&self, preview: &WechatPreview) {
        self.ensure_output_image(&preview.local_cover_url);
        self.ensure_output_image(&preview.local_thumbnail_url);
        self.ensure_output_image(&preview.local_avatar_url);
    }

    fn ensure_output_image(&self, local_url: &str) {
        let Some(filename) = local_url.strip_prefix(CARD_ASSET_URL_PREFIX) else {
            return;
        };

        let cache_path = self.asset_cache_dir.join(filename);
        if !cache_path.exists() {
            return;
        }

        if let Err(err) = fs::create_dir_all(&self.output_asset_dir)
            .and_then(|_| fs::copy(&cache_path, self.output_asset_dir.join(filename)).map(|_| ()))
        {
            log::warn!("Failed to copy cached WeChat image {filename}: {err}");
        }
    }

    fn cached_image_dimensions(&self, local_url: &str) -> Option<(u32, u32)> {
        let filename = local_url.strip_prefix(CARD_ASSET_URL_PREFIX)?;
        let bytes = fs::read(self.asset_cache_dir.join(filename)).ok()?;
        image_dimensions(&bytes)
    }
}

impl WechatPreview {
    fn has_content(&self) -> bool {
        !self.title.is_empty()
            || !self.description.is_empty()
            || !self.account_name.is_empty()
            || !self.cover_url.is_empty()
            || !self.thumbnail_url.is_empty()
            || !self.account_avatar_url.is_empty()
    }

    fn needs_refresh(&self) -> bool {
        self.has_suspect_text() || self.cover_asset_missing() || self.thumbnail_asset_missing()
    }

    fn has_suspect_text(&self) -> bool {
        self.account_name.is_empty()
            || self.account_name.starts_with("gh_")
            || self.account_name.starts_with("data-")
            || self.title.starts_with("data-")
            || self.description.contains("\\x")
    }

    fn cover_asset_missing(&self) -> bool {
        !self.cover_url.is_empty() && self.local_cover_url.is_empty()
    }

    fn thumbnail_asset_missing(&self) -> bool {
        !self.thumbnail_url.is_empty() && self.local_thumbnail_url.is_empty()
    }
}

pub fn parse_wechat_html(html: &str) -> WechatPreview {
    let mut meta = collect_meta_tags(html);

    WechatPreview {
        title: first_non_empty([
            js_var(html, "msg_title"),
            cgi_data_value(html, "title"),
            meta.remove("og:title"),
            meta.remove("twitter:title"),
        ]),
        description: first_non_empty([
            js_var(html, "msg_desc"),
            cgi_data_value(html, "desc"),
            meta.remove("og:description"),
            meta.remove("description"),
            meta.remove("twitter:description"),
        ]),
        account_name: first_non_empty([
            js_var(html, "nickname"),
            js_var(html, "nick_name"),
            cgi_data_value(html, "nick_name"),
            html_text_by_id(html, "js_name"),
            js_var(html, "user_name"),
            cgi_data_value(html, "user_name"),
            meta.remove("author"),
        ]),
        cover_url: first_non_empty([
            js_var(html, "msg_cdn_url"),
            cgi_data_value(html, "cdn_url"),
            cgi_data_value(html, "cover"),
            meta.remove("og:image"),
            meta.remove("twitter:image"),
        ]),
        thumbnail_url: first_non_empty([
            js_var(html, "cdn_url_1_1"),
            cgi_data_value(html, "cdn_url_1_1"),
            cgi_data_value(html, "cdn_1_1_img"),
        ]),
        account_avatar_url: first_non_empty([
            js_var(html, "round_head_img"),
            cgi_data_value(html, "round_head_img"),
            js_var(html, "hd_head_img"),
            cgi_data_value(html, "hd_head_img"),
        ]),
        ..Default::default()
    }
}

fn normalize_wechat_url(url: &str) -> String {
    url.trim().to_string()
}

fn collect_meta_tags(html: &str) -> HashMap<String, String> {
    static META_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?is)<meta\s+([^>]+)>"#).unwrap());
    static ATTR_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"(?is)([a-zA-Z_:.-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#).unwrap()
    });

    let mut values = HashMap::new();
    for caps in META_RE.captures_iter(html) {
        let mut key = None;
        let mut content = None;

        for attr in ATTR_RE.captures_iter(&caps[1]) {
            let attr_name = attr[1].to_ascii_lowercase();
            let attr_value = attr
                .get(2)
                .or_else(|| attr.get(3))
                .map(|m| html_unescape(&js_unescape(m.as_str())))
                .unwrap_or_default();

            match attr_name.as_str() {
                "property" | "name" => key = Some(attr_value.to_ascii_lowercase()),
                "content" => content = Some(attr_value),
                _ => {}
            }
        }

        if let (Some(key), Some(content)) = (key, content) {
            if !content.trim().is_empty() {
                values
                    .entry(key)
                    .or_insert_with(|| content.trim().to_string());
            }
        }
    }

    values
}

fn js_var(html: &str, name: &str) -> Option<String> {
    let double = Regex::new(&format!(
        r#"(?s)\b(?:var\s+)?{}\s*=\s*(?:htmlDecode\()?"((?:\\.|[^"\\])*)""#,
        regex::escape(name)
    ))
    .ok()?;
    let single = Regex::new(&format!(
        r#"(?s)\b(?:var\s+)?{}\s*=\s*(?:htmlDecode\()?'((?:\\.|[^'\\])*)'"#,
        regex::escape(name)
    ))
    .ok()?;

    double
        .captures(html)
        .or_else(|| single.captures(html))
        .and_then(|caps| caps.get(1))
        .map(|value| {
            html_unescape(&js_unescape(value.as_str()))
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
}

fn cgi_data_value(html: &str, name: &str) -> Option<String> {
    js_object_after(html, "window.cgiDataNew")
        .and_then(|object| js_object_string_field(object, name))
}

fn js_object_after<'a>(html: &'a str, marker: &str) -> Option<&'a str> {
    let marker_start = html.find(marker)?;
    let after_marker = &html[(marker_start + marker.len())..];
    let brace_offset = after_marker.find('{')?;
    let object_start = marker_start + marker.len() + brace_offset;
    let object = &html[object_start..];

    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;

    for (idx, ch) in object.char_indices() {
        if let Some(quote_char) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_char {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(&object[..=idx]);
                }
            }
            _ => {}
        }
    }

    None
}

fn js_object_string_field(object: &str, name: &str) -> Option<String> {
    let key = regex::escape(name);
    let double = Regex::new(&format!(
        r#"(?s)(?:^|[{{,]\s*)(?:"{key}"|'{key}'|{key})\s*:\s*(?:htmlDecode\()?"((?:\\.|[^"\\])*)""#
    ))
    .ok()?;
    let single = Regex::new(&format!(
        r#"(?s)(?:^|[{{,]\s*)(?:"{key}"|'{key}'|{key})\s*:\s*(?:htmlDecode\()?'((?:\\.|[^'\\])*)'"#
    ))
    .ok()?;

    double
        .captures(object)
        .or_else(|| single.captures(object))
        .and_then(|caps| caps.get(1))
        .map(|value| {
            html_unescape(&js_unescape(value.as_str()))
                .trim()
                .to_string()
        })
        .filter(|value| !value.is_empty())
}

fn html_text_by_id(html: &str, id: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"(?is)<(?P<tag>[a-z0-9]+)[^>]*\bid\s*=\s*(?:"{}"|'{}')[^>]*>(?P<body>.*?)</[a-z0-9]+>"#,
        regex::escape(id),
        regex::escape(id)
    ))
    .ok()?;
    let tags = Regex::new(r"(?is)<[^>]+>").ok()?;

    re.captures(html)
        .and_then(|caps| caps.name("body"))
        .map(|body| tags.replace_all(body.as_str(), "").into_owned())
        .map(|text| html_unescape(&text).trim().to_string())
        .filter(|text| !text.is_empty())
}

fn js_unescape(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('"') => out.push('"'),
            Some('\'') => out.push('\''),
            Some('\\') => out.push('\\'),
            Some('/') => out.push('/'),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                    if let Some(decoded) = char::from_u32(code) {
                        out.push(decoded);
                    }
                }
            }
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Ok(code) = u32::from_str_radix(&hex, 16) {
                    if let Some(decoded) = char::from_u32(code) {
                        out.push(decoded);
                    }
                }
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }

    out
}

fn first_non_empty(values: impl IntoIterator<Item = Option<String>>) -> String {
    values
        .into_iter()
        .flatten()
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
        .unwrap_or_default()
}

fn absolutize_url(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else {
        url.to_string()
    }
}

fn image_filename(url: &str) -> String {
    let ext = url
        .split('?')
        .next()
        .and_then(|path| path.rsplit('.').next())
        .map(|ext| ext.to_ascii_lowercase())
        .filter(|ext| matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "webp" | "gif"))
        .unwrap_or_else(|| "jpg".to_string());

    format!("{:016x}.{ext}", fnv1a64(url.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    png_dimensions(bytes)
        .or_else(|| gif_dimensions(bytes))
        .or_else(|| jpeg_dimensions(bytes))
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }

    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}

fn gif_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 10 || (&bytes[..6] != b"GIF87a" && &bytes[..6] != b"GIF89a") {
        return None;
    }

    Some((
        u16::from_le_bytes(bytes[6..8].try_into().ok()?) as u32,
        u16::from_le_bytes(bytes[8..10].try_into().ok()?) as u32,
    ))
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }

    let mut i = 2usize;
    while i + 4 < bytes.len() {
        while i < bytes.len() && bytes[i] == 0xff {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }

        let marker = bytes[i];
        i += 1;

        if marker == 0xd9 || marker == 0xda {
            return None;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        if i + 2 > bytes.len() {
            return None;
        }

        let len = u16::from_be_bytes(bytes[i..i + 2].try_into().ok()?) as usize;
        if len < 2 || i + len > bytes.len() {
            return None;
        }

        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if len < 7 {
                return None;
            }
            let height = u16::from_be_bytes(bytes[i + 3..i + 5].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes[i + 5..i + 7].try_into().ok()?) as u32;
            return Some((width, height));
        }

        i += len;
    }

    None
}

fn is_squareish(width: u32, height: u32) -> bool {
    if width == 0 || height == 0 {
        return false;
    }

    let ratio = width as f64 / height as f64;
    (0.9..=1.1).contains(&ratio)
}

fn is_false(value: &bool) -> bool {
    !*value
}
