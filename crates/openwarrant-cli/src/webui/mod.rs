// SPDX-License-Identifier: Apache-2.0
//! `war view ui` — the web UI (OW-WAR-0116).
//!
//! One page application served from the `war` binary on 127.0.0.1. It is a
//! rendering (SAS §76.6, OW-ADR-0019): every view is read through the
//! functions the CLI answers with, and every act it starts is one the CLI
//! would run — `war sign <target> --ssh-sign`, whose `ssh-add -c` dialog is
//! the human act, or an `auto` remedy, which is never a signing verb. The
//! page holds no key; nothing under `webui/` names `ssh-keygen` or the agent
//! socket, and the battery greps for both.
//!
//! # Controls (loopback)
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
//! Views are cached against the `war view watch` fingerprint of the records
//! (plus the roadmap directory). `/api/version` returns it; the page asks
//! every two seconds and refetches when it moves, so a signature given in a
//! terminal appears without a reload.
//!
//! No async (OW-ADR-0014): one accept loop, and an act runs on its own thread
//! so the loop stays responsive while the key's dialog waits for the human.
//!
//! # LAN (`war view ui --lan`, OW-WAR-0139)
//!
//! Opt-in, beside the loopback page, which is unchanged and stays the only
//! page that can start a signing act.
//!
//! - TLS only ([`tls`]): `--lan` refuses to start without `--cert` and
//!   `--key` (or the labelled `--self-signed` fallback), before anything
//!   binds. A failed handshake — plain HTTP included — gets no HTTP at all.
//! - The Host must be exactly `<name>:<port>` (else 421) and an Origin
//!   exactly `https://<name>:<port>`. Every response adds HSTS to the
//!   loopback headers.
//! - No request is trusted for its address, loopback included: every
//!   `/api/` request needs a paired device's credential ([`pairing`]); the
//!   loopback token means nothing here.
//! - U-001 option A: a device's queue rows carry the verdict and the host
//!   command and no act id; a POST naming a signing id is 403
//!   `act.host-only` and nothing runs. A device may start an `auto` remedy
//!   with a fresh single-use nonce, and may mark a signing act "requested
//!   from <device>" in the host's queue — which starts nothing.
//! - Connections are served on their own threads, at most
//!   [`LAN_CONNECTIONS`] at once, with the loopback's size bounds and a
//!   shorter time bound ([`LAN_REQUEST_TIME`]).
//!
//! # Tickets (t-67ed)
//!
//! `/api/tickets` is `ticket::board`, read-only, on both listeners. The
//! loopback page may also claim or finish a ticket item: `POST /api/ticket`
//! with `{"act": "claim"|"done", "target": "t-…/i-…", "note"?}`, run in
//! process through the same `ticket::claim_cmd` / `ticket::done` the CLI
//! runs. A ticket act is not a signing act and runs no argv. From a LAN
//! device the same route is 403 `act.host-only` and nothing is written: a
//! device reads tickets, it never changes them.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::repo::{RepoError, Repository};

pub mod pairing;
pub mod tls;

const INDEX: &str = include_str!("assets/index.html");
const APP_JS: &str = include_str!("assets/app.js");
const APP_CSS: &str = include_str!("assets/app.css");

/// The CSP every response carries. No `unsafe-inline` anywhere: the page's
/// script and style are separate files from this origin.
pub const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

const HEADER_LIMIT: usize = 8192;
const BODY_LIMIT: usize = 4096;
/// How long one loopback request may take to arrive, headers and body
/// together, from its accept. Running out of it is 408,
/// never 431 or 413: those name a size, and a client that is merely slow
/// sent nothing too large (t-5f49). Under load a client can be descheduled
/// for seconds between its connect and its first byte — measured with
/// three batteries' CPU burners running: sockets silent for 2.1 s, 4.8 s
/// and 5.2 s after accept, and one curl whose first write came 8.8 s after
/// its connect. Each connection is read on its own thread, so a silent
/// socket costs a thread for this long, not the page.
const LOOPBACK_REQUEST_TIME: Duration = Duration::from_secs(12);
/// At most this many loopback connections are open (being read or waiting
/// to be served) at once; the accept loop waits for a slot past it.
const LOOPBACK_CONNECTIONS: usize = 32;
/// The same bound on the LAN listener, unchanged by t-5f49: its
/// connections are served on their own threads, [`LAN_CONNECTIONS`] at
/// once, and a peer on the network is not the owner's starved local
/// client, so a slot is not held longer than it was.
const LAN_REQUEST_TIME: Duration = Duration::from_secs(2);
/// After a refusal, how many unread request bytes the server discards, and
/// for how long, before it closes (t-26ca). Bounded both ways: a client
/// that keeps sending is cut off, never read to its end.
const DRAIN_LIMIT: usize = 64 * 1024;
const DRAIN_TIME: Duration = Duration::from_secs(1);

/// The command an act runs: this server's own binary (t-470e).
/// `current_exe()` names a path, and when the file at that path is
/// replaced while the server runs — a rebuild, an upgrade — the path names
/// a deleted file and the act fails to start (`exit -1`, "No such file or
/// directory") after the page said 202. On Linux `/proc/self/exe`, opened
/// by the child that forks from this server, is the running binary's own
/// inode whatever happened to the path, so the act runs the binary that
/// built the allowlist; its argv\[0\] stays `war`. Elsewhere, `current_exe()`.
fn war_command() -> std::io::Result<std::process::Command> {
    #[cfg(target_os = "linux")]
    {
        let proc_exe = std::path::Path::new("/proc/self/exe");
        if proc_exe.exists() {
            use std::os::unix::process::CommandExt;
            let mut c = std::process::Command::new(proc_exe);
            c.arg0("war");
            return Ok(c);
        }
    }
    std::env::current_exe().map(std::process::Command::new)
}

/// Close a connection whose response is written without resetting it
/// (t-26ca). A refusal stops reading mid-request; closing a socket with
/// unread bytes makes the kernel send RST, and a client that has not yet
/// read the answer loses it (curl: status 000, "connection reset by
/// peer"). So: end our side (FIN after the response), then discard what
/// the client still sends until it closes, up to [`DRAIN_LIMIT`] bytes
/// and [`DRAIN_TIME`], then close.
fn drain_and_close(tcp: &mut TcpStream) {
    let _ = tcp.flush();
    let _ = tcp.shutdown(std::net::Shutdown::Write);
    let until = Instant::now() + DRAIN_TIME;
    let mut seen = 0usize;
    let mut sink = [0u8; 4096];
    while seen < DRAIN_LIMIT {
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() || tcp.set_read_timeout(Some(left)).is_err() {
            break;
        }
        match tcp.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(n) => seen += n,
        }
    }
}

fn err(s: impl std::fmt::Display) -> RepoError {
    RepoError::Message(s.to_string())
}

