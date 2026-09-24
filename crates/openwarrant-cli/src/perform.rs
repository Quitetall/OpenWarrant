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
//!   crashes, times out, is cancelled, or answers with something that is not a
//!   submission leaves the stage where it was: the Dispatch is on record,
//!   nothing claims the work happened.
//! - **run several performers at once.** `[perform] max_concurrent` defaults to
//!   1 and a larger value is refused, because nothing here contains a performer
//!   (no cgroups, no sandbox) and concurrency would mean several unbounded
//!   processes writing one tree. That is Q-006's recorded recommendation, and
//!   raising the number is a deliberate act once containment exists.
//!
//! Unlike the drafter, a performer is *expected* to write files — that is the
//! work — so there is no tree-delta refusal here. What it may not write is a
//! record of authority, and it has no key with which to try.
//!
//! # One writer, cancelled cleanly (OW-WAR-0131)
//!
//! - **OS admission.** The bound on a performer is a kill of its whole process
//!   group. A platform without one is refused `perform.unsupported-os` before
//!   anything is compiled or spawned, rather than run under a weaker bound than
//!   the report would claim.
//! - **Writer record.** While a performer runs, `dispatches/<id>.writer` names
//!   its pid, its process group and the leader's start time. It is removed only
//!   once the group is shown gone.
//! - **Handoff.** Before compiling, every `.writer` for the stage is probed. A
//!   live group refuses `perform.writer-alive`; a probe that cannot tell
//!   refuses `perform.writer-unknown`; only a group shown gone (`ESRCH`) lets
//!   the next writer start. A record's age proves nothing and is not read.
//! - **Cancellation.** SIGINT and SIGTERM set a flag the wait loop reads; the
//!   group is killed and reaped, the answer is discarded, and the report is
//!   `perform.cancelled`. An interrupted run is not completion.
//!
//! What none of this covers is in `docs/PERFORM.md`: a process that leaves the
//! group (`setsid`, a daemon), SIGKILL to `war` itself, and a writer on
//! another machine.

use std::io::{Read as _, Write as _};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use openwarrant_core::execution::StageDispatch;
use openwarrant_core::milestones::ExecutorKind;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const DISPATCHES_DIR: &str = "dispatches";
/// The extension of a writer record, beside the Dispatch it performs.
const WRITER_EXT: &str = "writer";
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

/// Where a performer runs.
///
/// The seam exists so the refusal on a platform without the group kill can be
/// exercised on one that has it: a test hands in a host that says "no group
/// kill" and counts the spawns it was asked for, which must be zero.
trait Host {
    /// Whether a kill reaches the performer's whole process tree here.
    fn group_kill(&self) -> bool;
    fn spawn(&self, command: &mut Command) -> std::io::Result<Child>;
}

/// The platform this binary was compiled for.
struct Os;

impl Host for Os {
    fn group_kill(&self) -> bool {
        // Linux and macOS are the release targets; both are unix, and on unix
        // the performer leads its own process group. Anything else has no
        // group kill in this crate and is refused (§55.2: fail closed).
        cfg!(unix)
    }

    fn spawn(&self, command: &mut Command) -> std::io::Result<Child> {
        command.spawn()
    }
}

/// What one performance produced, for the caller to report.
struct Performance {
    dispatch_id: String,
    /// The performer wrote past [`STDOUT_CAP`]; the read stopped and the answer
    /// is not a submission anyone will ingest.
    oversized: bool,
    /// Where the performer's answer was written, when it sent one.
    submission_path: Option<camino::Utf8PathBuf>,
    /// `None` when the performer was killed at the deadline or cancelled.
    exit: Option<std::process::ExitStatus>,
    /// The signal that cancelled the run, when one did.
    cancelled: Option<&'static str>,
    /// The performer exited, but something in its group had not by the end of
    /// the grace: its writer record is kept, and the next `war perform` on the
    /// stage will see it.
    lingering: Option<camino::Utf8PathBuf>,
    stderr_tail: String,
    seconds: u64,
}

