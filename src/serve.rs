//! Local development server with file watching and auto-rebuild.

use anyhow::Result;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub fn serve(output_dir: &Path, port: u16, rebuild: impl Fn() -> anyhow::Result<()> + Send + 'static) -> Result<()> {
    // Watch content/, templates/, static/, config.toml in a background thread
    let (tx, rx) = mpsc::channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                // Only trigger on meaningful events, ignore public/
                if let Some(path) = event.paths.first() {
                    if !path.starts_with("public") && !path.starts_with("target") {
                        let _ = tx.send(Instant::now());
                    }
                }
            }
        },
        notify::Config::default().with_poll_interval(Duration::from_secs(1)),
    )?;

    let watch_dirs = ["content", "templates", "static", "config.toml"];
    for dir in &watch_dirs {
        let path = PathBuf::from(dir);
        if path.exists() {
            let mode = if path.is_dir() { RecursiveMode::Recursive } else { RecursiveMode::NonRecursive };
            watcher.watch(&path, mode)?;
        }
    }

    // Debounce: wait 500ms after last change before rebuilding
    let debounce = Duration::from_millis(500);

    std::thread::spawn(move || {
        let mut last_trigger: Option<Instant> = None;
        loop {
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(instant) => last_trigger = Some(instant),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(t) = last_trigger {
                        if t.elapsed() >= debounce {
                            last_trigger = None;
                            log::info!("File changed, rebuilding...");
                            match rebuild() {
                                Ok(()) => log::info!("Rebuild complete."),
                                Err(e) => log::error!("Rebuild failed: {e:#}"),
                            }
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    // HTTP server
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
    percent_encoding::percent_decode(input)
        .decode_utf8_lossy()
        .into_owned()
}
