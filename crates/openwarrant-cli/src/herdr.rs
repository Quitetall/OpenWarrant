// SPDX-License-Identifier: Apache-2.0

//! Lightweight IPC client for the local `herdr` workspace daemon.
//!
//! Connects over `~/.config/herdr/herdr.sock` to inspect running AI agent
//! sessions, stream terminal output, and dispatch prompts or tabs.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Default socket path for the local Herdr daemon.
#[must_use]
pub fn socket_path() -> PathBuf {
    if let Ok(dir) = std::env::var("HERDR_CONFIG_DIR") {
        return PathBuf::from(dir).join("herdr.sock");
    }
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(config_home).join("herdr").join("herdr.sock");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("herdr")
            .join("herdr.sock");
    }
    PathBuf::from("/tmp/herdr.sock")
}

/// Check if the Herdr daemon socket is reachable.
#[must_use]
pub fn is_available() -> bool {
    let path = socket_path();
    if !path.exists() {
        return false;
    }
    #[cfg(unix)]
    {
        std::os::unix::net::UnixStream::connect(path).is_ok()
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// An active agent reported by Herdr.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HerdrAgent {
    pub pane_id: String,
    #[serde(default)]
    pub terminal_id: String,
    pub agent: String,
    #[serde(default)]
    pub terminal_title: String,
    #[serde(default)]
    pub terminal_title_stripped: String,
    #[serde(default)]
    pub agent_status: String,
    #[serde(default)]
    pub cwd: String,
    #[serde(default)]
    pub foreground_cwd: String,
    #[serde(default)]
    pub workspace_id: String,
    #[serde(default)]
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
        if clean.is_empty() {
            format!("{}: {}", self.agent, self.pane_id)
        } else {
            format!("[{}] {}", self.agent, clean)
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
    let agents_val = result.get("agents").ok_or("missing agents array")?;
    serde_json::from_value(agents_val.clone()).map_err(|e| e.to_string())
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

    #[test]
    fn socket_path_resolves() {
        let p = socket_path();
        assert!(p.ends_with("herdr.sock"));
    }
}
