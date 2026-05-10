//! Local development server using `tiny_http`.
//!
//! Serves the generated `public/` directory with proper MIME types
//! and percent-decoding for CJK characters in URLs.

use anyhow::Result;
use std::path::Path;

pub fn serve(output_dir: &Path, port: u16) -> Result<()> {
    let addr = format!("0.0.0.0:{port}");
    let server = tiny_http::Server::http(addr)
        .map_err(|e| anyhow::anyhow!("Failed to start server: {e}"))?;
    log::info!("Serving at http://localhost:{port}");

    for request in server.incoming_requests() {
        let url = percent_decode(request.url().as_bytes());

        let file_path = if url.ends_with('/') {
            output_dir.join(url.trim_start_matches('/')).join("index.html")
        } else {
            output_dir.join(url.trim_start_matches('/'))
        };

        if file_path.exists() && file_path.is_file() {
            let content_type = guess_content_type(&file_path);
            let response = tiny_http::Response::from_file(std::fs::File::open(&file_path)?)
                .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap());
            let _ = request.respond(response);
        } else {
            let index_path = file_path.join("index.html");
            if index_path.exists() {
                let response = tiny_http::Response::from_file(std::fs::File::open(&index_path)?)
                    .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], b"text/html; charset=utf-8").unwrap());
                let _ = request.respond(response);
            } else {
                let response = tiny_http::Response::from_string("404 Not Found")
                    .with_status_code(404);
                let _ = request.respond(response);
            }
        }
    }

    Ok(())
}

fn guess_content_type(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        Some("mp4") => "video/mp4",
        Some("ico") => "image/x-icon",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }.to_string()
}

fn percent_decode(input: &[u8]) -> String {
    let decoded = percent_encoding::percent_decode(input)
        .decode_utf8_lossy();
    decoded.into_owned()
}