/// `war view ui`. Blocks until interrupted.
pub fn run(
    root: Utf8PathBuf,
    port: u16,
    page: &str,
    actor: Option<String>,
    mode: crate::output::Mode,
) -> Result<u8, RepoError> {
    run_with(root, port, page, actor, None, mode)
}

/// `war view ui --lan …`: what the LAN listener needs (OW-WAR-0139).
#[derive(Debug, Clone)]
pub struct LanOptions {
    /// `--lan <addr:port>`, exactly as typed.
    pub addr: String,
    /// `--name`: the name devices use; the address when absent.
    pub name: Option<String>,
    pub cert: Option<Utf8PathBuf>,
    pub key: Option<Utf8PathBuf>,
    /// `--self-signed`: U-002 option B, the labelled fallback.
    pub self_signed: bool,
    pub answer_ttl: Duration,
    pub code_ttl: Duration,
    pub device_ttl: Duration,
}

/// At most this many LAN connections are served at once; more are dropped.
pub const LAN_CONNECTIONS: usize = 16;

/// The header the LAN adds to every response.
const HSTS: &str = "Strict-Transport-Security: max-age=31536000\r\n";

/// The LAN listener's state.
struct Lan {
    /// The Host every request must name: `<name>:<port>` (`<name>` on 443).
    host: String,
    /// `https://<host>`.
    origin: String,
    tls: tls::Tls,
    store: pairing::Store,
    codes: pairing::Codes,
    tty: pairing::HostTty,
    nonces: pairing::Nonces,
    answer_ttl: Duration,
    device_ttl: Duration,
    live: AtomicUsize,
}

impl Lan {
    /// Every refusal that can be made before binding, made before binding:
    /// no TLS material, a bad address or name, a device file under the
    /// repository.
    fn prepare(repo: &Repository, o: &LanOptions) -> Result<(Self, SocketAddr), RepoError> {
        let addr: SocketAddr = o.addr.parse().map_err(|_| {
            err(format!(
                "ui.lan-address: `--lan {}` is not an address:port (an IP literal; 0.0.0.0 only if you mean every interface)",
                o.addr
            ))
        })?;
        let name = match &o.name {
            Some(n) => n.clone(),
            None => match addr.ip() {
                std::net::IpAddr::V6(ip) => format!("[{ip}]"),
                ip => ip.to_string(),
            },
        };
        let plain = name.trim_start_matches('[').trim_end_matches(']');
        if name.is_empty()
            || !plain
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
        {
            return Err(err(format!(
                "ui.lan-name: `--name {name}` is not a host name or IP address"
            )));
        }
        let tls = match (&o.cert, &o.key, o.self_signed) {
            (Some(c), Some(k), false) => tls::Tls::operator(c, k).map_err(err)?,
            (None, None, true) => {
                tls::Tls::self_signed(&name, &pairing::state_dir().map_err(err)?.join("ui-tls"))
                    .map_err(err)?
            }
            _ => {
                return Err(err(
                    "ui.lan-needs-tls: `war view ui --lan` serves only TLS and did not start. Pass \
                     --cert <pem> and --key <pem> (an operator certificate, for example from \
                     `tailscale cert` or a local CA), or --self-signed (the fallback: every device \
                     shows a browser warning). Nothing was bound.",
                ));
            }
        };
        let store = pairing::Store::open(&repo.root).map_err(err)?;
        Ok((
            Self {
                host: name,
                origin: String::new(),
                tls,
                store,
                codes: pairing::Codes::new(o.code_ttl),
                tty: pairing::HostTty::default(),
                nonces: pairing::Nonces::default(),
                answer_ttl: o.answer_ttl,
                device_ttl: o.device_ttl,
                live: AtomicUsize::new(0),
            },
            addr,
        ))
    }

    /// A new pairing link, as text with its QR code.
    fn pairing_link(&self) -> Result<(String, String), RepoError> {
        let code = self.codes.fresh().map_err(err)?;
        let url = format!(
            "{}/#pair={code}&fp={}",
            self.origin,
            self.tls.fingerprint.replace(':', "")
        );
        let text = format!(
            "war ui: pair a device: open this on it (single use, {} min; you confirm it here):\n  {url}\n{}",
            self.codes.ttl().as_secs().div_ceil(60),
            qr(&url)
        );
        Ok((url, text))
    }
}

/// `text` as a QR code in half-block characters, light modules drawn, for a
/// terminal with a dark background; empty when it cannot be encoded.
fn qr(text: &str) -> String {
    let Ok(q) = qrcodegen::QrCode::encode_text(text, qrcodegen::QrCodeEcc::Low) else {
        return String::new();
    };
    let n = q.size();
    let border = 2;
    let light = |x: i32, y: i32| !q.get_module(x, y);
    let mut out = String::new();
    let mut y = -border;
    while y < n + border {
        out.push_str("  ");
        for x in -border..n + border {
            out.push(match (light(x, y), light(x, y + 1)) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            });
        }
        out.push('\n');
        y += 2;
    }
    out
}

