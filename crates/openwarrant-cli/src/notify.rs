// SPDX-License-Identifier: Apache-2.0
//! `[notify]`: tell a person something waits on them (OW-WAR-0148 M14,
//! decision 8).
//!
//! Off unless `openwarrant.toml` names a command:
//!
//! ```toml
//! [notify]
//! argv = ["notify-send", "OpenWarrant", "{message}"]
//! timeout_secs = 10
//! ```
//!
//! It runs when a command hands a person an act: an approval or a release
//! batch requested, an authorization or resolution request emitted, a PR the
//! gate holds for an approval. Each element of `argv` has `{event}`,
//! `{subject}`, `{message}` and `{command}` filled in, and the same four ride
//! in the environment as `OPENWARRANT_NOTIFY_EVENT`, `..._SUBJECT`,
//! `..._MESSAGE` and `..._COMMAND`. It is an argv, never a shell string.
//!
//! A notify that fails, times out or cannot start is reported
//! (`notify.failed`, a warning) and never blocks: the act it announced is
//! already written, and the exit code is the command's own.

use std::time::Duration;

use camino::Utf8Path;

use crate::diagnostic::Diagnostic;
use crate::plan::intake::{Ran, first_line, run_bounded_env};
use crate::preset::{NotifyPolicy, Policy};

/// Something waiting on a person.
#[derive(Debug, Clone, Copy)]
pub struct Wait<'a> {
    /// A short name: `approval.requested`, `release.requested`, ...
    pub event: &'a str,
    /// What it is about: a Warrant id, a tag, a PR number.
    pub subject: &'a str,
    /// One line for the person.
    pub message: &'a str,
    /// The command the person runs.
    pub command: &'a str,
}

/// Default bound on a notify command.
const DEFAULT_TIMEOUT: u64 = 10;

/// Run `[notify]` for `wait`, when configured. `None` when it is off (or
/// the configuration cannot be read, which `war check` reports); otherwise
/// a pass (`notify.sent`) or a warning (`notify.failed`), never an error.
#[must_use]
pub fn human_waits(root: &Utf8Path, wait: &Wait<'_>) -> Option<Diagnostic> {
    let policy = Policy::read(root).ok()?;
    let notify = policy.notify?;
    Some(run(root, &notify, wait))
}

/// [`human_waits`] with the policy in hand.
#[must_use]
pub fn run(root: &Utf8Path, notify: &NotifyPolicy, wait: &Wait<'_>) -> Diagnostic {
    let fill = |a: &str| {
        a.replace("{event}", wait.event)
            .replace("{subject}", wait.subject)
            .replace("{message}", wait.message)
            .replace("{command}", wait.command)
    };
    let argv: Vec<String> = notify.argv.iter().map(|a| fill(a)).collect();
    let deadline = Duration::from_secs(if notify.timeout_secs == 0 {
        DEFAULT_TIMEOUT
    } else {
        notify.timeout_secs
    });
    // The four values also ride in the environment, for a script that
    // would rather read them there.
    let vars = [
        ("OPENWARRANT_NOTIFY_EVENT", wait.event),
        ("OPENWARRANT_NOTIFY_SUBJECT", wait.subject),
        ("OPENWARRANT_NOTIFY_MESSAGE", wait.message),
        ("OPENWARRANT_NOTIFY_COMMAND", wait.command),
    ];
    let failed = |why: String| {
        Diagnostic::warn(
            "notify.failed",
            "openwarrant.toml",
            format!(
                "[notify] for {} ({}) did not complete: {why}. Nothing else changes: what it \
                 announced stands, and `war next` lists what waits on a person",
                wait.event, wait.subject
            ),
        )
    };
    match run_bounded_env(root, &argv, &vars, deadline) {
        Ok(Ran::Exited(status, _, _)) if status.success() => Diagnostic::pass(
            "notify.sent",
            format!("[notify] ran for {} ({})", wait.event, wait.subject),
        ),
        Ok(Ran::Exited(status, _, err)) => {
            failed(format!("{} exited {status}: {}", argv[0], first_line(&err)))
        }
        Ok(Ran::TimedOut) => failed(format!(
            "{} ran past {}s and was stopped",
            argv[0],
            deadline.as_secs()
        )),
        Err((what, e)) => failed(format!("{what}: {e}")),
    }
}

/// Print a notify outcome for a person, on stderr: the command's own output
/// on stdout stays exactly what it was.
pub fn say(d: Option<&Diagnostic>) {
    if let Some(d) = d
        && d.severity == crate::diagnostic::Severity::Warn
    {
        eprintln!("warning ({}): {}", d.rule, d.message);
    }
}
