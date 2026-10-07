// SPDX-License-Identifier: Apache-2.0

//! One line on stderr when a newer release exists (OW-WAR-0143).
//!
//! A self-updater only helps someone who thinks to run it. So, at the end of
//! an ordinary command, `war` may say — once, on stderr — that a newer
//! release is published and which command installs it. It never installs
//! anything, never writes to stdout, and never makes a command wait:
//!
//! - what it shows comes from a cache file, read locally;
//! - when that cache is older than a day, `war` starts itself as the hidden
//!   `war __release-check`, detached with null stdio, and does not wait. That
//!   process makes the one request (the releases list, five-second timeout)
//!   and rewrites the cache;
//! - it is off when `OPENWARRANT_NO_UPDATE_CHECK` or `CI` is set, under
//!   `--json`, when stderr is not a terminal, and for `update`, `version` and
//!   `mcp`.
//!
//! The request carries the request line, `host` and
//! `user-agent: openwarrant/<version>`. No repository path, alias, machine
//! identifier or cookie. The cache holds the check time, the channel, and the
//! newest tag or why there is none — nothing else.

use std::io::IsTerminal;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use camino::Utf8PathBuf;

use crate::install::{self, Channel};

/// Setting this to anything non-empty turns the notice, and its request, off.
pub const DISABLE_ENV: &str = "OPENWARRANT_NO_UPDATE_CHECK";

/// The hidden subcommand the refresh runs as.
pub const REFRESH_COMMAND: &str = "__release-check";

/// A day: at most one request per install per day.
const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// The refresh's whole budget for the network.
const TIMEOUT: Duration = Duration::from_secs(5);

/// `$XDG_CACHE_HOME/openwarrant/release-check.json`, or under `~/.cache`.
#[must_use]
pub fn cache_path() -> Utf8PathBuf {
    let base = match std::env::var("XDG_CACHE_HOME") {
        Ok(xdg) if !xdg.is_empty() => Utf8PathBuf::from(xdg),
        _ => Utf8PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache"),
    };
    base.join("openwarrant").join("release-check.json")
}

fn set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Whether the notice, and its refresh, may run for this invocation.
fn enabled(command: Option<&str>, json: bool) -> bool {
    !json
        && !set(DISABLE_ENV)
        && !set("CI")
        && !matches!(
            command,
            Some("update" | "version" | "mcp" | "host" | REFRESH_COMMAND)
        )
        && std::io::stderr().is_terminal()
}

#[derive(Debug)]
struct Cache {
    checked_at: u64,
    channel: String,
    newest: Option<String>,
}

fn read_cache() -> Option<Cache> {
    let text = std::fs::read_to_string(cache_path()).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(Cache {
        checked_at: value["checked_at"].as_u64()?,
        channel: value["channel"].as_str()?.to_owned(),
        newest: value["newest"].as_str().map(str::to_owned),
    })
}

fn write_cache(channel: Channel, outcome: Result<&str, &str>) {
    let path = cache_path();
    let mut value = serde_json::json!({
        "checked_at": now(),
        "channel": install::label(channel),
    });
    match outcome {
        Ok(tag) => value["newest"] = serde_json::json!(tag),
        Err(why) => value["unknown"] = serde_json::json!(why),
    }
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    // Write-then-rename: a reader never sees half a file.
    let tmp = path.with_extension(format!("json.{}", std::process::id()));
    if std::fs::write(&tmp, format!("{value}\n")).is_ok() && std::fs::rename(&tmp, &path).is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Called once, after a command has finished and printed everything it
/// prints. `command` is the subcommand's name (`None` for `war` alone).
pub fn after(command: Option<&str>, json: bool) {
    if !enabled(command, json) {
        return;
    }
    let channel = install::default_channel();
    let current = env!("CARGO_PKG_VERSION");
    let cache = read_cache();
    if let Some(cache) = &cache
        && cache.channel == install::label(channel)
        && let Some(tag) = &cache.newest
        && install::precedes(current, tag)
    {
        eprintln!(
            "war: {tag} is published (this is {current}). Update: {}",
            install::remedy(tag)
        );
    }
    let fresh = cache.as_ref().is_some_and(|c| {
        c.channel == install::label(channel)
            && now().saturating_sub(c.checked_at) < MAX_AGE.as_secs()
    });
    if fresh {
        return;
    }
    // Claim the day before starting the refresh, so a second command a moment
    // later does not start another: at most one request, even if it fails.
    write_cache(channel, Err("a check is in progress"));
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut refresh = std::process::Command::new(exe);
    refresh
        .arg(REFRESH_COMMAND)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Its own process group: a Ctrl-C at the prompt is not for it.
        refresh.process_group(0);
    }
    // Never waited on. The child outlives this process and writes the cache.
    let _ = refresh.spawn();
}

/// `war __release-check`: read the releases list once and record the answer.
/// Prints nothing; an unreachable list is recorded as UNKNOWN with its time,
/// so an offline machine tries again tomorrow, not on every command.
pub fn refresh() {
    let channel = install::default_channel();
    match install::newest(channel, TIMEOUT) {
        Ok(release) => write_cache(channel, Ok(&release.tag)),
        Err(why) => write_cache(channel, Err(&why)),
    }
}
