// SPDX-License-Identifier: Apache-2.0
//! `war ui` — the web UI (OW-WAR-0116).
//!
//! One page application served from the `war` binary on 127.0.0.1. It is a
//! rendering (SAS §76.6, OW-ADR-0019): every view is read through the
//! functions the CLI answers with, and every act it starts is one the CLI
//! would run — `war sign <target> --ssh-sign`, whose `ssh-add -c` dialog is
//! the human act, or an `auto` remedy, which is never a signing verb. The
//! page holds no key; nothing under `webui/` names `ssh-keygen` or the agent
//! socket, and the battery greps for both.
//!
//! # Controls (loopback; LAN is roadmap OW-PHASE-9/web-lan)
//!
//! - Bound to 127.0.0.1 only. The Host must be exactly the bound address and
//!   an Origin, when present, exactly `http://<host>`.
//! - A 256-bit token per start, printed once in the URL fragment
//!   (`#t=…`, never sent to a server or a Referer) and carried by the page
//!   in memory as `Authorization: Bearer`. Every `/api/` route needs it.
//! - Acts are POST only, need the token AND an Origin equal to the page's,
//!   and name a row id. The server looks the id up in an allowlist it built
//!   itself; the browser never sends an argv (a body with any field but
//!   `id` is refused). One act at a time.
//! - Every response: a CSP with no inline script or style, frame-ancestors
//!   none, nosniff, no-store, Referrer-Policy no-referrer. No CORS.
//! - Request line and headers bounded to 8 KiB, bodies to 4 KiB, reads to
//!   two seconds.
//!
//! # Current by construction
//!
//! Views are cached against the `war watch` fingerprint of the records
//! (plus the roadmap directory). `/api/version` returns it; the page asks
//! every two seconds and refetches when it moves, so a signature given in a
//! terminal appears without a reload.
//!
//! No async (OW-ADR-0014): one accept loop, and an act runs on its own thread
//! so the loop stays responsive while the key's dialog waits for the human.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::repo::{RepoError, Repository};

const INDEX: &str = include_str!("assets/index.html");
const APP_JS: &str = include_str!("assets/app.js");
const APP_CSS: &str = include_str!("assets/app.css");

/// The CSP every response carries. No `unsafe-inline` anywhere: the page's
/// script and style are separate files from this origin.
pub const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

const HEADER_LIMIT: usize = 8192;
const BODY_LIMIT: usize = 4096;

fn err(s: impl std::fmt::Display) -> RepoError {
    RepoError::Message(s.to_string())
}

/// `war ui`. Blocks until interrupted.
pub fn run(
    root: Utf8PathBuf,
    port: u16,
    page: &str,
    actor: Option<String>,
    mode: crate::output::Mode,
) -> Result<u8, RepoError> {
    let repo = Repository::discover(Some(root))?;
    let token = token()?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(err)?;
    listener.set_nonblocking(true).map_err(err)?;
    let host = listener.local_addr().map_err(err)?.to_string();
    let url = format!("http://{host}/#t={token}&p={page}");
    crate::output::emit(
        mode,
        "ui",
        &format!(
            "war ui: {url}\n  this machine only; the token in the link is this session's, and a restart rotates it. Ctrl-C stops."
        ),
        json!({"url": url, "host": host, "loopback_only": true}),
    );
    std::io::stdout().flush().map_err(err)?;
    let state = Server {
        repo,
        host,
        token,
        actor,
        cache: Mutex::new(BTreeMap::new()),
        act: Arc::new(Mutex::new(ActState::Idle)),
    };
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = state.handle(stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(15));
            }
            Err(e) => return Err(err(e)),
        }
    }
}

/// 32 bytes from the OS, hex.
fn token() -> Result<String, RepoError> {
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| err(format!("could not read the OS random source: {e}")))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Constant-time comparison: the token is not leaked a byte at a time.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum ActState {
    Idle,
    Running {
        id: String,
        command: String,
        started_secs_ago: u64,
        #[serde(skip)]
        started: Option<Instant>,
    },
    Done {
        id: String,
        command: String,
        exit: i32,
        output: String,
    },
}