/// `war view ui [--lan …]`. Blocks until interrupted.
pub fn run_with(
    root: Utf8PathBuf,
    port: u16,
    page: &str,
    actor: Option<String>,
    lan: Option<LanOptions>,
    mode: crate::output::Mode,
) -> Result<u8, RepoError> {
    let repo = Repository::discover(Some(root))?;
    let lan = match &lan {
        Some(o) => Some(Lan::prepare(&repo, o)?),
        None => None,
    };
    let token = token()?;
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(err)?;
    listener.set_nonblocking(true).map_err(err)?;
    let host = listener.local_addr().map_err(err)?.to_string();
    let url = format!("http://{host}/#t={token}&p={page}");
    let (lan, lan_listener) = match lan {
        None => {
            crate::output::emit(
                mode,
                "ui",
                &format!(
                    "war ui: {url}\n  this machine only; the token in the link is this session's, and a restart rotates it. Ctrl-C stops."
                ),
                json!({"url": url, "host": host, "loopback_only": true}),
            );
            (None, None)
        }
        Some((mut lan, addr)) => {
            let l = TcpListener::bind(addr)
                .map_err(|e| err(format!("ui.lan-bind: could not bind {addr}: {e}")))?;
            let bound = l.local_addr().map_err(err)?;
            if bound.port() != 443 {
                lan.host = format!("{}:{}", lan.host, bound.port());
            }
            lan.origin = format!("https://{}", lan.host);
            let (pair_url, pair_text) = lan.pairing_link()?;
            crate::output::emit(
                mode,
                "ui",
                &format!(
                    "war ui: {url}\n  this machine: the only page that can sign. The token in the link is this session's, and a restart rotates it. Ctrl-C stops.\n\
war ui --lan: {origin}/ — TLS, paired devices only (bound to {bound})\n  certificate: {label}\n  SHA-256 fingerprint: {fp}\n  \
a paired device reads, runs automatic remedies and can ask for a signature here; it can never sign.\n{pair_text}",
                    origin = lan.origin,
                    label = lan.tls.source.label(),
                    fp = lan.tls.fingerprint,
                ),
                json!({"url": url, "host": host, "loopback_only": false, "lan": {
                    "url": format!("{}/", lan.origin), "bound": bound.to_string(),
                    "certificate": lan.tls.source.label(), "fingerprint": lan.tls.fingerprint,
                    "pairing_url": pair_url, "devices": lan.store.path.as_str(),
                }}),
            );
            (Some(lan), Some(l))
        }
    };
    std::io::stdout().flush().map_err(err)?;
    let state = Arc::new(Server {
        repo,
        host,
        token,
        actor,
        cache: Mutex::new(BTreeMap::new()),
        corpus: Mutex::new(None),
        act: Arc::new(Mutex::new(ActState::Idle)),
        lan,
        requests: Mutex::new(BTreeMap::new()),
        request_gen: AtomicU64::new(0),
    });
    if let Some(l) = lan_listener {
        let s = Arc::clone(&state);
        std::thread::spawn(move || {
            for stream in l.incoming() {
                let Ok(stream) = stream else { continue };
                let Some(lan) = s.lan.as_ref() else { break };
                if lan.live.fetch_add(1, Ordering::SeqCst) >= LAN_CONNECTIONS {
                    lan.live.fetch_sub(1, Ordering::SeqCst);
                    drop(stream);
                    continue;
                }
                let s = Arc::clone(&s);
                std::thread::spawn(move || {
                    s.handle_lan(stream);
                    if let Some(lan) = s.lan.as_ref() {
                        lan.live.fetch_sub(1, Ordering::SeqCst);
                    }
                });
            }
        });
    }
    // Each loopback connection is READ on its own thread, and SERVED one at
    // a time under `turn` (t-5f49). Reading on the accept loop let one
    // silent socket — a client starved of CPU, or a browser's speculative
    // preconnect — hold every other request for the whole time bound;
    // serving stays serial, as it always was, so no route runs beside
    // another. At most LOOPBACK_CONNECTIONS are open at once; past that the
    // loop stops accepting (the kernel queues) rather than dropping one.
    let turn = Arc::new(Mutex::new(()));
    let live = Arc::new(AtomicUsize::new(0));
    loop {
        if live.load(Ordering::SeqCst) >= LOOPBACK_CONNECTIONS {
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                live.fetch_add(1, Ordering::SeqCst);
                let (s, turn, live) = (Arc::clone(&state), Arc::clone(&turn), Arc::clone(&live));
                std::thread::spawn(move || {
                    let _ = s.handle(stream, &turn);
                    live.fetch_sub(1, Ordering::SeqCst);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(15));
            }
            Err(e) => return Err(err(e)),
        }
    }
}

/// `war view ui devices [--revoke <id>]`: the devices paired for this repository.
pub fn devices(
    root: Utf8PathBuf,
    revoke: Option<String>,
    mode: crate::output::Mode,
) -> Result<u8, RepoError> {
    let repo = Repository::discover(Some(root))?;
    let store = pairing::Store::open(&repo.root).map_err(err)?;
    if let Some(id) = revoke {
        let d = store.revoke(&id).map_err(err)?;
        crate::output::emit(
            mode,
            "ui",
            &format!(
                "war ui devices: revoked {} ({}); its next request is refused.",
                d.id, d.address
            ),
            json!({"revoked": d.row(), "devices_file": store.path.as_str()}),
        );
        return Ok(0);
    }
    let list = store.list().map_err(err)?;
    let mut text = format!("war ui devices — {} ({} paired)\n", store.path, list.len());
    for d in &list {
        text.push_str(&format!(
            "  {}  {:<8} {}  paired {}  expires {}  {}\n",
            d.id,
            d.state(),
            d.address,
            d.row()["paired_at"].as_str().unwrap_or(""),
            d.row()["expires_at"].as_str().unwrap_or(""),
            d.user_agent
        ));
    }
    if list.is_empty() {
        text.push_str(
            "  none — `war view ui --lan <addr:port> --cert … --key …` prints a pairing link\n",
        );
    }
    crate::output::emit(
        mode,
        "ui",
        text.trim_end(),
        json!({"devices": list.iter().map(pairing::Device::row).collect::<Vec<_>>(),
               "devices_file": store.path.as_str()}),
    );
    Ok(0)
}

/// 32 bytes from the OS, hex.
pub(crate) fn token() -> Result<String, RepoError> {
    let mut b = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| err(format!("could not read the OS random source: {e}")))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Constant-time comparison: the token is not leaked a byte at a time.
pub(crate) fn same(a: &str, b: &str) -> bool {
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
    /// One compiled corpus per watch fingerprint (OW-WAR-0148): every view
    /// built while the fingerprint holds reads it, so the status, the sign
    /// queue and the frontier are built once per change, not once per view.
    corpus: Mutex<Option<(u64, Arc<crate::corpus::Corpus>)>>,
    act: Arc<Mutex<ActState>>,
    /// `--lan`: the LAN listener's state; `None` for loopback only.
    lan: Option<Lan>,
    /// Signing acts a device asked for (U-001 option A): target → who asked.
    /// Always empty without `--lan`.
    requests: Mutex<BTreeMap<String, String>>,
    /// Moves when a request is added, so the pages refetch.
    request_gen: AtomicU64,
}

/// Who a request is from: the loopback page (the session token), or a
/// paired LAN device.
#[derive(Clone, Copy)]
enum Session<'a> {
    Loopback,
    Device(&'a Lan, &'a pairing::Device),
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

    /// The device credential cookie; `None` when absent or given twice.
    fn device_cookie(&self) -> Option<&str> {
        let found: Vec<&str> = self
            .header("cookie")
            .into_iter()
            .flat_map(|h| h.split(';'))
            .filter_map(|kv| kv.trim().split_once('='))
            .filter(|(k, _)| *k == pairing::COOKIE)
            .map(|(_, v)| v)
            .collect();
        match found.as_slice() {
            [one] => Some(one),
            _ => None,
        }
    }
}

/// Anything a response can be written to and a request read from.
trait Io: Read + Write {}
impl<T: Read + Write> Io for T {}

/// A connection: loopback TCP, or a LAN TLS stream. `extra` is added to
/// every response's headers — HSTS on the LAN, nothing on loopback, whose
/// responses stay byte-for-byte what OW-WAR-0116 sent.
struct Conn<'a> {
    io: &'a mut dyn Io,
    extra: &'static str,
}

fn respond(stream: &mut Conn, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    respond_with(stream, status, kind, body, "")
}

fn respond_with(
    stream: &mut Conn,
    status: &str,
    kind: &str,
    body: &[u8],
    more: &str,
) -> std::io::Result<()> {
    write!(
        stream.io,
        "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\
Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n\
X-Frame-Options: DENY\r\nCross-Origin-Resource-Policy: same-origin\r\nContent-Security-Policy: {CSP}\r\n{}{more}\r\n",
        body.len(),
        stream.extra
    )?;
    stream.io.write_all(body)
}

