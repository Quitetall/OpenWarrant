// SPDX-License-Identifier: Apache-2.0
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

fn response(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}
fn handle(
    mut stream: TcpStream,
    host: &str,
    snapshot: &Snapshot,
    error: &Option<String>,
    interval: u64,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(1)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    let mut chunk = [0; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
        if bytes.len() >= 8192 || Instant::now() >= deadline {
            return response(
                &mut stream,
                "431 Request Header Fields Too Large",
                "text/plain",
                b"Request limit exceeded",
            );
        }
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Ok(());
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    if bytes.len() > 8192 {
        return response(
            &mut stream,
            "431 Request Header Fields Too Large",
            "text/plain",
            b"Request limit exceeded",
        );
    }
    let request = match std::str::from_utf8(&bytes) {
        Ok(s) => s,
        Err(_) => {
            return response(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Invalid request",
            );
        }
    };
    let mut lines = request.split("\r\n");
    let first = lines.next().unwrap_or("");
    let parts: Vec<_> = first.split_whitespace().collect();
    if parts.len() != 3 || !matches!(parts[2], "HTTP/1.1" | "HTTP/1.0") {
        return response(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            b"Invalid request line",
        );
    }
    let mut hosts = Vec::new();
    let mut origins = Vec::new();
    for line in lines.take_while(|s| !s.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            return response(
                &mut stream,
                "400 Bad Request",
                "text/plain",
                b"Invalid header",
            );
        };
        if name.eq_ignore_ascii_case("host") {
            hosts.push(value.trim());
        }
        if name.eq_ignore_ascii_case("origin") {
            origins.push(value.trim());
        }
    }
    let origin = format!("http://{host}");
    if hosts != [host] || origins.len() > 1 || origins.iter().any(|v| *v != origin) {
        return response(
            &mut stream,
            "403 Forbidden",
            "text/plain",
            b"Local origin required",
        );
    }
    if parts[0] != "GET" {
        return response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            b"Read-only viewer",
        );
    }
    match parts[1] {
        "/" => match html(snapshot, true, interval) {
            Ok(body) => response(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                body.as_bytes(),
            ),
            Err(_) => response(
                &mut stream,
                "500 Internal Server Error",
                "text/plain",
                b"Render failed",
            ),
        },
        "/api/progress" => {
            let body = serde_json::to_vec(&serde_json::json!({"snapshot":snapshot,"error":error}))
                .unwrap();
            response(&mut stream, "200 OK", "application/json", &body)
        }
        path if path.starts_with("/source/") => {
            let Some(source) = snapshot.sources.get(&path[8..]) else {
                return response(
                    &mut stream,
                    "404 Not Found",
                    "text/plain",
                    b"Unknown source",
                );
            };
            // A cached canonical filename must not be followed through a replacement symlink.
            if source.canonicalize().ok().as_ref() != Some(source) {
                return response(
                    &mut stream,
                    "403 Forbidden",
                    "text/plain",
                    b"Source identity changed",
                );
            }
            match source
                .strip_prefix(&snapshot.root)
                .map_err(|e| e.to_string())
                .and_then(|name| super::source::read(&snapshot.root, name, SOURCE_LIMIT))
            {
                Ok(body) => response(&mut stream, "200 OK", "text/plain; charset=utf-8", &body),
                Err(_) => response(
                    &mut stream,
                    "404 Not Found",
                    "text/plain",
                    b"Source unavailable or too large",
                ),
            }
        }
        _ => response(&mut stream, "404 Not Found", "text/plain", b"Unknown route"),
    }
}
pub(super) fn serve(
    repo: Repository,
    port: u16,
    interval: u64,
    mode: crate::output::Mode,
) -> Result<(), RepoError> {
    let mut snapshot = capture(&repo, true)?;
    let listener =
        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(|e| err(e.to_string()))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| err(e.to_string()))?;
    let host = listener
        .local_addr()
        .map_err(|e| err(e.to_string()))?
        .to_string();
    crate::output::emit(
        mode,
        "overview",
        &format!("Progress: http://{host}/ (read-only; refresh {interval}s; Ctrl-C stops)"),
        serde_json::json!({"url":format!("http://{host}/"),"read_only":true,"refresh_secs":interval}),
    );
    std::io::stdout().flush().map_err(|e| err(e.to_string()))?;
    let mut refreshed = Instant::now();
    let mut error = None;
    loop {
        if refreshed.elapsed() >= Duration::from_secs(interval) {
            match Repository::discover(Some(repo.root.clone()))
                .and_then(|current| capture(&current, true))
            {
                Ok(new) => {
                    snapshot = new;
                    error = None
                }
                Err(e) => error = Some(e.to_string()),
            };
            refreshed = Instant::now();
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = handle(stream, &host, &snapshot, &error, interval);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(e) => return Err(err(e.to_string())),
        }
    }
}