/// `war perform <alias> <stage>`: compile the Dispatch, hand it over, ingest.
pub fn run(
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    prototype: bool,
) -> Result<Report, RepoError> {
    run_on(&Os, repo, alias, stage_id, prototype)
}

fn run_on(
    host: &dyn Host,
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    prototype: bool,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if !admitted(host, &mut report) {
        return Ok(report);
    }
    perform_one(host, repo, alias, stage_id, prototype)
}

/// OS admission: refuse, before anything is compiled or spawned, a platform on
/// which the deadline and cancellation could not reach a performer's children.
fn admitted(host: &dyn Host, report: &mut Report) -> bool {
    if host.group_kill() {
        return true;
    }
    report.push(Diagnostic::error(
        "perform.unsupported-os",
        std::env::consts::OS.to_owned(),
        format!(
            "`war perform` bounds a performer by killing its whole process group, and this \
             platform ({}) has no group kill here: a performer's children would outlive its \
             deadline and a cancellation. Refused rather than run under a weaker bound than the \
             report would claim (§55.2). Linux and macOS are supported; compile the Dispatch with \
             `war dispatch` and hand it over yourself",
            std::env::consts::OS
        ),
    ));
    false
}

fn perform_one(
    host: &dyn Host,
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    prototype: bool,
) -> Result<Report, RepoError> {
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
    // Handoff, before anything is compiled: a stage whose previous writer may
    // still be writing gets no second one, and no Dispatch either.
    if !handoff(repo, &dir, alias, stage_id, &mut report) {
        return Ok(report);
    }

    // From here a SIGINT or SIGTERM is a cancellation, not a death: the flag is
    // read before spawning and by the wait loop, and the group is killed and
    // reaped before `war` exits. Before here, nothing has been started, so the
    // signal keeps its ordinary meaning.
    let _armed = cancel::arm()
        .map_err(|e| RepoError::Message(format!("could not install the signal handler: {e}")))?;

    let dispatch = match compile(repo, alias, stage_id, &dir, &mut report, prototype)? {
        Some(d) => d,
        None => return Ok(report),
    };
    if let Some(signal) = cancel::requested() {
        report.push(Diagnostic::error(
            "perform.cancelled",
            dispatch.dispatch_id.clone(),
            format!(
                "{alias}/{stage_id}: {signal} arrived before the performer was started, so none \
                 was. Dispatch {} is on record and unanswered; an interrupted run is not completion",
                dispatch.dispatch_id
            ),
        ));
        return Ok(report);
    }
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
    let outcome = hand_over(host, repo, &dir, &dispatch, bound)?;

    if let Some(path) = &outcome.lingering {
        report.push(Diagnostic::warn(
            "perform.writer-lingering",
            repo.relative(path),
            format!(
                "{alias}/{stage_id}: the performer is gone but its process group was still alive \
                 {}s later; the writer record is kept, and the next `war perform` on this stage \
                 will refuse until the group is gone",
                READ_GRACE.as_secs()
            ),
        ));
    }

    // A signal that arrived at any point up to here cancels the run, including
    // one that landed after the performer exited but before its answer was
    // ingested: a cancelled run leaves no submission (§55.7).
    if let Some(signal) = outcome.cancelled.or_else(cancel::requested) {
        discard(&outcome);
        report.push(Diagnostic::error(
            "perform.cancelled",
            outcome.dispatch_id.clone(),
            format!(
                "{alias}/{stage_id}: {signal} received; the performer's process group was killed \
                 and reaped and its answer discarded. Dispatch {} is on record and unanswered; an \
                 interrupted run is not completion{}",
                outcome.dispatch_id,
                tail(&outcome.stderr_tail)
            ),
        ));
        return Ok(report);
    }

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
pub fn all(repo: &Repository, prototype: bool) -> Result<Report, RepoError> {
    all_on(&Os, repo, prototype)
}

fn all_on(host: &dyn Host, repo: &Repository, prototype: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if !admitted(host, &mut report) {
        return Ok(report);
    }
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
    let total = open.len();
    for (done, (alias, stage)) in open.into_iter().enumerate() {
        let one = perform_one(host, repo, &alias, &stage, prototype)?;
        for d in one.diagnostics {
            report.push(d);
        }
        // A cancellation stops the walk after the stage it interrupted: the
        // operator asked `war` to stop, not to skip one stage.
        if cancel::requested().is_some() {
            let left = total - done - 1;
            if left > 0 {
                report.note(format!(
                    "cancelled: {left} open agent stage(s) after {alias}/{stage} were not started."
                ));
            }
            break;
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
    prototype: bool,
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
        crate::dispatch::Options {
            attempt_kind: openwarrant_core::execution::AttemptKind::Initial,
            prior_failure_evidence: &[],
            emit_to: Some(&scratch),
            emit_context_to: None,
            prototype,
        },
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
/// # Platform
///
/// The group kill is unix-only, and [`admitted`] has refused every other
/// platform before this is reached: elsewhere the deadline would reach the
/// process this spawned and not the children it spawned. A Windows port owes
/// this function a job object.
///
/// If the read grace expires the reader thread is left detached, still holding
/// the performer's pipes. The group kill makes that vanishingly rare — it takes
/// a grandchild wedged in uninterruptible sleep — and the alternative, joining
/// it, is the ten-minute hang this bound exists to prevent. A long
/// `war perform --all` on a box doing that repeatedly would accumulate threads;
/// what it must never do is stall, which it now cannot.
///
/// Its stdout is written verbatim to `dispatches/answer-<dispatch>.json` and
/// ingested from there, so what the tool validated is a file a reader can open
/// rather than a string that lived only in this process. It is deliberately NOT
/// written under `submissions/`: that directory holds submissions the seam
/// accepted, and a rejected answer sitting beside them would read as a record.
fn hand_over(
    host: &dyn Host,
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
    // The same choice keeps a terminal's Ctrl-C from reaching the performer
    // directly: it reaches `war`, whose handler kills the group.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = host.spawn(&mut command).map_err(|source| RepoError::Io {
        context: format!("could not run the performer {program}"),
        source,
    })?;
    // The writer record, before the performer is given anything to do. A
    // writer this tool cannot record is one it does not leave running.
    let record_path = writer_path(dir, &dispatch.dispatch_id);
    let record = WriterRecord {
        pid: child.id(),
        pgid: child.id(),
        leader_started: leader_started(child.id()).unwrap_or_default(),
        stage_id: dispatch.stage_id.clone(),
        dispatch_id: dispatch.dispatch_id.clone(),
    };
    let written = if record.leader_started.is_empty() {
        Err(format!(
            "could not read the start time of the performer (pid {})",
            record.pid
        ))
    } else {
        serde_json::to_string_pretty(&record)
            .map_err(|e| e.to_string())
            .and_then(|t| std::fs::write(&record_path, t).map_err(|e| e.to_string()))
    };
    if let Err(e) = written {
        kill_group(&mut child);
        if group_gone_within(record.pgid, READ_GRACE) {
            let _ = std::fs::remove_file(&record_path);
        }
        return Err(RepoError::Message(format!(
            "could not record the writer for dispatch {} at {record_path}: {e}. The performer \
             was killed rather than left running unrecorded",
            dispatch.dispatch_id
        )));
    }
    {
        let mut stdin = child.stdin.take().expect("piped");
        let body = serde_json::to_string_pretty(dispatch)
            .map_err(|e| RepoError::Message(format!("could not render the dispatch: {e}")))?;
        // A performer that exits before reading is reported by its status, not
        // by a broken pipe here.
        let _ = stdin.write_all(body.as_bytes());
    }
    let stdout = child.stdout.take().expect("piped");
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
        let _ = stdout.take(STDOUT_CAP + 1).read_to_string(&mut out);
        if out.len() as u64 > STDOUT_CAP {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        let _ = tx.send((out, err));
    });
    let deadline = Duration::from_secs(bound);
    let mut cancelled = None;
    let exit = loop {
        if let Some(signal) = cancel::requested() {
            kill_group(&mut child);
            cancelled = Some(signal);
            break None;
        }
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
    // The record goes only once the group is shown gone. A performer that
    // exited and left a child writing is still a writer, and the next
    // `war perform` on this stage has to be able to see it.
    let lingering = if group_gone_within(record.pgid, READ_GRACE) {
        let _ = std::fs::remove_file(&record_path);
        None
    } else {
        Some(record_path)
    };
    // Bounded, for the reason READ_GRACE gives. Output that has not arrived by
    // now is output this run does not have, and the report says the performer
    // answered nothing rather than waiting on a grandchild to close a pipe.
    let (out, err) = rx.recv_timeout(READ_GRACE).unwrap_or_default();
    let tail_start = err.len().saturating_sub(STDERR_TAIL);
    let oversized =
        out.len() as u64 > STDOUT_CAP || capped.load(std::sync::atomic::Ordering::Acquire);
    let submission_path = if out.trim().is_empty() || oversized || cancelled.is_some() {
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
        cancelled,
        lingering,
        stderr_tail: String::from_utf8_lossy(&err[tail_start..]).into_owned(),
        seconds: started.elapsed().as_secs(),
    })
}

/// SIGKILL the performer and everything it started, and reap it.
///
/// On unix the child leads its own process group (see the spawn above), so a
/// signal to the negated pid reaches the group.
fn kill_group(child: &mut Child) {
    #[cfg(unix)]
    if let Some(group) = i32::try_from(child.id())
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    {
        let _ = rustix::process::kill_process_group(group, rustix::process::Signal::KILL);
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

// ---------------------------------------------------------------------------
// The writer record and the handoff.
// ---------------------------------------------------------------------------

/// `dispatches/<dispatch>.writer`: who is performing a Dispatch right now.
///
/// It exists while a performer may write, so a second `war perform` on the
/// stage can ask whether the first has stopped. The leader's start time is kept
/// because a pid alone can be reused: a pid that is alive but started at a
/// different time is not shown to be the writer, and not shown to be gone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct WriterRecord {
    pid: u32,
    pgid: u32,
    /// Opaque, compared for equality only: clock ticks since boot from
    /// `/proc/<pid>/stat` on Linux, `ps -o lstart=` elsewhere.
    leader_started: String,
    stage_id: String,
    dispatch_id: String,
}

fn writer_path(dir: &camino::Utf8Path, dispatch_id: &str) -> camino::Utf8PathBuf {
    dir.join(DISPATCHES_DIR)
        .join(format!("{dispatch_id}.{WRITER_EXT}"))
}

/// What a probe of a recorded writer established.
#[derive(Debug, PartialEq, Eq)]
enum Liveness {
    /// Something in its process group is alive and may write.
    Alive,
    /// No process is in its group (`ESRCH`): it can no longer write.
    Gone,
    /// The probe could not tell, and says why. Never read as either answer.
    Unknown(String),
}

fn probe(record: &WriterRecord) -> Liveness {
    match group_liveness(record.pgid) {
        Liveness::Alive => match leader_started(record.pid) {
            Some(now) if now == record.leader_started => Liveness::Alive,
            Some(now) => Liveness::Unknown(format!(
                "pid {} is alive but started at {now:?}, not the recorded {:?}: it is not shown \
                 to be the recorded writer, and process group {} is not shown to be gone",
                record.pid, record.leader_started, record.pgid
            )),
            // The leader is gone and its group is not: what it started is
            // still running, and can still write.
            None => Liveness::Alive,
        },
        other => other,
    }
}

/// Before compiling: probe every writer record for this stage. `true` when the
/// stage has no writer that may still write; otherwise the report says why not.
fn handoff(
    repo: &Repository,
    dir: &camino::Utf8Path,
    alias: &str,
    stage_id: &str,
    report: &mut Report,
) -> bool {
    let dispatches = dir.join(DISPATCHES_DIR);
    let Ok(entries) = std::fs::read_dir(&dispatches) else {
        // No dispatches directory: nothing was ever handed over.
        return true;
    };
    let mut records: Vec<camino::Utf8PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|e| camino::Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.extension() == Some(WRITER_EXT))
        .collect();
    records.sort();
    let mut clear = true;
    for path in records {
        let at = repo.relative(&path);
        let parsed: Option<WriterRecord> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok());
        let Some(record) = parsed else {
            // A record nobody can read names a writer nobody can probe. Which
            // stage it was for is unknown too, so it holds every stage.
            report.push(Diagnostic::unknown(
                "perform.writer-unknown",
                at,
                format!(
                    "{alias}/{stage_id}: a writer record here cannot be read, so whether its \
                     performer can still write cannot be established. Refused rather than guessed: \
                     confirm by hand that its process group is gone, then remove the record"
                ),
            ));
            clear = false;
            continue;
        };
        if record.stage_id != stage_id {
            continue;
        }
        match probe(&record) {
            Liveness::Alive => {
                report.push(Diagnostic::error(
                    "perform.writer-alive",
                    at,
                    format!(
                        "{alias}/{stage_id}: dispatch {} is still being performed by pid {} \
                         (process group {}), which can still write. A second writer is not \
                         started while the first can; stop it, or let it finish",
                        record.dispatch_id, record.pid, record.pgid
                    ),
                ));
                clear = false;
            }
            Liveness::Unknown(why) => {
                report.push(Diagnostic::unknown(
                    "perform.writer-unknown",
                    at,
                    format!(
                        "{alias}/{stage_id}: whether the writer of dispatch {} has stopped cannot \
                         be established: {why}. Refused rather than guessed; confirm by hand that \
                         the group is gone, then remove the record",
                        record.dispatch_id
                    ),
                ));
                clear = false;
            }
            Liveness::Gone => {
                let _ = std::fs::remove_file(&path);
                report.push(Diagnostic::pass(
                    "perform.writer-stopped",
                    format!(
                        "{alias}/{stage_id}: the previous writer of dispatch {} (pid {}, process \
                         group {}) is gone — no process is left in its group — so its record was \
                         removed",
                        record.dispatch_id, record.pid, record.pgid
                    ),
                ));
            }
        }
    }
    clear
}

