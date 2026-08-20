// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war gate list` and `war gate run` — local gate execution (SAS §44).
//!
//! # Askability is decided BEFORE execution
//!
//! §44.1 separates askability from result, and the order matters more than the
//! separation. [`askability_of`] runs to completion before any process is
//! spawned, and it is the only thing that can produce a `not_askable` run. Once
//! a process has been spawned the code cannot reach `missing_tool` — which is
//! precisely how "could not ask" becomes "failed" in systems that decide
//! askability from a non-zero exit code.
//!
//! # This command runs code from the corpus
//!
//! `war gate --run` spawns each gate's declared `argv` with the repository root
//! as its working directory. There is no sandbox and no allowlist. A gate
//! definition is executable content, and `mutating` is self-declared — a gate
//! that lies about it will still run. Running this against a corpus you did not
//! author is running that corpus's code. The gate author is the trust boundary,
//! and sandboxing is beta hardening, not something this alpha claims.
//!
//! OW-WAR-0020's Intent records the cost of getting this wrong: the parent
//! project's corpus contained, when measured once at LamQuant `5369da81` on
//! 2026-08-17, 12 missing-tool, 7 missing-script and 4
//! missing-crate gates. Collapsed into `failed`, those 23 would have read as
//! measured failures of the subject rather than as gates that never ran.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use openwarrant_core::gate::GateDefinition;
use openwarrant_core::{Askability, ExecutionStatus, GateRun, ReasonCode, Verdict};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// The deadline used when a gate declares none.
const DEFAULT_GATE_TIMEOUT: Duration = Duration::from_secs(600);

/// How often the deadline is checked at first, and the ceiling it backs off to.
///
/// A flat 50ms wakes 2,400 times across a 120-second gate to learn nothing.
/// Backing off keeps a fast gate responsive and a slow one cheap.
const POLL_INTERVAL_MIN: Duration = Duration::from_millis(10);
const POLL_INTERVAL_MAX: Duration = Duration::from_millis(250);

/// How long to wait for a killed child to be reaped before giving up on it.
const REAP_GRACE: Duration = Duration::from_secs(5);

/// Why a gate cannot be asked, or `None` if it can.
///
/// Every branch returns a §96.4 class, so a gate that cannot be asked always
/// carries a reason and never has to be summarised as a failure.
#[must_use]
pub fn askability_of(def: &GateDefinition, repo: &Repository) -> Option<ReasonCode> {
    // §44.8 first. A mutating gate is not asked in a routine run at all, so it
    // is unaskable here regardless of how well it is declared.
    if def.mutating {
        return Some(ReasonCode::Mutating);
    }
    // §43.3 — an unqualified or invalidated definition cannot be bound, so there
    // is nothing legitimate to ask.
    if !def.lifecycle.is_bindable() {
        return Some(ReasonCode::Malformed);
    }
    let Some(program) = def.argv.first() else {
        // No argument vector is not an empty command; it is a definition that
        // never said what to run.
        return Some(ReasonCode::Malformed);
    };
    if program.trim().is_empty() {
        return Some(ReasonCode::Malformed);
    }
    if !repo.root.is_dir() {
        return Some(ReasonCode::ForeignWorkingDirectory);
    }
    if !tool_is_available(program, repo) {
        return Some(classify_missing(program));
    }
    None
}

/// Distinguish the three ways a thing can be absent (§96.4 keeps them apart).
///
/// The classification is by SHAPE, which is a heuristic and is the honest limit
/// of what can be known before the thing runs. It is still worth making: §96.4
/// preserves these three as distinct classes, and "the script is not in the tree"
/// and "the toolchain is not installed" call for different repairs.
///
/// `missing_crate` is only reachable when cargo or rustc itself is absent. A
/// gate that invokes a crate that does not exist gets as far as running cargo,
/// so it comes back `completed` + `fail` — correctly, since cargo was asked and
/// answered. Detecting a missing crate inside a successful cargo invocation
/// means parsing cargo's output, which is a gate's job and not the runner's.
fn classify_missing(program: &str) -> ReasonCode {
    if program.ends_with(".sh") || program.ends_with(".py") {
        ReasonCode::MissingScript
    } else if program == "cargo" || program == "rustc" {
        ReasonCode::MissingCrate
    } else {
        ReasonCode::MissingTool
    }
}

/// Whether the program resolves, on PATH or relative to the repository root.
///
/// Deliberately checks existence, NOT the execute bit. A file that exists but
/// cannot be executed is a different failure from one that is not there, and it
/// surfaces as `infrastructure_error` from the spawn rather than being guessed
/// at here. Do not "fix" this into an `access(X_OK)` check without deciding
/// which §96.4 class a non-executable file belongs to.
fn tool_is_available(program: &str, repo: &Repository) -> bool {
    if program.contains('/') {
        let candidate = repo.root.join(program);
        return candidate.is_file() || camino::Utf8Path::new(program).is_file();
    }
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}

