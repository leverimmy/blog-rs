//! Tera template engine initialization with custom filters.

use anyhow::Result;
use std::path::Path;
use tera::Tera;

pub fn init(templates_dir: &Path) -> Result<Tera> {
    let pattern = templates_dir.join("**/*.html");
    let pattern_str = pattern.to_string_lossy().to_string();

    let mut tera = Tera::new(&pattern_str)?;
    tera.register_filter("date_format", date_format_filter);
    Ok(tera)
}

fn date_format_filter(
    value: &tera::Value,
    args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let date_str = value.as_str().unwrap_or("");
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d");

    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S") {
        Ok(tera::Value::String(dt.format(format).to_string()))
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(tera::Value::String(date.format(format).to_string()))
    } else {
        Ok(value.clone())
    }
}
