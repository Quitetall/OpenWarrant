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
            set("XDG_CONFIG_HOME").map_or_else(
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

/// Create a new terminal tab in Herdr for an isolated task or worktree.
pub fn create_tab(cwd: &str, label: &str) -> Result<String, String> {
    let result = call_herdr(
        "tab.create",
        serde_json::json!({
            "cwd": cwd,
            "label": label,
            "focus": true
        }),
    )?;
    Ok(result
        .get("tab_id")
        .and_then(|t| t.as_str())
        .unwrap_or_default()
        .to_owned())
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
