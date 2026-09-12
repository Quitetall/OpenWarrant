// SPDX-License-Identifier: Apache-2.0

//! `war perform` (OW-WAR-0069 STAGE-003): an agent stage, started.
//!
//! `war run` covers a service stage: a registered gate, a receipt, a verdict.
//! An agent stage had no runner at all — a human compiled a Dispatch with `war
//! dispatch`, carried it to an agent by hand, and carried a Stage Submission
//! back. This is that walk, done by the tool: compile the Dispatch, hand it to
//! the configured performer on stdin, and ingest whatever it answers through
//! the same refusals `war submit` applies to a submission that arrived by post.
//!
//! What it does NOT do, and cannot:
//!
//! - **decide that the work is done.** §51.2 lets a performer ask to continue,
//!   be verified, block, amend or cancel. `war submit`'s
//!   `submission.self-completion` refusal is the one that matters here, and it
//!   is reused verbatim rather than re-implemented.
//! - **write a submission the performer did not send.** A performer that
//!   crashes, times out, or answers with something that is not a submission
//!   leaves the stage where it was: the Dispatch is on record, nothing claims
//!   the work happened.
//! - **run several performers at once.** `[perform] max_concurrent` defaults to
//!   1 and a larger value is refused, because nothing here contains a performer
//!   (no cgroups, no sandbox) and concurrency would mean several unbounded
//!   processes writing one tree. That is Q-006's recorded recommendation, and
//!   raising the number is a deliberate act once containment exists.
//!
//! Unlike the drafter, a performer is *expected* to write files — that is the
//! work — so there is no tree-delta refusal here. What it may not write is a
//! record of authority, and it has no key with which to try.

use std::io::{Read as _, Write as _};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use openwarrant_core::execution::StageDispatch;
use openwarrant_core::milestones::ExecutorKind;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const DISPATCHES_DIR: &str = "dispatches";
/// Bounded like the drafter's: a performer that writes a gigabyte to stderr
/// should not become a gigabyte in a record.
const STDERR_TAIL: usize = 64 * 1024;
/// A Stage Submission is a page of JSON. Anything past this is a performer in a
/// loop, and reading it to the end would be this process growing until the box
/// runs out of memory, so the read stops and the answer is refused.
const STDOUT_CAP: u64 = 8 * 1024 * 1024;
/// How long to wait for the performer's output after it has exited or been
/// killed. A performer that spawned its own children leaves them holding the
/// pipe, so a plain `join` here waits for THEM — and a runner whose whole point
/// is a deadline must not be the thing that hangs. Found by the fixture that
/// sleeps: its `sleep 600` survived the kill and held stdout open.
const READ_GRACE: Duration = Duration::from_secs(5);

/// What one performance produced, for the caller to report.
struct Performance {
    dispatch_id: String,
    /// The performer wrote past [`STDOUT_CAP`]; the read stopped and the answer
    /// is not a submission anyone will ingest.
    oversized: bool,
    /// Where the performer's answer was written, when it sent one.
    submission_path: Option<camino::Utf8PathBuf>,
    /// `None` when the performer was killed at the deadline.
    exit: Option<std::process::ExitStatus>,
    stderr_tail: String,
    seconds: u64,
}