/// Run one gate and produce its §44 result.
///
/// The `not_askable` path returns before any process is spawned.
#[must_use]
pub fn run_gate(def: &GateDefinition, repo: &Repository) -> GateRun {
    let id = format!("GR-{}", def.key());

    if let Some(reason) = askability_of(def, repo) {
        // §44.4: not_askable pairs with not_run (or invalid, for a malformed
        // definition) and verdict unknown. Never a verdict.
        let execution_status = if reason == ReasonCode::Malformed {
            ExecutionStatus::Invalid
        } else {
            ExecutionStatus::NotRun
        };
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status,
            verdict: Verdict::Unknown,
            reason_code: Some(reason),
        };
    }

    let deadline = def
        .timeout_secs
        .map_or(DEFAULT_GATE_TIMEOUT, Duration::from_secs);
    let started = Instant::now();
    let spawned = Command::new(&def.argv[0])
        .args(&def.argv[1..])
        .current_dir(&repo.root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    let mut child = match spawned {
        Ok(c) => c,
        // The tool resolved during askability and would not spawn. The commonest
        // cause is a race — it was removed, or it is not executable — so the two
        // are told apart rather than both landing on "infrastructure".
        Err(err) => {
            return match err.kind() {
                // It vanished between the check and the spawn. That is still
                // "could not ask", and saying so keeps it out of the failure
                // column where it does not belong.
                std::io::ErrorKind::NotFound => GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::NotAskable,
                    execution_status: ExecutionStatus::NotRun,
                    verdict: Verdict::Unknown,
                    reason_code: Some(classify_missing(&def.argv[0])),
                },
                // Present but unusable: permissions, exhausted resources. The
                // gate was askable and the environment failed, which is not a
                // verdict about the subject.
                _ => GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::InfrastructureError,
                    verdict: Verdict::Unknown,
                    reason_code: None,
                },
            };
        }
    };

    let mut poll = POLL_INTERVAL_MIN;

    // A real deadline. The previous version compared elapsed time AFTER
    // `output()` had already blocked to completion, which meant a gate running
    // 601 seconds and exiting 0 was reported `timeout` + `unknown` — discarding
    // a genuine pass and reporting a status that had not happened. Polling and
    // killing is what `timeout` has to mean for the word to be true.
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let passed = status.success();
                return GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::Completed,
                    verdict: if passed { Verdict::Pass } else { Verdict::Fail },
                    reason_code: Some(if passed {
                        ReasonCode::Passed
                    } else {
                        ReasonCode::Failed
                    }),
                };
            }
            Ok(None) if started.elapsed() >= deadline => {
                let _ = child.kill();
                // Reap, but never unboundedly. A plain `wait()` here blocks
                // forever if the child is in uninterruptible sleep and cannot
                // take the signal yet — and a runner whose whole purpose is a
                // deadline must not be the thing that hangs. SIGKILL is not
                // blockable, so the child dies once its I/O completes; if it
                // outlasts this window it is left to be reaped at exit rather
                // than held onto.
                let reap_until = Instant::now() + REAP_GRACE;
                while Instant::now() < reap_until {
                    match child.try_wait() {
                        Ok(Some(_)) | Err(_) => break,
                        Ok(None) => std::thread::sleep(POLL_INTERVAL_MIN),
                    }
                }
                return GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::Timeout,
                    verdict: Verdict::Unknown,
                    reason_code: Some(ReasonCode::Timeout),
                };
            }
            Ok(None) => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(POLL_INTERVAL_MAX);
            }
            Err(_) => {
                return GateRun {
                    id,
                    gate: def.key(),
                    askability: Askability::Askable,
                    execution_status: ExecutionStatus::InfrastructureError,
                    verdict: Verdict::Unknown,
                    reason_code: None,
                };
            }
        }
    }
}

/// `war gate list` / `war gate run`.
pub fn run(repo: &Repository, execute: bool, only: Option<&str>) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let registry = crate::check::load_gate_registry(repo, &mut report);

    if registry.is_empty() {
        report.note("no gate definitions found; nothing to run".to_owned());
        return Ok(report);
    }

    for def in &registry.definitions {
        if only.is_some_and(|filter| def.gate_id != filter && def.key() != filter) {
            continue;
        }

        if !execute {
            report.push(Diagnostic::pass(
                "gate.listed",
                format!(
                    "{}: {} · lifecycle {} · {}",
                    def.key(),
                    def.provenance,
                    def.lifecycle,
                    match askability_of(def, repo) {
                        Some(r) => format!("not askable ({r})"),
                        None => "askable".to_owned(),
                    }
                ),
            ));
            continue;
        }

        let run = run_gate(def, repo);
        // Coherence is checked on our own output. A runner that emits an
        // incoherent run is a runner that can emit a passing unaskable gate.
        if let Err(err) = run.validate() {
            report.push(Diagnostic::error(
                "gate-run.incoherent",
                def.key(),
                format!("the runner produced an invalid run: {err}"),
            ));
            continue;
        }

        let reason = run
            .reason_code
            .map_or_else(String::new, |r| format!(" ({r})"));
        let line = format!(
            "{}: askability {} · execution {} · verdict {}{reason}",
            def.key(),
            run.askability,
            run.execution_status,
            run.verdict
        );

        if run.satisfies_required_pass() {
            report.push(Diagnostic::pass("gate-run.pass", line));
        } else if run.askability == Askability::NotAskable {
            // §44.1 and RQ-054: this is NOT a failure. Reporting it as one is the
            // collapse §96.4 forbids, and it is why this branch exists.
            //
            // Routed on ASKABILITY, not on the verdict being unknown. Keyed on
            // `is_blocking_unknown()` this branch also caught askable runs that
            // timed out or hit an infrastructure error, and told the reader
            // "could not ask" about a gate that was asked and did not finish —
            // the same class of collapse, in the opposite direction.
            report.push(Diagnostic::unknown(
                "gate-run.unaskable",
                def.key(),
                format!("{line} — could not ask, so there is no result to report"),
            ));
        } else if run.is_blocking_unknown() {
            // Asked, started, did not produce an answer: timeout, cancellation,
            // infrastructure. Blocking under RQ-054 but NOT a failure of the
            // subject, so it gets neither the unaskable rule nor the fail rule.
            report.push(Diagnostic::unknown(
                "gate-run.no-result",
                def.key(),
                format!("{line} — asked, but produced no result"),
            ));
        } else {
            report.push(Diagnostic::error("gate-run.fail", def.key(), line));
        }
    }
    Ok(report)
}