fn json_response(stream: &mut Conn, status: &str, v: &Value) -> std::io::Result<()> {
    respond(
        stream,
        status,
        "application/json",
        &serde_json::to_vec(v).unwrap_or_default(),
    )
}

/// A read that ran into the socket's read timeout: the peer paused, it did
/// not end. The caller's deadline decides when a pause is too long.
pub(crate) fn stalled(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// Read one request within the bounds, or say which bound it broke:
/// `within` is the time bound. The caller has set the socket's read
/// timeout.
fn read_request(
    stream: &mut dyn Read,
    within: Duration,
) -> Result<Request, (&'static str, &'static str)> {
    let deadline = Instant::now() + within;
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    let head_end = loop {
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        // Two bounds, two answers (t-5f49). Only the size bound is 431; a
        // header section still incomplete when the time runs out is 408,
        // whatever the request would have carried. Under load the time
        // bound fired on a socket that had delivered nothing yet, and the
        // client, whose headers were ordinary and whose body was the
        // oversized part, was told its headers were too large.
        if bytes.len() >= HEADER_LIMIT {
            return Err((
                "431 Request Header Fields Too Large",
                "request limit exceeded",
            ));
        }
        if Instant::now() >= deadline {
            return Err(("408 Request Timeout", "request not received in time"));
        }
        match stream.read(&mut chunk) {
            Ok(n) if n > 0 => bytes.extend_from_slice(&chunk[..n]),
            // A read that times out is a pause, not an end: the deadline
            // above is the bound (t-26ca). Under load a client can stall
            // longer than one read's timeout mid-request.
            Err(e) if stalled(&e) => {}
            Ok(_) | Err(_) => return Err(("400 Bad Request", "incomplete request")),
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
            Ok(n) if n > 0 => body.extend_from_slice(&chunk[..n]),
            Err(e) if stalled(&e) => {}
            Ok(_) | Err(_) => return Err(("400 Bad Request", "incomplete body")),
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

/// A ticket act from the loopback page (t-67ed): which act, on which ticket
/// or item, and for `done` an optional note. Nothing else; never an argv.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TicketBody {
    act: TicketAct,
    target: String,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum TicketAct {
    Claim,
    Done,
}

/// A ticket or item reference as the ticket commands take it: `t-…`,
/// `i-…` or `t-…/i-…`, lowercase hex, and short. Anything else is refused
/// before the store is opened.
fn ticket_target_ok(t: &str) -> bool {
    !t.is_empty()
        && t.len() <= 64
        && t.split('/').count() <= 2
        && t.split('/').all(|part| {
            (part.starts_with("t-") || part.starts_with("i-"))
                && part.len() > 2
                && part[2..]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

/// A LAN device's act: the row id and a nonce the server issued to it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceActBody {
    id: String,
    nonce: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PairBody {
    code: String,
}

/// A user agent as the host terminal shows it: printable, bounded.
fn printable(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control())
        .take(120)
        .collect::<String>()
}

impl Server {
    /// One loopback connection: read its request on the caller's thread,
    /// then serve it holding `turn`, so requests are served one at a time
    /// however many are being read. A refusal of the read is answered
    /// without waiting for the turn: it runs no route.
    fn handle(&self, tcp: TcpStream, turn: &Mutex<()>) -> std::io::Result<()> {
        let mut tcp = tcp;
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(1)));
        let _ = tcp.set_write_timeout(Some(Duration::from_secs(2)));
        let mut stream = Conn {
            io: &mut tcp,
            extra: "",
        };
        let stream = &mut stream;
        let req = match read_request(stream.io, LOOPBACK_REQUEST_TIME) {
            Ok(r) => r,
            Err((status, why)) => {
                let answered = respond(stream, status, "text/plain", why.as_bytes());
                drain_and_close(&mut tcp);
                return answered;
            }
        };
        let _turn = turn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // The Host is exactly the bound loopback address; an Origin, when
        // present, is exactly this page's. A DNS-rebinding page names another
        // Host; a cross-site page sends another Origin.
        let origin = format!("http://{}", self.host);
        if req.header("host") != [self.host.as_str()] {
            return respond(stream, "400 Bad Request", "text/plain", b"wrong host");
        }
        let origins = req.header("origin");
        if origins.len() > 1 || origins.iter().any(|o| *o != origin) {
            return respond(stream, "403 Forbidden", "text/plain", b"foreign origin");
        }
        let path = req.path.split('?').next().unwrap_or("").to_owned();
        match (req.method.as_str(), path.as_str()) {
            ("GET", "/" | "/assets/app.js" | "/assets/app.css") => asset(stream, &path),
            (_, p) if p.starts_with("/api/") => {
                let bearer = req
                    .header("authorization")
                    .first()
                    .and_then(|v| v.strip_prefix("Bearer "))
                    .map(str::to_owned)
                    .unwrap_or_default();
                if !same(&bearer, &self.token) {
                    return respond(
                        stream,
                        "401 Unauthorized",
                        "text/plain",
                        b"session token required",
                    );
                }
                self.api(stream, &req, p, Session::Loopback)
            }
            ("GET", _) => respond(stream, "404 Not Found", "text/plain", b"unknown route"),
            _ => respond(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
        }
    }

    /// One LAN connection: the TLS handshake, then one request. A failed
    /// handshake writes nothing.
    fn handle_lan(&self, tcp: TcpStream) {
        let Some(lan) = self.lan.as_ref() else { return };
        let peer = tcp
            .peer_addr()
            .map_or_else(|_| "unknown".to_owned(), |a| a.ip().to_string());
        let Ok(mut tls) = lan.tls.accept(tcp) else {
            return;
        };
        {
            let mut conn = Conn {
                io: &mut tls,
                extra: HSTS,
            };
            let _ = self.serve_lan(&mut conn, lan, &peer);
            let _ = conn.io.flush();
        }
        tls.conn.send_close_notify();
        let _ = tls.flush();
        // A refusal leaves request bytes unread here too; the answer must
        // not be lost to a reset (t-26ca).
        drain_and_close(&mut tls.sock);
    }

    fn serve_lan(&self, stream: &mut Conn, lan: &Lan, peer: &str) -> std::io::Result<()> {
        let req = match read_request(stream.io, LAN_REQUEST_TIME) {
            Ok(r) => r,
            Err((status, why)) => {
                return respond(stream, status, "text/plain", why.as_bytes());
            }
        };
        if req.header("host") != [lan.host.as_str()] {
            return respond(
                stream,
                "421 Misdirected Request",
                "text/plain",
                format!("ui.wrong-host: this server answers only to {}", lan.host).as_bytes(),
            );
        }
        let origins = req.header("origin");
        if origins.len() > 1 || origins.iter().any(|o| *o != lan.origin) {
            return respond(stream, "403 Forbidden", "text/plain", b"foreign origin");
        }
        let path = req.path.split('?').next().unwrap_or("").to_owned();
        match (req.method.as_str(), path.as_str()) {
            ("GET", "/" | "/assets/app.js" | "/assets/app.css") => asset(stream, &path),
            ("POST", "/pair") => self.pair(stream, lan, &req, peer),
            (_, p) if p.starts_with("/api/") => {
                // No address is trusted, loopback included, and the loopback
                // token means nothing here: only a paired device's credential.
                match lan.store.authenticate(req.device_cookie()) {
                    Ok(device) => self.api(stream, &req, p, Session::Device(lan, &device)),
                    Err(why) => respond(
                        stream,
                        "401 Unauthorized",
                        "text/plain",
                        why.message().as_bytes(),
                    ),
                }
            }
            ("GET", _) => respond(stream, "404 Not Found", "text/plain", b"unknown route"),
            _ => respond(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
        }
    }

    /// POST /pair: spend the code, ask the human at the host, and issue a
    /// credential only on a `y` typed there.
    fn pair(&self, stream: &mut Conn, lan: &Lan, req: &Request, peer: &str) -> std::io::Result<()> {
        if req.header("origin") != [lan.origin.as_str()] {
            return respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"pairing needs the page's origin",
            );
        }
        let Ok(body) = serde_json::from_slice::<PairBody>(&req.body) else {
            return respond(
                stream,
                "400 Bad Request",
                "text/plain",
                b"a pairing names its code and nothing else",
            );
        };
        if let Err(why) = lan.codes.take(&body.code) {
            if why == pairing::CodeRefusal::Expired
                && let Ok((_, text)) = lan.pairing_link()
            {
                eprintln!("war ui: a pairing code expired unused.\n{text}");
            }
            return respond(
                stream,
                "403 Forbidden",
                "text/plain",
                why.message().as_bytes(),
            );
        }
        let ua = printable(
            req.header("user-agent")
                .first()
                .copied()
                .unwrap_or("no user agent"),
        );
        let days = lan.device_ttl.as_secs() / 86_400;
        let ttl = if days > 0 {
            format!("{days} day(s)")
        } else {
            format!("{} s", lan.device_ttl.as_secs())
        };
        let question = format!(
            "\nwar ui: pair a device from {peer} ({ua})?\n  For {ttl} it could read this program, run automatic remedies and ask for a signature here. It can never sign.\n  Pair it? [y/N] ({} s) ",
            lan.answer_ttl.as_secs()
        );
        let answer = lan.tty.ask(&question, lan.answer_ttl);
        let result = match answer {
            pairing::Answer::Yes => match lan.store.issue(peer, &ua, lan.device_ttl) {
                Ok((device, credential)) => {
                    eprintln!(
                        "war ui: paired device {} from {peer}; `war view ui devices --revoke {}` revokes it.",
                        device.id, device.id
                    );
                    let cookie = format!(
                        "Set-Cookie: {}={credential}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age={}\r\n",
                        pairing::COOKIE,
                        lan.device_ttl.as_secs()
                    );
                    let body = json!({
                        "paired": true, "device": device.id,
                        "expires_at": device.row()["expires_at"],
                    });
                    respond_with(
                        stream,
                        "200 OK",
                        "application/json",
                        &serde_json::to_vec(&body).unwrap_or_default(),
                        &cookie,
                    )
                }
                Err(e) => json_response(stream, "500 Internal Server Error", &json!({"error": e})),
            },
            pairing::Answer::No => respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"pair.declined: the host's human answered no; no credential was issued",
            ),
            pairing::Answer::TimedOut => respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"pair.timed-out: nobody answered at the host in time; no credential was issued",
            ),
            pairing::Answer::NoTty => respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"pair.no-tty: the server has no terminal to ask; a pairing is confirmed only by a human at the host's terminal, so none was issued",
            ),
        };
        let refused = match answer {
            pairing::Answer::Yes => None,
            pairing::Answer::No => Some("pair.declined"),
            pairing::Answer::TimedOut => Some("pair.timed-out"),
            pairing::Answer::NoTty => Some("pair.no-tty"),
        };
        if let Some(rule) = refused {
            eprintln!("war ui: not paired, {peer}: {rule}; no credential was issued.");
        }
        if let Ok((_, text)) = lan.pairing_link() {
            eprintln!("{text}");
        }
        result
    }

    fn api(
        &self,
        stream: &mut Conn,
        req: &Request,
        path: &str,
        session: Session,
    ) -> std::io::Result<()> {
        if let Session::Device(lan, device) = session {
            match (req.method.as_str(), path) {
                // t-67ed: a ticket act is the loopback page's only. A
                // device reads `/api/tickets`; it never claims or finishes.
                ("POST", "/api/ticket") => {
                    eprintln!(
                        "war ui: refused act.host-only from {}: a ticket act; nothing ran",
                        device.label()
                    );
                    return respond(
                        stream,
                        "403 Forbidden",
                        "text/plain",
                        b"act.host-only: a ticket act starts only on the host's loopback page; this device can read tickets, never change them. Nothing ran.",
                    );
                }
                ("GET", "/api/nonce") => {
                    return match lan.nonces.issue(&device.id) {
                        Ok(n) => json_response(stream, "200 OK", &json!({"nonce": n})),
                        Err(e) => {
                            json_response(stream, "500 Internal Server Error", &json!({"error": e}))
                        }
                    };
                }
                ("POST", "/api/act" | "/api/request") => {
                    if req.header("origin").is_empty() {
                        return respond(
                            stream,
                            "403 Forbidden",
                            "text/plain",
                            b"an act needs the page's origin",
                        );
                    }
                    let Ok(body) = serde_json::from_slice::<DeviceActBody>(&req.body) else {
                        return respond(
                            stream,
                            "400 Bad Request",
                            "text/plain",
                            b"a device's act names one row id and its nonce, and nothing else",
                        );
                    };
                    if let Err(why) = lan.nonces.spend(&device.id, &body.nonce) {
                        eprintln!(
                            "war ui: refused {} from {}: {}",
                            body.id,
                            device.label(),
                            why.message()
                        );
                        return respond(
                            stream,
                            why.status(),
                            "text/plain",
                            why.message().as_bytes(),
                        );
                    }
                    return if path == "/api/request" {
                        self.request_at_host(stream, &body.id, device)
                    } else {
                        self.start_act(stream, &body.id, Some(device))
                    };
                }
                ("GET", "/api/version") => {
                    return json_response(
                        stream,
                        "200 OK",
                        &json!({
                            "version": format!("{:016x}", self.fingerprint() ^ self.request_gen.load(Ordering::SeqCst)),
                            "program": self.repo.config.project.name,
                            "root": self.repo.root.as_str(),
                            "war": env!("CARGO_PKG_VERSION"),
                            "lan": true,
                            "device": device.id,
                        }),
                    );
                }
                ("GET", "/api/queue") => {
                    return match self.view("queue") {
                        Ok(Some(v)) => json_response(stream, "200 OK", &self.device_queue(v)),
                        Ok(None) => respond(stream, "404 Not Found", "text/plain", b"unknown view"),
                        Err(e) => json_response(
                            stream,
                            "500 Internal Server Error",
                            &json!({"error": e.to_string()}),
                        ),
                    };
                }
                _ => {}
            }
        }
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
                self.start_act(stream, &body.id, None)
            }
            ("POST", "/api/ticket") => {
                if !matches!(session, Session::Loopback) {
                    return respond(
                        stream,
                        "403 Forbidden",
                        "text/plain",
                        b"act.host-only: a ticket act starts only on the host's loopback page. Nothing ran.",
                    );
                }
                if req.header("origin").is_empty() {
                    return respond(
                        stream,
                        "403 Forbidden",
                        "text/plain",
                        b"an act needs the page's origin",
                    );
                }
                let body = match serde_json::from_slice::<TicketBody>(&req.body) {
                    Ok(b) if ticket_target_ok(&b.target) => b,
                    _ => {
                        return respond(
                            stream,
                            "400 Bad Request",
                            "text/plain",
                            b"a ticket act names act (claim|done), target (t-.../i-...) and, for done, a note; nothing else",
                        );
                    }
                };
                self.ticket_act(stream, &body)
            }
            (_, "/api/ticket") => respond(
                stream,
                "405 Method Not Allowed",
                "text/plain",
                b"method not allowed",
            ),
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
                    "version": format!("{:016x}", self.fingerprint() ^ self.request_gen.load(Ordering::SeqCst)),
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
                    Ok(Some(v)) if name == "queue" => {
                        json_response(stream, "200 OK", &self.with_requests(v))
                    }
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

    /// The loopback queue, with "requested from <device>" beside any act a
    /// device asked for. Unchanged when nothing was asked — always, without
    /// `--lan`.
    fn with_requests(&self, mut v: Value) -> Value {
        let Ok(requests) = self.requests.lock() else {
            return v;
        };
        if requests.is_empty() {
            return v;
        }
        for a in v["acts"].as_array_mut().into_iter().flatten() {
            if let Some(who) = a["target"].as_str().and_then(|t| requests.get(t)) {
                a["requested_from"] = json!(who);
            }
        }
        v
    }

    /// The queue as a LAN device sees it (U-001 option A): each act's
    /// verdict and the command to run at the host, no act id anywhere, and
    /// a request id that can only mark the act requested.
    fn device_queue(&self, v: Value) -> Value {
        let mut v = self.with_requests(v);
        for a in v["acts"].as_array_mut().into_iter().flatten() {
            if let Some(o) = a.as_object_mut() {
                o.remove("act_id");
                o.insert("host_only".to_owned(), json!(true));
                if let Some(t) = o.get("target").and_then(Value::as_str) {
                    let id = act_id("request", t);
                    o.insert("request_id".to_owned(), json!(id));
                }
            }
            for c in a
                .get_mut("choices")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
            {
                if let Some(o) = c.as_object_mut() {
                    o.remove("act_id");
                }
            }
        }
        if let Some(b) = v["batch"].as_object_mut() {
            b.remove("act_id");
            b.insert("host_only".to_owned(), json!(true));
        }
        v
    }

    /// POST /api/request from a device: mark one signing act "requested
    /// from <device>" in the host's queue. Nothing starts.
    fn request_at_host(
        &self,
        stream: &mut Conn,
        id: &str,
        device: &pairing::Device,
    ) -> std::io::Result<()> {
        let queue = match self.view("queue") {
            Ok(Some(q)) => q,
            _ => return respond(stream, "404 Not Found", "text/plain", b"no queue"),
        };
        let Some(act) = queue["acts"].as_array().into_iter().flatten().find(|a| {
            a["target"]
                .as_str()
                .is_some_and(|t| act_id("request", t) == id)
        }) else {
            return respond(
                stream,
                "404 Not Found",
                "text/plain",
                b"no act in the queue has this request id",
            );
        };
        let target = act["target"].as_str().unwrap_or_default().to_owned();
        let command = act["command"].as_str().unwrap_or_default().to_owned();
        if let Ok(mut r) = self.requests.lock() {
            r.insert(target.clone(), device.label());
        }
        self.request_gen.fetch_add(1, Ordering::SeqCst);
        eprintln!(
            "war ui: {} asks for a signature: `{command}` — sign it at this machine; no device can.",
            device.label()
        );
        json_response(
            stream,
            "200 OK",
            &json!({"requested": target, "command": command, "from": device.label()}),
        )
    }

    /// The records' fingerprint: `war view watch`'s trees plus the roadmap.
    /// A loopback ticket act, in process: the same `ticket::claim_cmd` or
    /// `ticket::done` the CLI runs, acting as `--as` when given. 200 with the
    /// command's words, or 409 naming the rule it refused by.
    fn ticket_act(&self, stream: &mut Conn, body: &TicketBody) -> std::io::Result<()> {
        let outcome = Repository::discover(Some(self.repo.root.clone()))
            .and_then(|repo| crate::ticket::Store::open(&repo, self.actor.as_deref()))
            // M11: an act by the holder renews its leases, as the CLI's do.
            .inspect(crate::ticket::Store::renew_all)
            .and_then(|store| match body.act {
                TicketAct::Claim => crate::ticket::claim_cmd(&store, &body.target, false),
                TicketAct::Done => crate::ticket::done(
                    &store,
                    &body.target,
                    body.note
                        .as_deref()
                        .map(str::trim)
                        .filter(|n| !n.is_empty()),
                    None,
                ),
            });
        let word = match body.act {
            TicketAct::Claim => "claim",
            TicketAct::Done => "done",
        };
        match outcome {
            Ok(o) if !o.is_refused() => {
                eprintln!("war ui: ticket {word} {}", body.target);
                json_response(
                    stream,
                    "200 OK",
                    &json!({"ok": true, "act": word, "target": body.target, "message": o.human}),
                )
            }
            Ok(o) => {
                let rule = o
                    .report
                    .diagnostics
                    .iter()
                    .find(|d| d.severity == crate::diagnostic::Severity::Error)
                    .map(|d| d.rule.clone())
                    .unwrap_or_default();
                json_response(
                    stream,
                    "409 Conflict",
                    &json!({"ok": false, "act": word, "target": body.target, "rule": rule, "message": o.human}),
                )
            }
            Err(e) => json_response(
                stream,
                "500 Internal Server Error",
                &json!({"ok": false, "error": e.to_string()}),
            ),
        }
    }

    fn fingerprint(&self) -> u64 {
        let mut dirs = crate::watch::watched_dirs(&self.repo);
        dirs.push(crate::roadmap_cmd::dir(&self.repo));
        dirs.extend(crate::ticket::watched(&self.repo));
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

    /// The corpus for fingerprint `fp`: the one held when the fingerprint
    /// has not moved, else the repository re-read (records changed since the
    /// server began) into a new one.
    fn corpus_at(&self, fp: u64) -> Result<Arc<crate::corpus::Corpus>, RepoError> {
        if let Ok(held) = self.corpus.lock()
            && let Some((f, c)) = held.as_ref()
            && *f == fp
        {
            return Ok(Arc::clone(c));
        }
        let repo = Repository::discover(Some(self.repo.root.clone()))?;
        let corpus = Arc::new(crate::corpus::Corpus::new(&repo));
        if let Ok(mut held) = self.corpus.lock() {
            *held = Some((fp, Arc::clone(&corpus)));
        }
        Ok(corpus)
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
        let built = self
            .corpus_at(fp)
            .and_then(|corpus| build_view(&corpus, name, self.actor.as_deref()));
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
    ///
    /// From a LAN device (`from`), a signing act is refused `act.host-only`
    /// before anything runs (U-001 option A): only an `auto` remedy starts.
    fn start_act(
        &self,
        stream: &mut Conn,
        id: &str,
        from: Option<&pairing::Device>,
    ) -> std::io::Result<()> {
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
        if let Some(device) = from
            && argv.first().is_some_and(|verb| verb == "sign")
        {
            eprintln!(
                "war ui: refused act.host-only from {}: {id} is a signing act; nothing ran",
                device.label()
            );
            return respond(
                stream,
                "403 Forbidden",
                "text/plain",
                b"act.host-only: a signing act starts only at the host; this device can ask for it, never start it. Nothing ran.",
            );
        }
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
        match from {
            None => eprintln!(
                "war ui: act {id}: war --root {} {}",
                self.repo.root,
                argv.join(" ")
            ),
            Some(device) => eprintln!(
                "war ui: act {id}: war --root {} {} — from {}",
                self.repo.root,
                argv.join(" "),
                device.label()
            ),
        }
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
                let text = war_command()
                    .and_then(|mut c| {
                        c.arg("--root")
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
            let out = war_command().and_then(|mut c| {
                c.arg("--root")
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

/// The page's three assets; nothing else is served from the binary.
fn asset(stream: &mut Conn, path: &str) -> std::io::Result<()> {
    match path {
        "/" => respond(
            stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX.as_bytes(),
        ),
        "/assets/app.js" => respond(
            stream,
            "200 OK",
            "text/javascript; charset=utf-8",
            APP_JS.as_bytes(),
        ),
        _ => respond(
            stream,
            "200 OK",
            "text/css; charset=utf-8",
            APP_CSS.as_bytes(),
        ),
    }
}

const VIEWS: &[&str] = &[
    "tickets",
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

fn build_view(
    corpus: &crate::corpus::Corpus,
    name: &str,
    actor: Option<&str>,
) -> Result<Value, RepoError> {
    let repo = corpus.repo();
    Ok(match name {
        "progress" => progress(corpus)?,
        "queue" => queue(corpus, actor)?,
        "questions" => {
            let b = crate::console::board_with(corpus)?;
            json!({"questions": b.questions})
        }
        "frontier" => {
            let (_, f) = corpus.frontier()?;
            serde_json::to_value(f).map_err(err)?
        }
        "corpus" => {
            let s = corpus.status()?;
            let rows: Vec<Value> = s
                .warrants
                .iter()
                .map(|w| {
                    json!({
                        "alias": w.alias, "title": w.title, "rung": w.rung,
                        "unmet": w.unmet, "unestablished": w.unestablished,
                        "command": format!("war sign resolve {} --dry-run", w.alias),
                    })
                })
                .collect();
            json!({"warrants": rows})
        }
        "help" => help(corpus)?,
        "tickets" => crate::ticket::board(&crate::ticket::Store::open(repo, None)?)?,
        // `war progress --snapshot`: attributed work reports, their links,
        // the frontier — the viewer's own validated snapshot.
        "snapshot" => json!({"snapshot": crate::progress_viewer::json_snapshot(repo)?}),
        _ => Value::Null,
    })
}

/// The Progress page: the canonical roadmap (OW-ADR-0023), each phase with
/// its members' rungs and unmet counts, the open work, and the unassigned.
fn progress(corpus: &crate::corpus::Corpus) -> Result<Value, RepoError> {
    let repo = corpus.repo();
    let status = corpus.status()?;
    let by_alias: BTreeMap<&str, &openwarrant_core::status::WarrantStatus> = status
        .warrants
        .iter()
        .map(|w| (w.alias.as_str(), w))
        .collect();
    let member = |a: &str| -> Value {
        match by_alias.get(a) {
            Some(w) => json!({
                "alias": a, "title": w.title, "rung": w.rung,
                "unmet": w.unmet.len(), "command": format!("war sign resolve {a} --dry-run"),
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
    let (roadmap, phases) = match crate::roadmap_cmd::view_with(repo, status) {
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
    // OW-WAR-0148 M13: the tickets' milestones, each with the level its tick
    // was earned at; independent of the roadmap record, so a program without
    // one still sees them.
    let milestones = crate::ticket::Store::open(repo, None)
        .ok()
        .and_then(|s| crate::ticket::ladder::tracker(&s).ok())
        .map(|t| t.milestones)
        .unwrap_or_default();
    Ok(json!({
        "roadmap": roadmap,
        "phases": phases,
        "unassigned": unassigned,
        "ladder": ladder,
        "milestones": milestones,
    }))
}

/// The Queue: every pending act with its dry-run verdict; an act id only for
/// one whose verdict is would-record and that needs no decision.
fn queue(corpus: &crate::corpus::Corpus, actor: Option<&str>) -> Result<Value, RepoError> {
    let repo = corpus.repo();
    let board = crate::console::board_with(corpus)?;
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
            "command": "war view ui --as <your name as roles.toml spells it>",
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
fn help(corpus: &crate::corpus::Corpus) -> Result<Value, RepoError> {
    let next = crate::next::run_with(corpus)?;
    let check = crate::check::run_with(corpus, None, false)?;
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
        // M9: the plain first line `war next` prints, ahead of the reason.
        "nothing": match (&next.idle, &next.nothing) {
            (Some(idle), Some(why)) => Some(format!("{idle}: {why}")),
            (Some(idle), None) => Some(idle.clone()),
            (None, why) => why.clone(),
        },
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

    /// t-26ca: a client sends an oversized request, then reads the answer
    /// only after the server has finished with the connection — a busy
    /// machine's order of events. `graceful` is the server's refusal path
    /// with [`drain_and_close`]; without it the socket is dropped with
    /// request bytes unread. Returns what the client read, or its error.
    fn refuse_then_read_late(graceful: bool) -> std::io::Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let mut server = Some(std::thread::spawn(move || {
            let (mut tcp, _) = listener.accept().expect("accept");
            let _ = tcp.set_read_timeout(Some(Duration::from_secs(1)));
            let refused = read_request(&mut tcp, LOOPBACK_REQUEST_TIME)
                .err()
                .expect("an oversized request is refused");
            {
                let mut conn = Conn {
                    io: &mut tcp,
                    extra: "",
                };
                respond(&mut conn, refused.0, "text/plain", refused.1.as_bytes()).expect("write");
            }
            if graceful {
                drain_and_close(&mut tcp);
            }
        }));
        let mut client = TcpStream::connect(addr).expect("connect");
        let big = "a".repeat(9000);
        client
            .write_all(format!("GET / HTTP/1.1\r\nHost: x\r\nX-Big: {big}\r\n\r\n").as_bytes())
            .expect("send");
        if !graceful && let Some(s) = server.take() {
            s.join().expect("server");
        }
        std::thread::sleep(Duration::from_millis(100));
        let mut got = String::new();
        let read = client.read_to_string(&mut got);
        drop(client);
        if let Some(s) = server {
            s.join().expect("server");
        }
        read.map(|_| got)
    }

    /// A client that pauses mid-request for longer than one read's timeout
    /// but inside the deadline is read to the end, not refused as
    /// incomplete (t-26ca: under load, one of fifty oversized requests got
    /// 400 instead of 431 this way).
    #[test]
    fn a_pause_shorter_than_the_deadline_is_not_the_end_of_a_request() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client = std::thread::spawn(move || {
            let mut c = TcpStream::connect(addr).expect("connect");
            c.write_all(b"GET /x HTTP/1.1\r\nHost: h\r\n")
                .expect("first half");
            std::thread::sleep(Duration::from_millis(1300));
            c.write_all(b"X-After: pause\r\n\r\n").expect("second half");
            c
        });
        let (mut tcp, _) = listener.accept().expect("accept");
        let _ = tcp.set_read_timeout(Some(Duration::from_secs(1)));
        let req = read_request(&mut tcp, LOOPBACK_REQUEST_TIME)
            .map_err(|e| e.1)
            .expect("read to the end");
        assert_eq!(req.path, "/x");
        assert_eq!(req.header("x-after"), ["pause"]);
        drop(client.join().expect("client"));
    }

    /// Read one request sent by `send` (on its own thread) within `within`,
    /// with a 100 ms read timeout; the refusal's status, or "ok".
    fn status_of(within: Duration, send: impl FnOnce(&mut TcpStream) + Send + 'static) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let client = std::thread::spawn(move || {
            let mut c = TcpStream::connect(addr).expect("connect");
            send(&mut c);
            c
        });
        let (mut tcp, _) = listener.accept().expect("accept");
        let _ = tcp.set_read_timeout(Some(Duration::from_millis(100)));
        let got =
            read_request(&mut tcp, within).map_or_else(|e| e.0.to_owned(), |_| "ok".to_owned());
        drop(client.join().expect("client"));
        got
    }

    /// t-5f49: the time bound running out is 408, not 431. Under load a
    /// socket that had delivered nothing when the bound ran out was told
    /// its headers were too large, though it carried ordinary headers and
    /// an oversized body.
    #[test]
    fn a_request_that_does_not_arrive_in_time_is_408_not_431() {
        let silent = status_of(Duration::from_millis(300), |_| {
            std::thread::sleep(Duration::from_millis(600));
        });
        assert!(silent.starts_with("408 "), "{silent}");
        let half = status_of(Duration::from_millis(300), |c| {
            c.write_all(b"POST /api/act HTTP/1.1\r\nHost: h\r\n")
                .expect("half");
            std::thread::sleep(Duration::from_millis(600));
        });
        assert!(half.starts_with("408 "), "{half}");
    }

    /// The control on the other side: the size bounds still answer by size.
    /// Headers over the bound are 431 however fast they come, and ordinary
    /// headers with an oversized body are 413 — whether the body arrives
    /// in the same write as the headers or after a pause longer than one
    /// read's timeout.
    #[test]
    fn the_size_bounds_still_answer_by_size() {
        let within = Duration::from_secs(5);
        let big = status_of(within, |c| {
            let x = "a".repeat(9000);
            let _ =
                c.write_all(format!("GET / HTTP/1.1\r\nHost: h\r\nX-Big: {x}\r\n\r\n").as_bytes());
        });
        assert!(big.starts_with("431 "), "{big}");
        let body = "a".repeat(8000);
        let head = format!(
            "POST /api/act HTTP/1.1\r\nHost: h\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        let (h1, b1) = (head.clone(), body.clone());
        let together = status_of(within, move |c| {
            let _ = c.write_all(format!("{h1}{b1}").as_bytes());
        });
        assert!(together.starts_with("413 "), "{together}");
        let paused = status_of(within, move |c| {
            std::thread::sleep(Duration::from_millis(300));
            let _ = c.write_all(head.as_bytes());
            std::thread::sleep(Duration::from_millis(300));
            let _ = c.write_all(body.as_bytes());
        });
        assert!(paused.starts_with("413 "), "{paused}");
    }

    #[test]
    fn a_refusal_reaches_a_client_that_reads_late() {
        let got = refuse_then_read_late(true).expect("no reset");
        assert!(got.starts_with("HTTP/1.1 431 "), "{got}");
        assert!(got.ends_with("request limit exceeded"), "{got}");
    }

    /// The control: the same refusal without the drain is lost to a reset,
    /// which is what curl reported as status 000.
    #[cfg(target_os = "linux")]
    #[test]
    fn without_the_drain_the_refusal_is_lost_to_a_reset() {
        let err = refuse_then_read_late(false).expect_err("the kernel resets");
        assert_eq!(err.kind(), std::io::ErrorKind::ConnectionReset, "{err}");
    }

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
    fn a_ticket_act_names_act_and_target_and_nothing_else() {
        assert!(
            serde_json::from_str::<TicketBody>(r#"{"act":"claim","target":"t-3f2a/i-9c01"}"#)
                .is_ok()
        );
        assert!(
            serde_json::from_str::<TicketBody>(
                r#"{"act":"done","target":"i-9c01","note":"shipped"}"#
            )
            .is_ok()
        );
        // Refusals: another act, an argv, a field it does not know.
        assert!(serde_json::from_str::<TicketBody>(r#"{"act":"sign","target":"t-3f2a"}"#).is_err());
        assert!(
            serde_json::from_str::<TicketBody>(
                r#"{"act":"claim","target":"t-3f2a","argv":["rm"]}"#
            )
            .is_err()
        );
        assert!(ticket_target_ok("t-3f2a"));
        assert!(ticket_target_ok("t-3f2a/i-9c01"));
        assert!(!ticket_target_ok("OW-WAR-0001"));
        assert!(!ticket_target_ok("t-3f2a/i-9c01/x"));
        assert!(!ticket_target_ok("t-../../etc"));
        assert!(!ticket_target_ok("t-"));
        assert!(!ticket_target_ok(""));
    }

    #[test]
    fn an_act_body_names_an_id_and_nothing_else() {
        assert!(serde_json::from_str::<ActBody>(r#"{"id":"sign-0"}"#).is_ok());
        assert!(serde_json::from_str::<ActBody>(r#"{"id":"x","argv":["rm"]}"#).is_err());
        assert!(serde_json::from_str::<ActBody>(r#"{"argv":["sign"]}"#).is_err());
    }
}