/// `war perform <alias> <stage>`: compile the Dispatch, hand it over, ingest.
pub fn run(repo: &Repository, alias: &str, stage_id: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let graph = loaded
        .basis
        .as_ref()
        .and_then(|b| {
            b.atoms
                .iter()
                .filter(|a| a.role == "milestones")
                .find_map(|a| {
                    std::str::from_utf8(&a.bytes)
                        .ok()
                        .and_then(|t| openwarrant_core::milestones::parse(t).ok())
                })
        })
        .ok_or_else(|| RepoError::Message(format!("{alias}: no milestones atom parses")))?;
    let Some(stage) = graph.stages.iter().find(|s| s.id == stage_id) else {
        return Err(RepoError::Message(format!(
            "{alias}: no stage {stage_id:?}; declared: {}",
            graph
                .stages
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    };
    let at = repo.relative(&dir.join("atoms/45-milestones.yaml"));
    match stage.executor_kind {
        ExecutorKind::Agent => {}
        ExecutorKind::Human => {
            report.push(Diagnostic::error(
                "perform.human-stage",
                at,
                format!(
                    "{alias}/{stage_id} is a human stage; a human does it. Nothing a tool spawns \
                     can stand in for a person (§27.2)"
                ),
            ));
            return Ok(report);
        }
        other => {
            report.push(Diagnostic::error(
                "perform.not-an-agent",
                at,
                format!(
                    "{alias}/{stage_id}: executor_kind is {other}; `war perform` performs agent \
                     stages. A service stage is run by `war run`, which mints its receipt"
                ),
            ));
            return Ok(report);
        }
    }
    let policy = &repo.config.perform;
    if policy.performer_argv.is_empty() {
        report.push(Diagnostic::error(
            "perform.no-performer",
            "openwarrant.toml".to_owned(),
            "no performer is configured: set [perform] performer_argv, or compile the Dispatch \
             with `war dispatch` and hand it to an agent yourself"
                .to_owned(),
        ));
        return Ok(report);
    }
    if policy.concurrency() > 1 {
        report.push(Diagnostic::error(
            "perform.no-containment",
            "openwarrant.toml".to_owned(),
            format!(
                "[perform] max_concurrent is {}; 1.0 performs one stage at a time. Nothing here \
                 contains a performer — no cgroups, no sandbox — so several at once would be \
                 several unbounded processes writing one tree (OW-WAR-0069 Q-006)",
                policy.concurrency()
            ),
        ));
        return Ok(report);
    }

    let dispatch = match compile(repo, alias, stage_id, &dir, &mut report)? {
        Some(d) => d,
        None => return Ok(report),
    };
    // The tighter of the two bounds, as `war run` does for a gate: a stage that
    // declares 60s does not get 600 because the config says so, and a config
    // that says 2s is not widened by a stage that declares none.
    let configured = (policy.performer_timeout_secs != 0).then_some(policy.performer_timeout_secs);
    let bound = match (configured, stage.wall_time_seconds) {
        (Some(c), Some(w)) => c.min(w),
        (Some(c), None) => c,
        (None, Some(w)) => w,
        (None, None) => repo.config.run.wall_time_seconds(),
    };
    let outcome = hand_over(repo, &dir, &dispatch, bound)?;

    match (&outcome.exit, &outcome.submission_path) {
        (Some(status), Some(path)) if status.success() => {
            report.push(Diagnostic::pass(
                "perform.answered",
                format!(
                    "{alias}/{stage_id}: the performer answered dispatch {} in {}s",
                    outcome.dispatch_id, outcome.seconds
                ),
            ));
            // The same ingest an external submission gets: §51.2 first, the
            // dispatch-id witness after. A performer this tool spawned earns no
            // shortcut past either.
            let ingest = crate::run_cmd::submit(repo, alias, path)?;
            let accepted = ingest.is_ready();
            for d in ingest.diagnostics {
                report.push(d);
            }
            for n in ingest.notes {
                report.note(n);
            }
            // A refused answer leaves no file behind: the seam's refusal says
            // nothing was written, and that has to be true of the raw answer
            // too, or the next reader finds a submission nothing accepted.
            let _ = std::fs::remove_file(path);
            if !accepted {
                report.note(format!(
                    "the performer's answer was discarded; dispatch {} stands unanswered.",
                    outcome.dispatch_id
                ));
            }
        }
        // Killed for flooding is a failure with its own sentence, not a
        // deadline: the bound was never reached.
        (None, _) if outcome.oversized => {
            discard(&outcome);
            report.push(Diagnostic::error(
                "perform.failed",
                outcome.dispatch_id.clone(),
                format!(
                    "{alias}/{stage_id}: the performer wrote past {STDOUT_CAP} bytes on stdout, so \
                     the read stopped and it was killed; a Stage Submission is a page of JSON. The \
                     stage is where it was{}",
                    tail(&outcome.stderr_tail)
                ),
            ));
        }
        (None, _) => {
            discard(&outcome);
            report.push(Diagnostic::error(
            "perform.timeout",
            outcome.dispatch_id.clone(),
            format!(
                "{alias}/{stage_id}: the performer did not finish within {bound}s and was killed; \
                 the Dispatch is on record and nothing claims the work happened{}",
                tail(&outcome.stderr_tail)
            ),
            ));
        }
        (Some(status), _) => {
            discard(&outcome);
            report.push(Diagnostic::error(
                "perform.failed",
                outcome.dispatch_id.clone(),
                format!(
                    "{alias}/{stage_id}: the performer exited {status} without a usable \
                     submission{}; the stage is where it was{}",
                    if outcome.oversized {
                        format!(" (it wrote past {STDOUT_CAP} bytes and the read stopped)")
                    } else {
                        String::new()
                    },
                    tail(&outcome.stderr_tail)
                ),
            ));
        }
    }
    Ok(report)
}

/// Delete the performer's raw answer. It is scratch on every path: an answer
/// nothing accepted must not be left in the Warrant's directory looking like a
/// record of one.
fn discard(outcome: &Performance) {
    if let Some(p) = &outcome.submission_path {
        let _ = std::fs::remove_file(p);
    }
}

/// `war perform --all`: every open agent stage, one at a time.
pub fn all(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let (_, frontier) = crate::frontier::run(repo, None)?;
    let open: Vec<(String, String)> = frontier
        .rows
        .iter()
        .filter(|r| r.state == crate::frontier::StageState::Open && r.executor_kind == "agent")
        .map(|r| (r.warrant.clone(), r.stage.clone()))
        .collect();
    if open.is_empty() {
        report.push(Diagnostic::pass(
            "perform.nothing-open",
            "no agent stage is open: every one is claimed, done, or waiting on a milestone"
                .to_owned(),
        ));
        return Ok(report);
    }
    report.note(format!(
        "{} open agent stage(s), performed one at a time ([perform] max_concurrent).",
        open.len()
    ));
    for (alias, stage) in open {
        let one = run(repo, &alias, &stage)?;
        for d in one.diagnostics {
            report.push(d);
        }
    }
    Ok(report)
}

/// Compile the stage's Dispatch and keep it under `dispatches/` as the record
/// of what was handed over. `None` when it will not compile — the diagnostics
/// say why, and nothing was spawned.
fn compile(
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    dir: &camino::Utf8Path,
    report: &mut Report,
) -> Result<Option<StageDispatch>, RepoError> {
    let dispatches = dir.join(DISPATCHES_DIR);
    std::fs::create_dir_all(&dispatches).map_err(|source| RepoError::Io {
        context: format!("could not create {dispatches}"),
        source,
    })?;
    let scratch = dispatches.join(format!("{stage_id}.pending.json"));
    let compiled = crate::dispatch::run(
        repo,
        alias,
        stage_id,
        openwarrant_core::execution::AttemptKind::Initial,
        &[],
        Some(&scratch),
        None,
    )?;
    if !compiled.is_ready() {
        let _ = std::fs::remove_file(&scratch);
        for d in compiled.diagnostics {
            report.push(d);
        }
        return Ok(None);
    }
    let dispatch: StageDispatch = std::fs::read_to_string(&scratch)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or_else(|| {
            RepoError::Message(format!(
                "{alias}/{stage_id}: the compiled dispatch could not be read back"
            ))
        })?;
    let path = dispatches.join(format!("{}.json", dispatch.dispatch_id));
    std::fs::rename(&scratch, &path).map_err(|source| RepoError::Io {
        context: format!("could not move {scratch} to {path}"),
        source,
    })?;
    Ok(Some(dispatch))
}

/// Spawn the performer with the Dispatch on stdin, under a wall-clock bound.
///
/// Its stdout is written verbatim to `dispatches/answer-<dispatch>.json` and
/// ingested from there, so what the tool validated is a file a reader can open
/// rather than a string that lived only in this process. It is deliberately NOT
/// written under `submissions/`: that directory holds submissions the seam
/// accepted, and a rejected answer sitting beside them would read as a record.
fn hand_over(
    repo: &Repository,
    dir: &camino::Utf8Path,
    dispatch: &StageDispatch,
    bound: u64,
) -> Result<Performance, RepoError> {
    let policy = &repo.config.perform;
    let program = policy.performer_argv[0].clone();
    let started = Instant::now();
    let mut command = Command::new(&program);
    command
        .args(&policy.performer_argv[1..])
        .current_dir(&repo.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Its own process group, so the deadline reaches the children a performer
    // spawns. Killing only the process it launched leaves its `sleep` running.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command.spawn().map_err(|source| RepoError::Io {
        context: format!("could not run the performer {program}"),
        source,
    })?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        let body = serde_json::to_string_pretty(dispatch)
            .map_err(|e| RepoError::Message(format!("could not render the dispatch: {e}")))?;
        // A performer that exits before reading is reported by its status, not
        // by a broken pipe here.
        let _ = stdin.write_all(body.as_bytes());
    }
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let (tx, rx) = std::sync::mpsc::channel();
    // Set the moment the performer writes past the cap, so the wait below ends
    // it then rather than at the deadline: a performer that flooded stdout has
    // already failed, and waiting ten minutes to say so wastes ten minutes.
    let capped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&capped);
    std::thread::spawn(move || {
        // `take`, not `read_to_string`: the cap is the whole point.
        let mut out = String::new();
        let _ = stdout
            .by_ref()
            .take(STDOUT_CAP + 1)
            .read_to_string(&mut out);
        if out.len() as u64 > STDOUT_CAP {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        let _ = tx.send((out, err));
    });
    let deadline = Duration::from_secs(bound);
    let exit = loop {
        if let Some(s) = child.try_wait().map_err(|source| RepoError::Io {
            context: "could not wait for the performer".to_owned(),
            source,
        })? {
            break Some(s);
        }
        if capped.load(std::sync::atomic::Ordering::Acquire) || started.elapsed() > deadline {
            kill_group(&mut child);
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // Bounded, for the reason READ_GRACE gives. Output that has not arrived by
    // now is output this run does not have, and the report says the performer
    // answered nothing rather than waiting on a grandchild to close a pipe.
    let (out, err) = rx.recv_timeout(READ_GRACE).unwrap_or_default();
    let tail_start = err.len().saturating_sub(STDERR_TAIL);
    let oversized =
        out.len() as u64 > STDOUT_CAP || capped.load(std::sync::atomic::Ordering::Acquire);
    let submission_path = if out.trim().is_empty() || oversized {
        None
    } else {
        let scratch = dir.join(DISPATCHES_DIR);
        std::fs::create_dir_all(&scratch).map_err(|source| RepoError::Io {
            context: format!("could not create {scratch}"),
            source,
        })?;
        let path = scratch.join(format!("answer-{}.json", dispatch.dispatch_id));
        std::fs::write(&path, out.as_bytes()).map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
        Some(path)
    };
    Ok(Performance {
        dispatch_id: dispatch.dispatch_id.clone(),
        oversized,
        submission_path,
        exit,
        stderr_tail: String::from_utf8_lossy(&err[tail_start..]).into_owned(),
        seconds: started.elapsed().as_secs(),
    })
}

/// SIGKILL the performer and everything it started.
///
/// On unix the child leads its own process group (see the spawn above), so the
/// negated pid reaches the group. `kill` is spawned rather than linking libc:
/// this crate has no libc dependency and a signal is not worth one.
fn kill_group(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("kill")
            .args(["-KILL", &group])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    // Belt and braces, and the whole story on a platform without groups.
    let _ = child.kill();
    // Reap without blocking: SIGKILL is not blockable, but a child in
    // uninterruptible sleep takes it only once its I/O completes.
    let until = Instant::now() + READ_GRACE;
    while Instant::now() < until {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => break,
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn tail(stderr: &str) -> String {
    let last = stderr.lines().rfind(|l| !l.trim().is_empty());
    last.map_or_else(String::new, |l| format!(". Its last stderr line: {l}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stderr_note_is_the_last_line_or_nothing() {
        assert_eq!(tail(""), "");
        assert_eq!(tail("   \n\n"), "");
        assert_eq!(
            tail("starting\nreading the dispatch\nno model configured\n"),
            ". Its last stderr line: no model configured"
        );
    }

    /// One at a time, until something contains a performer. The number is a
    /// configuration surface so the decision is visible, not a constant.
    #[test]
    fn concurrency_defaults_to_one() {
        let p = openwarrant_core::config::PerformPolicy::default();
        assert_eq!(p.concurrency(), 1);
        assert!(p.is_empty());
        let two = openwarrant_core::config::PerformPolicy {
            max_concurrent: 2,
            ..Default::default()
        };
        assert_eq!(two.concurrency(), 2, "the refusal reports what was asked");
        assert!(!two.is_empty());
    }
}
