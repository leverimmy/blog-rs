//! Shared utility functions.

/// Escape HTML special characters (`&`, `<`, `>`, `"`).
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Decode common HTML entities and numeric character references.
pub fn html_unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;

    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];

        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };

        let entity = &rest[1..end];
        match decode_entity(entity) {
            Some(decoded) => out.push_str(&decoded),
            None => out.push_str(&rest[..=end]),
        }
        rest = &rest[(end + 1)..];
    }

    out.push_str(rest);
    out
}

fn decode_entity(entity: &str) -> Option<String> {
    match entity {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" | "#39" => Some("'".to_string()),
        "nbsp" => Some(" ".to_string()),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
            u32::from_str_radix(&entity[2..], 16)
                .ok()
                .and_then(char::from_u32)
                .map(|ch| ch.to_string())
        }
        _ if entity.starts_with('#') => entity[1..]
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(|ch| ch.to_string()),
        _ => None,
    }
}
