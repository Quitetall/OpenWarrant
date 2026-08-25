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

use openwarrant_compiler::sha256_hex;
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

/// The file a gate's structured run is written to, under the receipts path.
///
/// One file per gate key, overwritten by the next run: the QUESTION answered is
/// "what did this gate last say?", and keeping a history here would invite
/// reading a stale pass as a current one.
#[must_use]
pub fn run_record_path(gate_key: &str, repo: &Repository) -> camino::Utf8PathBuf {
    let safe = gate_key.replace(['/', ':', '@', '.'], "_");
    repo.root
        .join(&repo.config.paths.receipts)
        .join(format!("{safe}.run.toml"))
}

/// Write a Gate Run where a later resolution can read it (§44.6).
pub fn persist_run(run: &GateRun, repo: &Repository) -> Result<(), String> {
    let path = run_record_path(&run.gate, repo);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("could not create {parent}: {e}"))?;
    }
    let rendered = toml::to_string_pretty(run).map_err(|e| e.to_string())?;
    std::fs::write(&path, rendered).map_err(|e| format!("could not write {path}: {e}"))
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

    // Where the streams land. Created before the spawn so a failure to open them
    // is an askability problem, not a half-run.
    let dir = repo.root.join(&repo.config.paths.receipts);
    if std::fs::create_dir_all(&dir).is_err() {
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status: ExecutionStatus::Invalid,
            verdict: Verdict::Unknown,
            reason_code: Some(ReasonCode::ForeignWorkingDirectory),
        };
    }
    let slug = def.key().replace(['/', '@', '.'], "_");
    let (out_path, err_path) = (
        dir.join(format!("{slug}.stdout.txt")),
        dir.join(format!("{slug}.stderr.txt")),
    );
    let (Ok(stdout_file), Ok(stderr_file)) = (
        std::fs::File::create(&out_path),
        std::fs::File::create(&err_path),
    ) else {
        return GateRun {
            id,
            gate: def.key(),
            askability: Askability::NotAskable,
            execution_status: ExecutionStatus::Invalid,
            verdict: Verdict::Unknown,
            reason_code: Some(ReasonCode::ForeignWorkingDirectory),
        };
    };

    let started = Instant::now();
    let spawned = Command::new(&def.argv[0])
        .args(&def.argv[1..])
        .current_dir(&repo.root)
        // §44.6 requires stdout and stderr REFS on the receipt, so the streams
        // are captured to files rather than discarded. Piping without draining
        // can deadlock a chatty child on a full pipe buffer, so they go straight
        // to files opened here.
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
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
pub fn run(
    repo: &Repository,
    execute: bool,
    only: Option<&str>,
    record: bool,
    subject_digests: &[String],
    raw_evidence_refs: &[String],
) -> Result<Report, RepoError> {
    validate_bonsai_bindings(repo, subject_digests, raw_evidence_refs)?;
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

        let started_at = receipt::now_rfc3339_public();
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

        // §44.6 — persist the RUN, not only its streams, but ONLY when asked.
        //
        // Recording is opt-in because a gate is run for two different reasons.
        // Producing evidence about a subject is one; PROBING the gate's own
        // behaviour is the other, and `conformance/plant.sh` does the second by
        // deliberately corrupting the gate definition and checking the refusal.
        //
        // When recording happened on every invocation, the last such probe left
        // `not_askable / invalid / malformed` as the gate's persisted last word,
        // and every later `war resolve` read that as the real verdict. A
        // deliberately broken test run had silently become the evidentiary
        // record.
        //
        // The fix is NOT to skip recording bad runs — refusing to write failures
        // is the "only record good news" pattern this system exists to prevent.
        // It is to make recording deliberate, which is what the receipts
        // .gitignore already says: evidence is committed on purpose, never as a
        // side effect of running.
        //
        // Before this, a run existed for the length of the process and left
        // behind stdout/stderr text. §56.1's "every required gate has admissible
        // result" cannot be answered from prose, so the structured verdict is
        // written where a later `war resolve` can read it.
        //
        // Written under the receipts path, which is disposable by policy: a run
        // is evidence produced BY running, and it becomes committed evidence
        // deliberately at resolution rather than as a side effect.
        if record && let Err(err) = persist_run(&run, repo) {
            report.push(Diagnostic::error(
                "gate-run.not-persisted",
                def.key(),
                format!(
                    "{err} — a run that is not written cannot answer §56.1's \
                     admissible-result requirement later"
                ),
            ));
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

        // §44.6 — a completed run produces a receipt. Only a run that actually
        // executed has anything to receipt: an unaskable gate produced no
        // working directory, no exit result and no streams, and minting a
        // receipt for it would be minting evidence of something that did not
        // happen.
        if run.execution_status == ExecutionStatus::Completed {
            match receipt::mint(
                repo,
                def,
                &run,
                &started_at,
                &format!("{}", run.verdict),
                subject_digests,
                raw_evidence_refs,
            ) {
                Ok(path) => report.push(Diagnostic::pass(
                    "gate-run.receipt",
                    format!(
                        "{}: §44.6 receipt written to {}",
                        def.key(),
                        repo.relative(&path)
                    ),
                )),
                Err(e) => report.push(Diagnostic::error(
                    "gate-run.receipt-failed",
                    def.key(),
                    format!("{e}"),
                )),
            }
        }

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

/// Refuse a receipt that attaches a Bonsai document by name but not by bytes,
/// or that lets a passing local gate appear to endorse failed Bonsai evidence.
///
/// This is intentionally narrow: only the Warrant/Bonsai adapter needs these
/// extra receipt fields today. A future generic evidence registry can widen it
/// with typed artifact kinds instead of accepting arbitrary strings now.
fn validate_bonsai_bindings(
    repo: &Repository,
    subject_digests: &[String],
    raw_evidence_refs: &[String],
) -> Result<(), RepoError> {
    if subject_digests.is_empty() && raw_evidence_refs.is_empty() {
        return Ok(());
    }
    if subject_digests.len() != 1 || raw_evidence_refs.len() != 1 {
        return Err(RepoError::Message(
            "Bonsai receipt binding requires exactly one contract subject and one evidence reference"
                .to_owned(),
        ));
    }
    let subject = &subject_digests[0];
    let Some(contract_digest) = subject.strip_prefix("contract:sha256:") else {
        return Err(RepoError::Message(
            "Bonsai receipt subject must be contract:sha256:<digest>".to_owned(),
        ));
    };
    if !is_hex_digest(contract_digest) {
        return Err(RepoError::Message(
            "Bonsai receipt contract digest must be 64 lowercase hex characters".to_owned(),
        ));
    }
    let Some((path, expected_digest)) = raw_evidence_refs[0]
        .strip_prefix("file:")
        .and_then(|reference| reference.rsplit_once("#sha256:"))
    else {
        return Err(RepoError::Message(
            "Bonsai evidence reference must be file:<repo-relative-path>#sha256:<digest>"
                .to_owned(),
        ));
    };
    if !safe_evidence_path(path) || !is_hex_digest(expected_digest) {
        return Err(RepoError::Message(
            "Bonsai evidence reference has an unsafe path or invalid digest".to_owned(),
        ));
    }
    let bytes = std::fs::read(repo.root.join(path)).map_err(|source| RepoError::Io {
        context: format!("could not read Bonsai evidence {path}"),
        source,
    })?;
    if sha256_hex(&bytes) != expected_digest {
        return Err(RepoError::Message(
            "Bonsai evidence reference digest does not match file bytes".to_owned(),
        ));
    }
    let evidence: serde_json::Value = serde_json::from_slice(&bytes).map_err(|source| {
        RepoError::Message(format!("Bonsai evidence is not valid JSON: {source}"))
    })?;
    let evidence_contract = evidence
        .pointer("/warrant/contract_digest")
        .and_then(serde_json::Value::as_str);
    if evidence.get("schema").and_then(serde_json::Value::as_str)
        != Some("oh.war/bonsai-evidence/v1")
        || evidence.get("verdict").and_then(serde_json::Value::as_str) != Some("pass")
        || evidence_contract != Some(contract_digest)
    {
        return Err(RepoError::Message(
            "Bonsai evidence must be a passing v1 report for the bound contract digest".to_owned(),
        ));
    }
    Ok(())
}

fn safe_evidence_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn is_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Mint a §44.6 receipt for a completed run, and write it beside its streams.
///
/// # Why this exists
///
/// Through the whole of alpha, `GateReceipt` was implemented, unit-tested, and
/// referenced by no code in any binary. §44.6 says a receipt SHALL record
/// eighteen things; nothing produced one, so the requirement was satisfied by a
/// struct definition and a test that constructed one by hand.
///
/// A receipt is also the artifact every beta obligation cites. An obligation
/// demanding "a gate-run receipt whose verdict is pass" is undischargeable by a
/// `#[test]` precisely because a receipt binds wall-clock times, a working
/// directory, an exit result and a digest that has to recompute.
pub mod receipt {
    use camino::Utf8Path;
    use openwarrant_compiler::canonical::sha256_digest;
    use openwarrant_compiler::digest::DigestDomain;
    use openwarrant_core::gate::GateDefinition;
    use openwarrant_core::{GateReceipt, GateRun};

    use crate::repo::{RepoError, Repository};

    /// RFC 3339, UTC, seconds precision — enough to order runs, and no more
    /// precision than the value actually carries.
    /// Public alias so the run path can stamp `started_at` before spawning.
    #[must_use]
    pub fn now_rfc3339_public() -> String {
        now_rfc3339()
    }

    fn now_rfc3339() -> String {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());
        // Civil-from-days, so a receipt does not pull in a date crate for one
        // timestamp. Correct for all dates this system will ever record.
        let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
            rem / 3600,
            (rem % 3600) / 60,
            rem % 60
        )
    }

    /// Build and persist the receipt. Returns its path.
    ///
    /// The receipt is VALIDATED before it is written. A malformed receipt on
    /// disk is worse than none: it looks like evidence.
    pub fn mint(
        repo: &Repository,
        def: &GateDefinition,
        run: &GateRun,
        started_at: &str,
        exit_result: &str,
        subject_digests: &[String],
        raw_evidence_refs: &[String],
    ) -> Result<camino::Utf8PathBuf, RepoError> {
        let slug = def.key().replace(['/', '@', '.'], "_");
        let rel = |p: &Utf8Path| repo.relative(p);
        let dir = repo.root.join(&repo.config.paths.receipts);

        let mut receipt = GateReceipt {
            run_id: run.id.clone(),
            gate_definition_digest: if def.digest.is_empty() {
                // A definition with no declared digest still has an identity;
                // deriving one here keeps the receipt complete without inventing
                // a value that looks like the author's.
                format!(
                    "sha256:{}",
                    sha256_digest(DigestDomain::GateRun, &def.key())
                        .map_err(|e| RepoError::Message(format!("{e}")))?
                )
            } else {
                def.digest.clone()
            },
            // §43.5 bindings do not exist in this corpus yet. Recording the
            // gate's own key is honest; inventing a binding digest would not be.
            gate_binding_digest: format!("unbound:{}", def.key()),
            subject_digests: if subject_digests.is_empty() {
                vec![format!("warrant-corpus:{}", repo.config.paths.warrants)]
            } else {
                subject_digests.to_vec()
            },
            fixture_digests: vec![],
            runner: "war gate --run".to_owned(),
            runtime_environment: format!(
                "{} {} / rustc {}",
                std::env::consts::OS,
                std::env::consts::ARCH,
                option_env!("CARGO_PKG_RUST_VERSION").unwrap_or("unknown")
            ),
            arguments: def.argv.clone(),
            working_directory: repo.root.to_string(),
            started_at: started_at.to_owned(),
            completed_at: now_rfc3339(),
            exit_result: exit_result.to_owned(),
            selected_test_count: 0,
            selected_test_manifest: vec![],
            raw_evidence_refs: raw_evidence_refs.to_vec(),
            stdout_ref: rel(&dir.join(format!("{slug}.stdout.txt"))),
            stderr_ref: rel(&dir.join(format!("{slug}.stderr.txt"))),
            resource_usage: format!("wall-clock only; {} argv item(s)", def.argv.len()),
            verdict: run.verdict,
            receipt_digest: String::new(),
        };

        // Digest last, over everything else, so it covers the record it seals.
        receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateRun, &receipt)
                .map_err(|e| RepoError::Message(format!("{e}")))?
        );

        receipt.validate().map_err(|e| {
            RepoError::Message(format!("refusing to write a malformed receipt: {e}"))
        })?;

        let path = dir.join(format!("{slug}.receipt.json"));
        let body = serde_json::to_string_pretty(&receipt)
            .map_err(|e| RepoError::Message(format!("{e}")))?;
        std::fs::write(&path, body + "\n")
            .map_err(|e| RepoError::Message(format!("cannot write {path}: {e}")))?;
        Ok(path)
    }
}
