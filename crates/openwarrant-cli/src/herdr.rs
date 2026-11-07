// SPDX-License-Identifier: Apache-2.0

//! Lightweight IPC client for the local `herdr` workspace daemon: an optional
//! adapter, and everything that uses it works without it.
//!
//! One request per connection over the Unix socket, one JSON object per line:
//! `{"id", "method", "params"}` answered by `{"id", "result"}` or
//! `{"id", "error"}`. It is not JSON-RPC. The socket is the one `herdr` itself
//! would use: `HERDR_SOCKET_PATH`, else the session `HERDR_SESSION` names, else
//! the default session's under the configuration directory.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize};

/// `herdr`'s own rule for a session name, so a name from the environment
/// cannot walk out of the sessions directory.
fn session_name_is_valid(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// The socket `env` names; `env` is a parameter so the order can be tested
/// without touching the process's environment.
fn socket_path_from(env: impl Fn(&str) -> Option<String>) -> PathBuf {
    let set = |name: &str| env(name).filter(|v| !v.is_empty());
    if let Some(path) = set("HERDR_SOCKET_PATH") {
        return PathBuf::from(path);
    }
    let config = set("HERDR_CONFIG_DIR").map_or_else(
        || {
            set("XDG_CONFIG_HOME")
                .map_or_else(
                    || {
                        set("HOME").map_or_else(
                            || PathBuf::from("/tmp"),
                            |home| Path::new(&home).join(".config"),
                        )
                    },
                    PathBuf::from,
                )
                .join("herdr")
        },
        PathBuf::from,
    );
    match set("HERDR_SESSION") {
        Some(name) if name != "default" && session_name_is_valid(&name) => {
            config.join("sessions").join(name).join("herdr.sock")
        }
        _ => config.join("herdr.sock"),
    }
}

/// The socket of the Herdr session this process belongs to.
#[must_use]
pub fn socket_path() -> PathBuf {
    socket_path_from(|name| std::env::var(name).ok())
}

/// What `ping` answers: the daemon is there, and which one it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub version: String,
    #[serde(default)]
    pub protocol: u64,
}

/// `ping`. An `Err` says why there is no Herdr, for a reader to show.
pub fn ping() -> Result<Pong, String> {
    let result = call_herdr("ping", serde_json::json!({}))?;
    serde_json::from_value(result).map_err(|e| format!("unexpected ping answer: {e}"))
}

/// A field the API may send as `null`; absent and `null` both read as empty.
fn nullable<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// An active agent reported by Herdr. Every string but its ids is nullable in
/// Herdr's schema: a pane whose agent is not recognized yet has no `agent`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HerdrAgent {
    pub pane_id: String,
    #[serde(default, deserialize_with = "nullable")]
    pub terminal_id: String,
    #[serde(default, deserialize_with = "nullable")]
    pub agent: String,
    #[serde(default, deserialize_with = "nullable")]
    pub name: String,
    #[serde(default, deserialize_with = "nullable")]
    pub terminal_title: String,
    #[serde(default, deserialize_with = "nullable")]
    pub terminal_title_stripped: String,
    #[serde(default, deserialize_with = "nullable")]
    pub agent_status: String,
    #[serde(default, deserialize_with = "nullable")]
    pub cwd: String,
    #[serde(default, deserialize_with = "nullable")]
    pub foreground_cwd: String,
    #[serde(default, deserialize_with = "nullable")]
    pub workspace_id: String,
    #[serde(default, deserialize_with = "nullable")]
    pub tab_id: String,
    #[serde(default)]
    pub focused: bool,
}

impl HerdrAgent {
    /// Human-friendly display label combining model/agent and title.
    #[must_use]
    pub fn display_label(&self) -> String {
        let title = if self.terminal_title_stripped.is_empty() {
            &self.terminal_title
        } else {
            &self.terminal_title_stripped
        };
        let clean = title.trim_start_matches("✳ ").trim();
        let who = if self.name.is_empty() {
            if self.agent.is_empty() {
                "agent"
            } else {
                self.agent.as_str()
            }
        } else {
            self.name.as_str()
        };
        if clean.is_empty() {
            format!("{who}: {}", self.pane_id)
        } else {
            format!("[{who}] {clean}")
        }
    }

    /// Where it works: what its foreground process runs in, else its pane's
    /// directory. Empty when Herdr knows neither.
    #[must_use]
    pub fn workdir(&self) -> &str {
        if self.foreground_cwd.is_empty() {
            &self.cwd
        } else {
            &self.foreground_cwd
        }
    }

    /// Status badge for Ratatui list rendering.
    #[must_use]
    pub fn status_badge(&self) -> &'static str {
        match self.agent_status.as_str() {
            "working" => "● BUSY",
            "idle" => "○ IDLE",
            "blocked" => "▲ BLOCKED",
            "done" => "✔ DONE",
            _ => "? UNKNOWN",
        }
    }
}

