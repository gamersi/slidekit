//! `serve`: watch + a small static HTTP server for the output dir with live reload.
//!
//! Std-only HTTP/1.1 (one thread per connection, `Connection: close`), with range requests
//! so videos can seek, and a server-sent-events endpoint that tells players to reload.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::build::Options;
use crate::config::Config;
use crate::player;

pub const EVENTS_PATH: &str = "/__slidekit/events";

/// Connected live-reload clients.
#[derive(Default)]
struct Clients(Mutex<Vec<mpsc::Sender<()>>>);

impl Clients {
    fn reload_all(&self) {
        self.0.lock().unwrap().retain(|tx| tx.send(()).is_ok());
    }
}

pub fn run(dir: &Path, port: u16, open: bool, options: Options, quiet: bool) -> Result<()> {
    let config = Config::load(dir)?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("cannot listen on port {port}"))?;
    let index = config
        .html
        .as_ref()
        .and_then(|p| p.strip_prefix(&config.out_dir).ok())
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let url = format!("http://127.0.0.1:{port}/{index}");
    eprintln!("serving {} at {url}", config.out_dir.display());

    let clients = Arc::new(Clients::default());
    let root = config.out_dir.clone();
    let server_clients = clients.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (root, clients, index) = (root.clone(), server_clients.clone(), index.clone());
            std::thread::spawn(move || {
                let _ = handle(stream, &root, &index, &clients);
            });
        }
    });

    let mut first = true;
    crate::watch::run(dir, options, quiet, move |_| {
        if first && open {
            open_browser(&url);
        }
        first = false;
        clients.reload_all();
    })
}

fn open_browser(url: &str) {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    if let Err(err) = std::process::Command::new(cmd).arg(url).spawn() {
        eprintln!("cannot open the browser ({cmd}): {err}");
    }
}

struct Request {
    method: String,
    path: String,
    range: Option<String>,
}

fn read_request(stream: &TcpStream) -> std::io::Result<Request> {
    let mut reader = BufReader::new(stream.take(16 * 1024));
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let path = parts.next().unwrap_or("/").to_owned();
    let mut range = None;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 || line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':')
            && name.eq_ignore_ascii_case("range")
        {
            range = Some(value.trim().to_owned());
        }
    }
    Ok(Request {
        method,
        path,
        range,
    })
}

fn handle(
    mut stream: TcpStream,
    root: &Path,
    index: &str,
    clients: &Clients,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let req = read_request(&stream)?;
    let path = req.path.split(['?', '#']).next().unwrap_or("/");
    if req.method != "GET" && req.method != "HEAD" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"method not allowed",
            &req,
        );
    }
    if path == EVENTS_PATH {
        return events(stream, clients);
    }
    let Some(file) = resolve(root, path, index) else {
        return respond(
            &mut stream,
            "404 Not Found",
            "text/plain",
            b"not found",
            &req,
        );
    };
    let mime = mime(&file);
    if mime == "text/html" {
        // Enable live reload in the player.
        let html = std::fs::read_to_string(&file)?.replacen(
            player::LIVE_RELOAD_OFF,
            player::LIVE_RELOAD_ON,
            1,
        );
        return respond(&mut stream, "200 OK", mime, html.as_bytes(), &req);
    }

    let mut f = std::fs::File::open(&file)?;
    let len = f.metadata()?.len();
    let (status, start, end) = match req.range.as_deref().and_then(|r| parse_range(r, len)) {
        Some((start, end)) => ("206 Partial Content", start, end),
        None if req.range.is_some() && len > 0 => {
            let head = format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{len}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            return stream.write_all(head.as_bytes());
        }
        None => ("200 OK", 0, len.saturating_sub(1)),
    };
    let count = if len == 0 { 0 } else { end - start + 1 };
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {count}\r\nAccept-Ranges: bytes\r\nCache-Control: no-cache\r\nConnection: close\r\n"
    );
    if status.starts_with("206") {
        head.push_str(&format!("Content-Range: bytes {start}-{end}/{len}\r\n"));
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    if req.method == "GET" {
        f.seek(SeekFrom::Start(start))?;
        std::io::copy(&mut f.take(count), &mut stream)?;
    }
    Ok(())
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    mime: &str,
    body: &[u8],
    req: &Request,
) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    if req.method == "GET" {
        stream.write_all(body)?;
    }
    Ok(())
}

/// Server-sent events: `reload` after each successful build, comments as keep-alive.
fn events(mut stream: TcpStream, clients: &Clients) -> std::io::Result<()> {
    let (tx, rx) = mpsc::channel();
    clients.0.lock().unwrap().push(tx);
    stream.write_all(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\nretry: 1000\n\n",
    )?;
    loop {
        let msg: &[u8] = match rx.recv_timeout(Duration::from_secs(15)) {
            Ok(()) => b"event: reload\ndata: 1\n\n",
            Err(RecvTimeoutError::Timeout) => b": ping\n\n",
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
        };
        stream.write_all(msg)?;
        stream.flush()?;
    }
}

/// Maps a URL path to a file inside `root`; refuses anything that could escape it.
fn resolve(root: &Path, url_path: &str, index: &str) -> Option<PathBuf> {
    let decoded = percent_decode(url_path)?;
    let rel = Path::new(decoded.trim_start_matches('/'));
    if !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    let mut path = root.join(rel);
    if path.is_dir() {
        path = if rel.as_os_str().is_empty() && !index.is_empty() {
            root.join(index)
        } else {
            path.join("index.html")
        };
    }
    path.is_file().then_some(path)
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = std::str::from_utf8(bytes.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Parses `bytes=start-end`, `bytes=start-` and `bytes=-suffix` (single ranges only).
fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?;
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (start, end) = match (a.trim(), b.trim()) {
        ("", suffix) => {
            let n: u64 = suffix.parse().ok()?;
            (len.saturating_sub(n), len - 1)
        }
        (a, "") => (a.parse().ok()?, len - 1),
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(len - 1)),
    };
    (start <= end && start < len).then_some((start, end))
}

fn mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => "text/html",
        Some("js" | "mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("pdf") => "application/pdf",
        Some("mp4" | "m4v") => "video/mp4",
        Some("webm") => "video/webm",
        Some("ogv") => "video/ogg",
        Some("mov") => "video/quicktime",
        Some("mp3") => "audio/mpeg",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-99", 1000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=0-5000", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=5-1", 1000), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 1000), None);
    }

    #[test]
    fn paths_stay_inside_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), "x").unwrap();
        std::fs::write(dir.path().join("a b.txt"), "x").unwrap();
        assert_eq!(
            resolve(dir.path(), "/", "index.html"),
            Some(dir.path().join("index.html"))
        );
        assert_eq!(
            resolve(dir.path(), "/a%20b.txt", ""),
            Some(dir.path().join("a b.txt"))
        );
        assert_eq!(resolve(dir.path(), "/../etc/passwd", ""), None);
        assert_eq!(resolve(dir.path(), "/%2e%2e/x", ""), None);
        assert_eq!(resolve(dir.path(), "/missing", ""), None);
    }
}