/// `kill(-pgid, 0)`: whether any process is left in the group.
#[cfg(unix)]
fn group_liveness(pgid: u32) -> Liveness {
    let Some(group) = i32::try_from(pgid)
        .ok()
        .filter(|g| *g > 1)
        .and_then(rustix::process::Pid::from_raw)
    else {
        return Liveness::Unknown(format!("{pgid} is not a process group this can probe"));
    };
    match rustix::process::test_kill_process_group(group) {
        Ok(()) => Liveness::Alive,
        Err(rustix::io::Errno::SRCH) => Liveness::Gone,
        // EPERM: a group exists and belongs to someone else. It is not shown
        // gone, and it is not shown to be ours.
        Err(e) => Liveness::Unknown(format!("probing process group {pgid}: {e}")),
    }
}

#[cfg(not(unix))]
fn group_liveness(pgid: u32) -> Liveness {
    Liveness::Unknown(format!(
        "process group {pgid}: this platform has no process-group probe"
    ))
}

/// Poll until the group is shown gone, for at most `grace`.
fn group_gone_within(pgid: u32, grace: Duration) -> bool {
    let until = Instant::now() + grace;
    loop {
        if group_liveness(pgid) == Liveness::Gone {
            return true;
        }
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The start time of `pid`, or `None` when no such process exists.
#[cfg(target_os = "linux")]
fn leader_started(pid: u32) -> Option<String> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Field 22, `starttime`. The command name (field 2) is in parentheses and
    // may itself contain spaces or parentheses, so count from the last `)`:
    // field 3 is the first after it.
    let ticks = stat.rsplit_once(')')?.1.split_whitespace().nth(19)?;
    Some(format!("boot+{ticks}ticks"))
}

#[cfg(all(unix, not(target_os = "linux")))]
fn leader_started(pid: u32) -> Option<String> {
    let out = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    (out.status.success() && !text.is_empty()).then_some(text)
}

#[cfg(not(unix))]
fn leader_started(_pid: u32) -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Cancellation.
// ---------------------------------------------------------------------------

/// SIGINT and SIGTERM, turned into a flag the wait loop reads (OW-WAR-0131
/// U-001 option A, `signal-hook`).
///
/// The handlers are installed once, the first time a performance is armed, and
/// stay installed. Outside an armed performance they run the signal's default
/// action — `war` dies of it as it always did — so the console, which performs
/// a stage and then goes on prompting, keeps an ordinary Ctrl-C. Inside, they
/// only record which signal arrived.
#[cfg(unix)]
mod cancel {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, OnceLock};

    use signal_hook::consts::{SIGINT, SIGTERM};

    struct Flags {
        /// The signal that arrived while armed; 0 for none.
        signal: Arc<AtomicUsize>,
        /// True outside a performance: the default action runs.
        outside: Arc<AtomicBool>,
    }

    static FLAGS: OnceLock<Result<Flags, String>> = OnceLock::new();

    fn flags() -> Result<&'static Flags, String> {
        FLAGS
            .get_or_init(|| {
                let f = Flags {
                    signal: Arc::new(AtomicUsize::new(0)),
                    outside: Arc::new(AtomicBool::new(true)),
                };
                for sig in [SIGINT, SIGTERM] {
                    // The default goes first: registered actions run in order,
                    // and outside a performance the signal must still kill.
                    signal_hook::flag::register_conditional_default(sig, Arc::clone(&f.outside))
                        .map_err(|e| e.to_string())?;
                    let value = usize::try_from(sig).map_err(|e| e.to_string())?;
                    signal_hook::flag::register_usize(sig, Arc::clone(&f.signal), value)
                        .map_err(|e| e.to_string())?;
                }
                Ok(f)
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// While this lives, SIGINT and SIGTERM cancel the performance instead of
    /// killing `war` outright.
    pub struct Armed(&'static Flags);

    pub fn arm() -> Result<Armed, String> {
        let f = flags()?;
        f.signal.store(0, Ordering::SeqCst);
        f.outside.store(false, Ordering::SeqCst);
        Ok(Armed(f))
    }

    impl Drop for Armed {
        fn drop(&mut self) {
            self.0.outside.store(true, Ordering::SeqCst);
        }
    }

    /// The signal that cancelled the last armed performance, if one did.
    pub fn requested() -> Option<&'static str> {
        let f = FLAGS.get()?.as_ref().ok()?;
        match f.signal.load(Ordering::SeqCst) {
            0 => None,
            s if usize::try_from(SIGINT).is_ok_and(|i| i == s) => Some("SIGINT"),
            _ => Some("SIGTERM"),
        }
    }
}

/// No platform without the group kill gets past [`admitted`], so nothing here
/// is ever armed.
#[cfg(not(unix))]
mod cancel {
    pub struct Armed;

    pub fn arm() -> Result<Armed, String> {
        Ok(Armed)
    }

    pub fn requested() -> Option<&'static str> {
        None
    }
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

    /// A host that counts the spawns it is asked for and performs none.
    struct FakeHost {
        group_kill: bool,
        spawns: std::cell::Cell<usize>,
    }

    impl Host for FakeHost {
        fn group_kill(&self) -> bool {
            self.group_kill
        }
        fn spawn(&self, _: &mut Command) -> std::io::Result<Child> {
            self.spawns.set(self.spawns.get() + 1);
            Err(std::io::Error::other("the fake host spawns nothing"))
        }
    }

    /// A repository with this one's configuration (a performer configured) and
    /// no Warrants, in a directory of its own.
    fn scratch_repo(name: &str) -> Repository {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-perform-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let root = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        std::fs::copy(root.join("openwarrant.toml"), dir.join("openwarrant.toml")).unwrap();
        let repo = Repository::open(dir).expect("scratch repository opens");
        assert!(
            !repo.config.perform.performer_argv.is_empty(),
            "the scratch must have a performer configured, or the refusal below could be \
             perform.no-performer's"
        );
        repo
    }

    /// OW-WAR-0131 OBL-003: a platform without the group kill is refused
    /// `perform.unsupported-os`, and nothing is compiled or spawned — for one
    /// stage and for `--all`.
    #[test]
    fn a_platform_without_the_group_kill_is_refused_before_anything_runs() {
        let repo = scratch_repo("unsupported");
        let host = FakeHost {
            group_kill: false,
            spawns: std::cell::Cell::new(0),
        };
        for report in [
            run_on(&host, &repo, "OW-WAR-0068", "STAGE-001", true).expect("a report"),
            all_on(&host, &repo, true).expect("a report"),
        ] {
            let rules: Vec<&str> = report.diagnostics.iter().map(|d| d.rule.as_str()).collect();
            assert_eq!(rules, ["perform.unsupported-os"], "{rules:?}");
            assert!(!report.is_ready());
        }
        assert_eq!(host.spawns.get(), 0, "no spawn was attempted");
        assert!(
            !repo.root.join("docs").exists(),
            "nothing was compiled: no dispatches directory, no journal"
        );
        let _ = std::fs::remove_dir_all(&repo.root);
    }

    /// The refusal above is the platform's, not blanket: the same call on a
    /// host with the group kill gets past admission and fails on what the
    /// scratch lacks instead — a Warrant — still without spawning.
    #[test]
    fn a_platform_with_the_group_kill_is_admitted() {
        let repo = scratch_repo("supported");
        let host = FakeHost {
            group_kill: true,
            spawns: std::cell::Cell::new(0),
        };
        match run_on(&host, &repo, "OW-WAR-0068", "STAGE-001", true) {
            Err(_) => {}
            Ok(report) => assert!(
                report
                    .diagnostics
                    .iter()
                    .all(|d| d.rule != "perform.unsupported-os"),
                "{:?}",
                report.diagnostics
            ),
        }
        assert_eq!(host.spawns.get(), 0);
        let _ = std::fs::remove_dir_all(&repo.root);
    }

    /// The compiled binary is on the branch the conformance plants exercise.
    #[test]
    fn this_build_has_the_group_kill_exactly_on_unix() {
        assert_eq!(Os.group_kill(), cfg!(unix));
    }

    /// The probe's three answers, on processes this test owns.
    #[cfg(unix)]
    #[test]
    fn a_writer_is_alive_gone_or_unknown_and_never_guessed() {
        use std::os::unix::process::CommandExt as _;
        let mut child = Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .expect("sleep runs");
        let pid = child.id();
        let record = WriterRecord {
            pid,
            pgid: pid,
            leader_started: leader_started(pid).expect("a live process has a start time"),
            stage_id: "STAGE-001".into(),
            dispatch_id: "D".into(),
        };
        assert_eq!(probe(&record), Liveness::Alive);
        // A start time that does not match is not "the same writer", and it
        // is not "gone" either.
        let reused = WriterRecord {
            leader_started: "boot+0ticks".into(),
            ..record.clone()
        };
        assert!(matches!(probe(&reused), Liveness::Unknown(_)));
        let _ = rustix::process::kill_process_group(
            rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap(),
            rustix::process::Signal::KILL,
        );
        let _ = child.wait();
        assert!(group_gone_within(pid, READ_GRACE));
        assert_eq!(probe(&record), Liveness::Gone);
        assert_eq!(probe(&reused), Liveness::Gone);
        // A group that cannot be named is unknown, not gone.
        assert!(matches!(group_liveness(0), Liveness::Unknown(_)));
    }
}