/// Low-level RPC dispatch to the Herdr Unix domain socket.
#[cfg(unix)]
fn call_herdr(method: &str, params: serde_json::Value) -> Result<serde_json::Value, String> {
    use std::os::unix::net::UnixStream;

    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .map_err(|e| format!("could not connect to herdr at {}: {e}", path.display()))?;

    stream
        .set_read_timeout(Some(Duration::from_millis(1500)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1000)))
        .map_err(|e| e.to_string())?;

    let req = serde_json::json!({
        "id": "ow-tui",
        "method": method,
        "params": params,
    });

    let mut payload = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    payload.push(b'\n');
    stream.write_all(&payload).map_err(|e| e.to_string())?;

    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if chunk[..n].contains(&b'\n') {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && !buf.is_empty() => break,
            Err(e) => return Err(format!("error reading from herdr: {e}")),
        }
    }

    let val: serde_json::Value =
        serde_json::from_slice(&buf).map_err(|e| format!("invalid JSON from herdr: {e}"))?;

    if let Some(err) = val.get("error") {
        return Err(err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("herdr error")
            .to_owned());
    }

    Ok(val
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

#[cfg(not(unix))]
fn call_herdr(_method: &str, _params: serde_json::Value) -> Result<serde_json::Value, String> {
    Err("herdr IPC is supported on Unix only".to_owned())
}

/// What can change the fleet: an agent's status, a pane coming or going. Herdr
/// has no event for new output, so a working agent's terminal is still read.
const EVENTS: &[&str] = &[
    "pane.created",
    "pane.closed",
    "pane.exited",
    "pane.updated",
    "pane.agent_detected",
];

/// Hold an `events.subscribe` connection open. `event` runs for each event and
/// `alive` about once a second; a false from it ends the subscription. Returns
/// when the daemon goes away or says no.
#[cfg(unix)]
pub fn subscribe(mut event: impl FnMut(), alive: impl Fn() -> bool) -> Result<(), String> {
    use std::io::ErrorKind;
    use std::os::unix::net::UnixStream;

    let mut stream = UnixStream::connect(socket_path()).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|e| e.to_string())?;
    let subscriptions: Vec<_> = EVENTS
        .iter()
        .map(|t| serde_json::json!({"type": t}))
        .collect();
    let req = serde_json::json!({
        "id": "ow-tui-events",
        "method": "events.subscribe",
        "params": {"subscriptions": subscriptions},
    });
    let mut payload = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    payload.push(b'\n');
    stream.write_all(&payload).map_err(|e| e.to_string())?;
    let mut pending = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut started = false;
    while alive() {
        match stream.read(&mut chunk) {
            Ok(0) => return Err("herdr closed the subscription".to_owned()),
            Ok(n) => pending.extend_from_slice(&chunk[..n]),
            Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => continue,
            Err(e) => return Err(e.to_string()),
        }
        while let Some(end) = pending.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = pending.drain(..=end).collect();
            if started {
                event();
                continue;
            }
            let v: serde_json::Value =
                serde_json::from_slice(&line).map_err(|e| format!("invalid JSON: {e}"))?;
            if v.get("error").is_some() {
                return Err(v["error"]["message"]
                    .as_str()
                    .unwrap_or("refused")
                    .to_owned());
            }
            started = true;
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn subscribe(_event: impl FnMut(), _alive: impl Fn() -> bool) -> Result<(), String> {
    Err("herdr IPC is supported on Unix only".to_owned())
}

/// Query active agent fleet across all workspaces from Herdr.
pub fn list_agents() -> Result<Vec<HerdrAgent>, String> {
    let result = call_herdr("agent.list", serde_json::json!({}))?;
    agents_in(&result)
}

/// The agents of an `agent_list` answer. An entry that cannot be read is an
/// error, not a shorter list: a fleet count must not quietly lose an agent.
fn agents_in(result: &serde_json::Value) -> Result<Vec<HerdrAgent>, String> {
    let agents = result.get("agents").ok_or("missing agents array")?;
    serde_json::from_value(agents.clone()).map_err(|e| format!("unexpected agent list: {e}"))
}

/// Read the recent terminal output / transcript of an agent pane.
pub fn read_agent(target: &str, lines: usize) -> Result<String, String> {
    let result = call_herdr(
        "agent.read",
        serde_json::json!({
            "target": target,
            "lines": lines,
            "format": "text",
            "source": "recent"
        }),
    )?;
    Ok(result
        .get("read")
        .and_then(|r| r.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_owned())
}

/// Send a prompt or answer directly to an agent pane.
pub fn prompt_agent(target: &str, text: &str) -> Result<(), String> {
    call_herdr(
        "agent.prompt",
        serde_json::json!({
            "target": target,
            "text": text
        }),
    )?;
    Ok(())
}

/// Create a tab for an isolated task or worktree. Returns its root pane, the
/// shell `agent.start` runs in.
pub fn create_tab(cwd: &str, label: &str, env: &[(&str, &str)]) -> Result<String, String> {
    let env: serde_json::Map<String, serde_json::Value> = env
        .iter()
        .map(|(k, v)| ((*k).to_owned(), serde_json::json!(v)))
        .collect();
    let result = call_herdr(
        "tab.create",
        serde_json::json!({"cwd": cwd, "label": label, "env": env, "focus": false}),
    )?;
    root_pane_in(&result)
}

fn root_pane_in(result: &serde_json::Value) -> Result<String, String> {
    result
        .get("root_pane")
        .and_then(|p| p.get("pane_id"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "tab.create answered with no root pane".to_owned())
}

/// How long a new tab's shell gets to come up.
const SHELL_TRIES: u32 = 25;
const SHELL_WAIT: Duration = Duration::from_millis(200);

/// Start an agent of `kind` in a shell pane. Herdr waits until it is up. A
/// tab's shell is not there the moment the tab is: until it is, Herdr answers
/// `agent_pane_busy`, "not an available shell", and that alone is retried.
pub fn start_agent(name: &str, kind: &str, pane: &str, args: &[String]) -> Result<(), String> {
    let params = serde_json::json!({"name": name, "kind": kind, "pane_id": pane, "args": args});
    let mut tries = SHELL_TRIES;
    loop {
        match call_herdr("agent.start", params.clone()) {
            Err(why) if tries > 0 && why.contains("not an available shell") => {
                tries -= 1;
                std::thread::sleep(SHELL_WAIT);
            }
            Err(why) => return Err(why),
            Ok(_) => return Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |name| {
            vars.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| (*v).to_owned())
        }
    }

    #[test]
    fn the_socket_is_the_one_herdr_would_use() {
        let path = |vars: &[(&str, &str)]| socket_path_from(env(vars));
        assert_eq!(
            path(&[("HERDR_SOCKET_PATH", "/run/x.sock"), ("HOME", "/h")]),
            PathBuf::from("/run/x.sock")
        );
        assert_eq!(
            path(&[("HOME", "/h")]),
            PathBuf::from("/h/.config/herdr/herdr.sock")
        );
        assert_eq!(
            path(&[("XDG_CONFIG_HOME", "/c"), ("HOME", "/h")]),
            PathBuf::from("/c/herdr/herdr.sock")
        );
        // A named session has its own directory, as `herdr pane list` connects.
        assert_eq!(
            path(&[("HERDR_SESSION", "repro-1"), ("HOME", "/h")]),
            PathBuf::from("/h/.config/herdr/sessions/repro-1/herdr.sock")
        );
        assert_eq!(
            path(&[("HERDR_SESSION", "default"), ("HOME", "/h")]),
            PathBuf::from("/h/.config/herdr/herdr.sock")
        );
    }

    /// Refusal: a session name is not a path.
    #[test]
    fn a_session_name_cannot_leave_the_sessions_directory() {
        let path = socket_path_from(env(&[("HERDR_SESSION", "../../etc"), ("HOME", "/h")]));
        assert_eq!(path, PathBuf::from("/h/.config/herdr/herdr.sock"));
    }

    #[test]
    fn a_null_field_reads_as_empty_and_the_label_still_says_something() {
        let answer = serde_json::json!({"agents": [{
            "pane_id": "w1:p1", "terminal_id": "term_1", "agent": null, "name": null,
            "terminal_title": null, "terminal_title_stripped": null, "agent_status": "unknown",
            "cwd": null, "foreground_cwd": null, "workspace_id": "w1", "tab_id": "w1:t1",
            "focused": false, "revision": 3,
        }]});
        let agents = agents_in(&answer).expect("nulls are allowed");
        assert_eq!(agents[0].agent, "");
        assert_eq!(agents[0].display_label(), "agent: w1:p1");
        assert_eq!(agents[0].status_badge(), "? UNKNOWN");
        assert_eq!(agents[0].workdir(), "");
    }

    /// Refusal: an entry that is not an agent is an error, not a shorter list.
    #[test]
    fn an_unreadable_entry_is_an_error() {
        let answer = serde_json::json!({"agents": [{"pane_id": 7}]});
        assert!(agents_in(&answer).is_err());
        assert!(agents_in(&serde_json::json!({})).is_err());
    }

    #[test]
    fn a_new_tab_answers_with_its_root_pane() {
        let answer = serde_json::json!({"type": "tab_created", "tab": {"tab_id": "w1:t2"},
            "root_pane": {"pane_id": "w1:p9"}});
        assert_eq!(root_pane_in(&answer).as_deref(), Ok("w1:p9"));
        assert!(root_pane_in(&serde_json::json!({"tab": {}})).is_err());
    }

    #[test]
    fn a_label_prefers_the_agents_name_and_a_workdir_its_foreground() {
        let a: HerdrAgent = serde_json::from_value(serde_json::json!({
            "pane_id": "w1:p1", "agent": "claude", "name": "builder",
            "terminal_title_stripped": "Lamquant", "cwd": "/a", "foreground_cwd": "/b",
        }))
        .expect("an agent");
        assert_eq!(a.display_label(), "[builder] Lamquant");
        assert_eq!(a.workdir(), "/b");
    }
}