struct Server {
    repo: Repository,
    host: String,
    token: String,
    /// `--as`: who signs, when the register names more than one.
    actor: Option<String>,
    /// view name → (fingerprint, json)
    cache: Mutex<BTreeMap<String, (u64, Value)>>,
    act: Arc<Mutex<ActState>>,
}

/// One parsed request.
struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
            .collect()
    }
}

fn respond(stream: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\
Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n\
X-Frame-Options: DENY\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: {CSP}\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)
}

fn json_response(stream: &mut TcpStream, status: &str, v: &Value) -> std::io::Result<()> {
    respond(
        stream,
        status,
        "application/json",
        &serde_json::to_vec(v).unwrap_or_default(),
    )
}

/// Read one request within the bounds, or say which bound it broke.
fn read_request(stream: &mut TcpStream) -> Result<Request, (&'static str, &'static str)> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if bytes.len() >= HEADER_LIMIT || Instant::now() >= deadline {
            return Err((
                "431 Request Header Fields Too Large",
                "request limit exceeded",
            ));
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(("400 Bad Request", "incomplete request")),
            Ok(n) => bytes.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&bytes[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let first: Vec<&str> = lines.next().unwrap_or("").split_whitespace().collect();
    if first.len() != 3 || !matches!(first[2], "HTTP/1.1" | "HTTP/1.0") {
        return Err(("400 Bad Request", "invalid request line"));
    }
    let mut headers = Vec::new();
    for line in lines {
        let Some((k, v)) = line.split_once(':') else {
            return Err(("400 Bad Request", "invalid header"));
        };
        headers.push((k.trim().to_owned(), v.trim().to_owned()));
    }
    let len: usize = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-length"))
        .map_or(Ok(0), |(_, v)| v.parse())
        .map_err(|_| ("400 Bad Request", "bad content-length"))?;
    if headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("transfer-encoding"))
    {
        return Err(("400 Bad Request", "transfer-encoding is not supported"));
    }
    if len > BODY_LIMIT {
        return Err(("413 Payload Too Large", "body limit exceeded"));
    }
    let mut body = bytes[head_end + 4..].to_vec();
    while body.len() < len {
        if Instant::now() >= deadline {
            return Err(("408 Request Timeout", "body not received in time"));
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return Err(("400 Bad Request", "incomplete body")),
            Ok(n) => body.extend_from_slice(&chunk[..n]),
        }
    }
    body.truncate(len);
    Ok(Request {
        method: first[0].to_owned(),
        path: first[1].to_owned(),
        headers,
        body,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ActBody {
    id: String,
}

impl Server {
    fn handle(&self, mut stream: TcpStream) -> std::io::Result<()> {
        let req = match read_request(&mut stream) {
            Ok(r) => r,
            Err((status, why)) => {
                return respond(&mut stream, status, "text/plain", why.as_bytes());
            }
        };
        // The Host is exactly the bound loopback address; an Origin, when
        // present, is exactly this page's. A DNS-rebinding page names another
        // Host; a cross-site page sends another Origin.
        let origin = format!("http://{}", self.host);
        if req.header("host") != [self.host.as_str()] {
            return respond(&mut stream, "400 Bad Request", "text/plain", b"wrong host");
        }
        let origins = req.header("origin");
        if origins.len() > 1 || origins.iter().any(|o| *o != origin) {
            return respond(
                &mut stream,
                "403 Forbidden",
                "text/plain",
                b"foreign origin",
            );
        }
        let path = req.path.split('?').next().unwrap_or("").to_owned();
        match (req.method.as_str(), path.as_str()) {
            ("GET", "/") => respond(
                &mut stream,
                "200 OK",
                "text/html; charset=utf-8",
                INDEX.as_bytes(),
            ),
            ("GET", "/assets/app.js") => respond(
                &mut stream,
                "200 OK",
                "text/javascript; charset=utf-8",
                APP_JS.as_bytes(),
            ),
            ("GET", "/assets/app.css") => respond(
                &mut stream,
                "200 OK",
                "text/css; charset=utf-8",
                APP_CSS.as_bytes(),
            ),
            (_, p) if p.starts_with("/api/") => {
                let bearer = req
                    .header("authorization")
                    .first()
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(str::to_owned)
                    .unwrap_or_default();
                if !same(&bearer, &self.token) {
                    return respond(
                        &mut stream,
                        "401 Unauthorized",
                        "text/plain",
                        b"session token required",
                    );
                }
                self.api(&mut stream, &req, p)
            }
            ("GET", _) => respond(&mut stream, "404 Not Found", "text/plain", b"unknown route"),
            _ => respond(
                &mut stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
        }
    }

    fn api(&self, stream: &mut TcpStream, req: &Request, path: &str) -> std::io::Result<()> {
        match (req.method.as_str(), path) {
            ("POST", "/api/act") => {
                // An act needs the page's Origin, not merely no foreign one:
                // a request with no Origin is not from the page.
                if req.header("origin").is_empty() {
                    return respond(
                        stream,
                        "403 Forbidden",
                        "text/plain",
                        b"an act needs the page's origin",
                    );
                }
                let body: ActBody = match serde_json::from_slice(&req.body) {
                    Ok(b) => b,
                    Err(_) => {
                        return respond(
                            stream,
                            "400 Bad Request",
                            "text/plain",
                            b"an act names one row id and nothing else",
                        );
                    }
                };
                self.start_act(stream, &body.id)
            }
            (_, "/api/act") if req.method != "GET" => respond(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
            ("GET", "/api/act") => {
                let s = self.act.lock().map(|g| g.clone()).unwrap_or(ActState::Idle);
                let s = match s {
                    ActState::Running {
                        id,
                        command,
                        started,
                        ..
                    } => ActState::Running {
                        id,
                        command,
                        started_secs_ago: started.map_or(0, |t| t.elapsed().as_secs()),
                        started: None,
                    },
                    other => other,
                };
                json_response(
                    stream,
                    "200 OK",
                    &serde_json::to_value(s).unwrap_or(Value::Null),
                )
            }
            ("GET", "/api/version") => json_response(
                stream,
                "200 OK",
                &json!({
                    "version": format!("{:016x}", self.fingerprint()),
                    "program": self.repo.config.project.name,
                    "root": self.repo.root.as_str(),
                    "war": env!("CARGO_PKG_VERSION"),
                }),
            ),
            ("GET", source) if source.starts_with("/api/source/") => {
                let key = &source["/api/source/".len()..];
                let result = Repository::discover(Some(self.repo.root.clone()))
                    .map_err(|e| e.to_string())
                    .and_then(|repo| crate::progress_viewer::live_source(&repo, key));
                match result {
                    Ok(Some(bytes)) => {
                        respond(stream, "200 OK", "text/plain; charset=utf-8", &bytes)
                    }
                    Ok(None) => respond(stream, "404 Not Found", "text/plain", b"unknown source"),
                    Err(_) => respond(
                        stream,
                        "404 Not Found",
                        "text/plain",
                        b"source unavailable, replaced, or too large",
                    ),
                }
            }
            ("GET", view) => {
                let name = view.trim_start_matches("/api/");
                match self.view(name) {
                    Ok(Some(v)) => json_response(stream, "200 OK", &v),
                    Ok(None) => respond(stream, "404 Not Found", "text/plain", b"unknown view"),
                    Err(e) => json_response(
                        stream,
                        "500 Internal Server Error",
                        &json!({"error": e.to_string()}),
                    ),
                }
            }
            _ => respond(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
        }
    }

    /// The records' fingerprint: `war watch`'s trees plus the roadmap.
    fn fingerprint(&self) -> u64 {
        let mut dirs = crate::watch::watched_dirs(&self.repo);
        dirs.push(crate::roadmap_cmd::dir(&self.repo));
        // The configuration too: a broken `openwarrant.toml` must be seen.
        let config = std::fs::metadata(self.repo.root.join(crate::init::CONFIG_FILE))
            .ok()
            .and_then(|m| Some((m.len(), m.modified().ok()?)))
            .map_or(0, |(len, t)| {
                len ^ t
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos() as u64)
            });
        crate::watch::fingerprint(&dirs).rotate_left(1) ^ config
    }

    /// A view, cached until the fingerprint moves.
    fn view(&self, name: &str) -> Result<Option<Value>, RepoError> {
        if !VIEWS.contains(&name) {
            return Ok(None);
        }
        let fp = self.fingerprint();
        if let Ok(c) = self.cache.lock()
            && let Some((f, v)) = c.get(name)
            && *f == fp
        {
            return Ok(Some(v.clone()));
        }
        // The repository is re-read: records changed since the server began.
        // A refresh that fails keeps the last good view and says why beside
        // it — a page that went blank on a bad edit would hide the very
        // state the reader was watching.
        let built = Repository::discover(Some(self.repo.root.clone()))
            .and_then(|repo| build_view(&repo, name, self.actor.as_deref()));
        match built {
            Ok(mut v) => {
                if let Some(o) = v.as_object_mut() {
                    o.insert("error".to_owned(), Value::Null);
                }
                if let Ok(mut c) = self.cache.lock() {
                    c.insert(name.to_owned(), (fp, v.clone()));
                }
                Ok(Some(v))
            }
            Err(e) => match self.cache.lock().ok().and_then(|c| c.get(name).cloned()) {
                Some((_, mut last)) => {
                    if let Some(o) = last.as_object_mut() {
                        o.insert("error".to_owned(), json!(e.to_string()));
                    }
                    Ok(Some(last))
                }
                None => Err(e),
            },
        }
    }

    /// Start one allowlisted act on its own thread.
    fn start_act(&self, stream: &mut TcpStream, id: &str) -> std::io::Result<()> {
        let allow = match self.allowlist() {
            Ok(a) => a,
            Err(e) => {
                return json_response(
                    stream,
                    "500 Internal Server Error",
                    &json!({"error": e.to_string()}),
                );
            }
        };
        let Some(argv) = allow.get(id).cloned() else {
            return respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"not an act this page may start",
            );
        };
        {
            let mut g = match self.act.lock() {
                Ok(g) => g,
                Err(_) => {
                    return respond(
                        stream,
                        "500 Internal Server Error",
                        "text/plain",
                        b"act state poisoned",
                    );
                }
            };
            if matches!(*g, ActState::Running { .. }) {
                return respond(
                    stream,
                    "409 Conflict",
                    "text/plain",
                    b"an act is already running",
                );
            }
            *g = ActState::Running {
                id: id.to_owned(),
                command: format!("war {}", argv.join(" ")),
                started_secs_ago: 0,
                started: Some(Instant::now()),
            };
        }
        eprintln!(
            "war ui: act {id}: war --root {} {}",
            self.repo.root,
            argv.join(" ")
        );
        let state = Arc::clone(&self.act);
        let root = self.repo.root.clone();
        let id = id.to_owned();
        std::thread::spawn(move || {
            let command = format!("war {}", argv.join(" "));
            // A signing act is dry-run first with its exact flags; anything
            // but a would-record verdict stops it before a key is asked.
            if argv.first().is_some_and(|a| a == "sign") {
                let dry: Vec<String> = argv
                    .iter()
                    .map(|a| {
                        if a == "--ssh-sign" {
                            "--dry-run".to_owned()
                        } else {
                            a.clone()
                        }
                    })
                    .collect();
                let text = std::env::current_exe()
                    .and_then(|exe| {
                        std::process::Command::new(exe)
                            .arg("--root")
                            .arg(root.as_str())
                            .args(&dry)
                            .current_dir(root.as_str())
                            .stdin(std::process::Stdio::null())
                            .output()
                    })
                    .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                    .unwrap_or_default();
                let want = if argv.iter().any(|a| a.starts_with("--batch")) {
                    "batch.would-record"
                } else {
                    "sign.would-record"
                };
                if !text.contains(want) {
                    if let Ok(mut g) = state.lock() {
                        *g = ActState::Done {
                            id,
                            command,
                            exit: 2,
                            output: format!(
                                "stopped before signing: the dry run of this exact act does not say it would record.\n\n{text}"
                            ),
                        };
                    }
                    return;
                }
            }
            let out = std::env::current_exe().and_then(|exe| {
                std::process::Command::new(exe)
                    .arg("--root")
                    .arg(root.as_str())
                    .args(&argv)
                    .current_dir(root.as_str())
                    .stdin(std::process::Stdio::null())
                    .output()
            });
            let done = match out {
                Ok(o) => ActState::Done {
                    id,
                    command,
                    exit: o.status.code().unwrap_or(-1),
                    output: format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    ),
                },
                Err(e) => ActState::Done {
                    id,
                    command,
                    exit: -1,
                    output: e.to_string(),
                },
            };
            if let Ok(mut g) = state.lock() {
                *g = done;
            }
        });
        json_response(stream, "202 Accepted", &json!({"started": true}))
    }

    /// `--as <actor>` after `sign <target>`, when the session names a signer.
    fn with_actor(&self, mut argv: Vec<String>) -> Vec<String> {
        if let Some(a) = &self.actor {
            argv.insert(2, "--as".to_owned());
            argv.insert(3, a.clone());
        }
        argv
    }

    /// Every act the page may start right now: id → argv (after `--root`).
    /// Built by the server from the queue's dry-run verdicts and the remedy
    /// table — never from anything the browser sent.
    fn allowlist(&self) -> Result<BTreeMap<String, Vec<String>>, RepoError> {
        let mut out = BTreeMap::new();
        if let Some(q) = self.view("queue")? {
            for a in q["acts"].as_array().into_iter().flatten() {
                if let (Some(id), Some(target)) = (a["act_id"].as_str(), a["target"].as_str()) {
                    out.insert(
                        id.to_owned(),
                        self.with_actor(vec![
                            "sign".to_owned(),
                            target.to_owned(),
                            "--ssh-sign".to_owned(),
                        ]),
                    );
                }
                for c in a["choices"].as_array().into_iter().flatten() {
                    if let (Some(id), Some(target), Some(flag), Some(value)) = (
                        c["act_id"].as_str(),
                        a["target"].as_str(),
                        c["flag"].as_str(),
                        c["value"].as_str(),
                    ) {
                        out.insert(
                            id.to_owned(),
                            self.with_actor(vec![
                                "sign".to_owned(),
                                target.to_owned(),
                                flag.to_owned(),
                                value.to_owned(),
                                "--ssh-sign".to_owned(),
                            ]),
                        );
                    }
                }
            }
            if let (Some(id), Some(targets)) = (
                q["batch"]["act_id"].as_str(),
                q["batch"]["targets"].as_array(),
            ) {
                let targets: Vec<&str> = targets.iter().filter_map(Value::as_str).collect();
                out.insert(
                    id.to_owned(),
                    self.with_actor(vec![
                        "sign".to_owned(),
                        // One argument, so `with_actor`'s `--as` cannot land
                        // between the flag and its list.
                        format!("--batch={}", targets.join(",")),
                        "--ssh-sign".to_owned(),
                    ]),
                );
            }
        }
        if let Some(h) = self.view("help")? {
            for r in h["remedies"].as_array().into_iter().flatten() {
                if let (Some(id), Some(argv)) = (r["act_id"].as_str(), r["argv"].as_array()) {
                    let argv: Vec<String> = argv
                        .iter()
                        .filter_map(|x| x.as_str().map(str::to_owned))
                        .collect();
                    // `war …` only, and the `war` itself dropped: the child is
                    // this binary. A remedy that is not a war command is shown,
                    // never run.
                    if argv.first().is_some_and(|w| w == "war") {
                        out.insert(id.to_owned(), argv[1..].to_vec());
                    }
                }
            }
        }
        Ok(out)
    }
}

const VIEWS: &[&str] = &[
    "snapshot",
    "progress",
    "queue",
    "questions",
    "frontier",
    "corpus",
    "help",
];

/// A short stable id for an allowlisted act.
fn act_id(kind: &str, key: &str) -> String {
    format!(
        "{kind}-{}",
        &openwarrant_compiler::sha256_hex(key.as_bytes())[..16]
    )
}

fn build_view(repo: &Repository, name: &str, actor: Option<&str>) -> Result<Value, RepoError> {
    Ok(match name {
        "progress" => progress(repo)?,
        "queue" => queue(repo, actor)?,
        "questions" => {
            let b = crate::console::board(repo)?;
            json!({"questions": b.questions})
        }
        "frontier" => {
            let (_, f) = crate::frontier::run(repo, None)?;
            serde_json::to_value(f).map_err(err)?
        }
        "corpus" => {
            let s = crate::status::build(repo)?;
            let rows: Vec<Value> = s
                .warrants
                .iter()
                .map(|w| {
                    json!({
                        "alias": w.alias, "title": w.title, "rung": w.rung,
                        "unmet": w.unmet, "unestablished": w.unestablished,
                        "command": format!("war resolve {} --dry-run", w.alias),
                    })
                })
                .collect();
            json!({"warrants": rows})
        }
        "help" => help(repo)?,
        // `war progress --snapshot`: attributed work reports, their links,
        // the frontier — the viewer's own validated snapshot.
        "snapshot" => json!({"snapshot": crate::progress_viewer::json_snapshot(repo)?}),
        _ => Value::Null,
    })
}

/// The Progress page: the canonical roadmap (OW-ADR-0023), each phase with
/// its members' rungs and unmet counts, the open work, and the unassigned.
fn progress(repo: &Repository) -> Result<Value, RepoError> {
    let status = crate::status::build(repo)?;
    let by_alias: BTreeMap<&str, &openwarrant_core::status::WarrantStatus> = status
        .warrants
        .iter()
        .map(|w| (w.alias.as_str(), w))
        .collect();
    let member = |a: &str| -> Value {
        match by_alias.get(a) {
            Some(w) => json!({
                "alias": a, "title": w.title, "rung": w.rung,
                "unmet": w.unmet.len(), "command": format!("war resolve {a} --dry-run"),
            }),
            None => json!({"alias": a}),
        }
    };
    let unassigned: Vec<Value> = status
        .objectives
        .iter()
        .filter(|o| o.roadmap_ref.is_none())
        .flat_map(|o| o.warrants.iter().map(|a| member(a)))
        .collect();
    let (roadmap, phases) = match crate::roadmap_cmd::view_with(repo, &status) {
        Ok((_, v)) => {
            let phases: Vec<Value> = v
                .phases
                .iter()
                .map(|p| {
                    json!({
                        "id": p.id, "title": p.title, "tier": p.tier, "exit": p.exit,
                        "depends_on": p.depends_on, "achieved": p.achieved,
                        "exit_warrant": p.exit_warrant, "open": p.open,
                        "members": p.members.iter().map(|a| member(a)).collect::<Vec<_>>(),
                    })
                })
                .collect();
            (
                json!({"program": v.program, "accepted": v.accepted,
                       "accepted_revision": v.accepted_revision, "pending_revision": v.pending_revision}),
                phases,
            )
        }
        Err(e) => (json!({"error": e.to_string()}), vec![]),
    };
    let ladder = status.warrant_ladder();
    Ok(json!({
        "roadmap": roadmap,
        "phases": phases,
        "unassigned": unassigned,
        "ladder": ladder,
    }))
}

/// The Queue: every pending act with its dry-run verdict; an act id only for
/// one whose verdict is would-record and that needs no decision.
fn queue(repo: &Repository, actor: Option<&str>) -> Result<Value, RepoError> {
    let board = crate::console::board(repo)?;
    let opts = crate::sign::Options {
        all: true,
        dry_run: true,
        actor: actor.map(str::to_owned),
        ..crate::sign::Options::default()
    };
    let judged = crate::sign::run(repo, None, &opts)?;
    // More than one eligible signer and no `--as`: every act would be
    // refused on `sign.who`. Say so once, with the command that fixes it,
    // rather than offering buttons that would all fail.
    let who: Option<String> = judged
        .diagnostics
        .iter()
        .find(|d| d.rule == "sign.who")
        .map(|d| d.message.clone());
    let acts: Vec<Value> = board
        .acts
        .iter()
        .map(|a| {
            let record = judged
                .diagnostics
                .iter()
                .any(|d| d.rule == "sign.would-record" && d.message.contains(a.line.trim()));
            let against: Vec<&crate::diagnostic::Diagnostic> = judged
                .diagnostics
                .iter()
                .filter(|d| d.file.as_deref() == Some(a.line.as_str()))
                .collect();
            let decision = against.iter().any(|d| d.rule == "sign.needs-decision");
            let verdict = if decision {
                "needs a decision"
            } else if record {
                "would record"
            } else {
                "would be refused"
            };
            let reasons: Vec<String> = against
                .iter()
                .map(|d| format!("{}: {}", d.rule, d.message))
                .collect();
            let mut v = json!({
                "n": a.n, "act": a.act, "target": a.target, "line": a.line,
                "command": format!("war sign {} --ssh-sign", a.target),
                "verdict": verdict, "reasons": reasons,
            });
            if record && !decision {
                v["act_id"] = json!(act_id("sign", &a.target));
            }
            // A decision the dry run names — `--outcome a|b|c`, `--kind a|b` —
            // becomes one choice per permitted value. Choosing is the human's
            // act on this page; the key's dialog is still the signature, and
            // the server dry-runs the exact choice before running it.
            if decision {
                let choices: Vec<Value> = against
                    .iter()
                    .filter_map(|d| decision_flag(&d.message))
                    .flat_map(|(flag, values)| {
                        values.into_iter().map(move |value| (flag.clone(), value))
                    })
                    .map(|(flag, value)| {
                        json!({
                            "label": format!("{} as {value}", a.act),
                            "flag": flag, "value": value,
                            "act_id": act_id("sign", &format!("{} {flag} {value}", a.target)),
                            "command": format!("war sign {} {flag} {value} --ssh-sign", a.target),
                        })
                    })
                    .collect();
                if let Some(first) = choices.first() {
                    v["command"] = first["command"].clone();
                }
                v["choices"] = json!(choices);
            }
            v
        })
        .collect();
    // Two or more acts that would record and need no decision: one batch
    // (OW-WAR-0072), one dialog. The targets are named, not "all", so what
    // the button signs is what the page showed.
    let batchable: Vec<String> = acts
        .iter()
        .filter(|a| a.get("act_id").is_some())
        .filter_map(|a| a["target"].as_str().map(str::to_owned))
        .collect();
    let batch = (batchable.len() >= 2).then(|| {
        json!({
            "act_id": act_id("batch", &batchable.join(",")),
            "targets": batchable,
            "command": format!("war sign --batch {} --ssh-sign", batchable.join(",")),
        })
    });
    Ok(json!({
        "acts": acts,
        "batch": batch,
        "signer": actor,
        "who": who.map(|m| json!({
            "why": m,
            "command": "war ui --as <your name as roles.toml spells it>",
        })),
    }))
}

/// `pass --outcome a|b|c` or `needs --kind a|b` in a needs-decision message →
/// the flag and its permitted values.
fn decision_flag(message: &str) -> Option<(String, Vec<String>)> {
    let at = message.find("--")?;
    let mut words = message[at..].split_whitespace();
    let flag = words.next()?.to_owned();
    if !matches!(flag.as_str(), "--outcome" | "--kind") {
        return None;
    }
    let values: Vec<String> = words
        .next()?
        .trim_end_matches(|c: char| !c.is_ascii_alphanumeric())
        .split('|')
        .map(str::to_owned)
        .filter(|v| {
            !v.is_empty()
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .collect();
    (!values.is_empty()).then_some((flag, values))
}

/// Help: `war next`'s actions, then `war check`'s errors by rule with their
/// remedies; an `auto` remedy carries an act id.
fn help(repo: &Repository) -> Result<Value, RepoError> {
    let next = crate::next::run(repo)?;
    let check = crate::check::run(repo, None, false)?;
    let mut by_rule: BTreeMap<String, (usize, crate::diagnostic::Diagnostic)> = BTreeMap::new();
    for d in check
        .diagnostics
        .iter()
        .filter(|d| d.severity == crate::diagnostic::Severity::Error)
    {
        by_rule
            .entry(d.rule.clone())
            .and_modify(|e| e.0 += 1)
            .or_insert((1, d.clone()));
    }
    let mut remedies = Vec::new();
    for (rule, (n, d)) in &by_rule {
        let r = crate::remedy::remedy_for(d);
        let mut v = json!({
            "rule": rule, "count": n, "example": d.message,
            "kind": r.as_ref().map(|r| r.kind.label()),
            "argv": r.as_ref().map(|r| r.argv.clone()),
            "command": r.as_ref().map(crate::remedy::Remedy::command),
            "purpose": r.as_ref().map(|r| r.purpose.clone()),
        });
        if let Some(r) = &r
            && r.kind == crate::remedy::Kind::Auto
        {
            v["act_id"] = json!(act_id("remedy", &r.command()));
        }
        remedies.push(v);
    }
    Ok(json!({
        "next": next.actions,
        "nothing": next.nothing,
        "remedies": remedies,
        "counts": {
            "error": check.count(crate::diagnostic::Severity::Error),
            "warn": check.count(crate::diagnostic::Severity::Warn),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_csp_allows_no_inline_code() {
        assert!(!CSP.contains("unsafe-inline"));
        assert!(!CSP.contains("unsafe-eval"));
        assert!(CSP.contains("frame-ancestors 'none'"));
        assert!(CSP.contains("script-src 'self'"));
    }

    /// ` on<letters>=` anywhere in the markup: an inline event handler.
    fn has_inline_handler(html: &str) -> bool {
        html.match_indices(" on").any(|(i, _)| {
            let rest = &html[i + 3..];
            let word: String = rest.chars().take_while(char::is_ascii_lowercase).collect();
            !word.is_empty() && rest[word.len()..].starts_with('=')
        })
    }

    #[test]
    fn the_handler_scan_finds_a_handler_and_ignores_prose() {
        assert!(has_inline_handler(r#"<a onclick="x()">"#));
        assert!(!has_inline_handler("<p>done once, one only</p>"));
    }

    #[test]
    fn the_page_has_no_inline_script_or_style() {
        assert!(!INDEX.contains("<script>"), "inline script in index.html");
        assert!(!INDEX.contains("<style"), "inline style in index.html");
        assert!(
            !INDEX.contains(" style="),
            "inline style attribute in index.html"
        );
        assert!(
            !has_inline_handler(INDEX),
            "inline event handler in index.html"
        );
        assert!(INDEX.contains(r#"<script src="/assets/app.js""#));
    }

    #[test]
    fn tokens_are_long_random_and_compared_in_constant_time() {
        let a = token().unwrap();
        let b = token().unwrap();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(same(&a, &a.clone()));
        assert!(!same(&a, &b));
        assert!(!same(&a, &a[..63]));
    }

    #[test]
    fn a_decision_names_its_flag_and_values() {
        assert_eq!(
            decision_flag(
                "OW-WAR-0001: §38.6 does not permit `satisfied`; pass --outcome not_satisfied|cancelled|blocked — not judged"
            ),
            Some((
                "--outcome".into(),
                vec!["not_satisfied".into(), "cancelled".into(), "blocked".into()]
            ))
        );
        assert_eq!(
            decision_flag("a correction needs --kind behaviour-change|added-refusal — not judged"),
            Some((
                "--kind".into(),
                vec!["behaviour-change".into(), "added-refusal".into()]
            ))
        );
        assert_eq!(decision_flag("run `--root x; rm -rf /`"), None);
        assert_eq!(decision_flag("nothing here"), None);
    }

    #[test]
    fn an_act_body_names_an_id_and_nothing_else() {
        assert!(serde_json::from_str::<ActBody>(r#"{"id":"sign-0"}"#).is_ok());
        assert!(serde_json::from_str::<ActBody>(r#"{"id":"x","argv":["rm"]}"#).is_err());
        assert!(serde_json::from_str::<ActBody>(r#"{"argv":["sign"]}"#).is_err());
    }
}
