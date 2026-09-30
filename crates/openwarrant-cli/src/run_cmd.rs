// SPDX-License-Identifier: Apache-2.0
//! `war run <alias> <stage>` and `war submit <alias> <file>` — the ops / runs /
//! experiments work kind (slice C4b; SAS §47, §51, §44.6).
//!
//! A `service` stage names a registered gate as its executor
//! (`executor_ref = "gate://<key>"`). `war run` compiles the stage's Dispatch,
//! runs that gate under the smaller of the stage's `wall_time_seconds` and
//! the gate's own timeout, mints a §44.6 receipt whose subject is the
//! **dispatch digest**, and writes a Stage Submission (§51) whose requested
//! next action is `verify` on a pass and `block` — with a Blocker — on a
//! failure or a timeout. Never anything else: a run cannot ask to be
//! resolved, and `validate_requested_action` refuses `resolve` by name.
//!
//! `war submit` ingests a submission something else produced (an agent, a
//! BLUT job) through the same two refusals — it must name a dispatch this
//! Warrant compiled, and it may not request its own completion — and writes
//! it under `submissions/` only when both hold.

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::execution::{
    AttemptKind, Blocker, PerformerClaim, RequestedNextAction, StageDispatch, StageSubmission,
};
use openwarrant_core::milestones::ExecutorKind;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const SUBMISSIONS_DIR: &str = "submissions";
const DISPATCHES_DIR: &str = "dispatches";

fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    crate::gate_cmd::receipt::rfc3339_from_secs(secs)
}

fn write_json<T: serde::Serialize>(path: &Utf8Path, v: &T) -> Result<(), RepoError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
            context: format!("could not create {parent}"),
            source,
        })?;
    }
    let text = serde_json::to_string_pretty(v).map_err(|e| RepoError::Message(e.to_string()))?;
    std::fs::write(path, format!("{text}\n")).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })
}

/// The dispatch ids this Warrant's journal says were compiled (§24). A
/// journal that cannot be read is an error, not an empty set: a submission
/// refused for "unknown dispatch" when the journal is corrupt would be
/// refused for the wrong reason.
fn compiled_dispatch_ids(dir: &Utf8Path) -> Result<Vec<String>, RepoError> {
    crate::journal_cmd::load(dir).map(|j| {
        j.events
            .iter()
            .filter(|e| e.event_type == "dispatch.compiled")
            .filter_map(|e| {
                serde_json::from_str::<serde_json::Value>(&e.payload)
                    .ok()
                    .and_then(|v| {
                        v.get("dispatch_id")
                            .and_then(|d| d.as_str())
                            .map(str::to_owned)
                    })
            })
            .collect()
    })
}

/// Write a submission and journal it. The caller has validated it.
fn record_submission(
    repo: &Repository,
    dir: &Utf8Path,
    uuid: &str,
    submission: &StageSubmission,
    actor: &str,
    report: &mut Report,
) -> Result<Utf8PathBuf, RepoError> {
    let path = dir
        .join(SUBMISSIONS_DIR)
        .join(format!("{}.json", submission.dispatch_id));
    write_json(&path, submission)?;
    crate::journal_cmd::record(
        dir,
        uuid,
        "submission.recorded",
        actor,
        &serde_json::json!({
            "dispatch_id": submission.dispatch_id,
            "stage": submission.stage_id,
            "requested_next_action": submission.requested_next_action.map(|a| a.to_string()),
            "blockers": submission.blockers.len(),
            "path": repo.relative(&path),
        })
        .to_string(),
    )?;
    report.push(Diagnostic::pass(
        "submission.recorded",
        format!(
            "{}: submission for dispatch {} requests `{}`",
            repo.relative(&path),
            submission.dispatch_id,
            submission
                .requested_next_action
                .map_or_else(|| "nothing".to_owned(), |a| a.to_string())
        ),
    ));
    Ok(path)
}

/// `war run <alias> <stage>`.
pub fn run(
    repo: &Repository,
    alias: &str,
    stage_id: &str,
    prototype: bool,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let uuid = loaded
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
        .ok_or_else(|| RepoError::Message(format!("{alias}: the manifest does not validate")))?;
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
    let refuse = |report: &mut Report, rule: &'static str, why: String| {
        report.push(Diagnostic::error(
            rule,
            repo.relative(&dir.join("atoms/45-milestones.yaml")),
            why,
        ));
    };
    if stage.executor_kind != ExecutorKind::Service {
        refuse(
            &mut report,
            "run.not-a-service",
            format!(
                "{alias}/{stage_id}: executor_kind is {}; `war run` runs service stages whose \
                 executor_ref names a registered gate. A human stage is done by a human; an agent \
                 stage is dispatched with `war dispatch`",
                stage.executor_kind
            ),
        );
        return Ok(report);
    }
    let Some(key) = stage
        .executor_ref
        .as_deref()
        .and_then(|r| r.strip_prefix("gate://"))
    else {
        refuse(
            &mut report,
            "run.no-gate",
            format!(
                "{alias}/{stage_id}: a service stage names its executor as `gate://<key>`; found {:?}",
                stage.executor_ref
            ),
        );
        return Ok(report);
    };
    let registry = crate::check::load_gate_registry(repo, &mut Report::default());
    let Some(def) = registry.get(key) else {
        refuse(
            &mut report,
            "run.unknown-gate",
            format!(
                "{alias}/{stage_id}: gate://{key} is not in the registry; registered: {}",
                registry
                    .definitions
                    .iter()
                    .map(|d| d.key())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
        return Ok(report);
    };

    // The Dispatch, compiled through the same path `war dispatch` takes and
    // kept under dispatches/ as the record of what ran.
    let scratch = dir
        .join(DISPATCHES_DIR)
        .join(format!("{stage_id}.pending.json"));
    std::fs::create_dir_all(dir.join(DISPATCHES_DIR)).map_err(|source| RepoError::Io {
        context: format!("could not create {}", dir.join(DISPATCHES_DIR)),
        source,
    })?;
    let dispatch_report = crate::dispatch::run(
        repo,
        alias,
        stage_id,
        crate::dispatch::Options {
            attempt_kind: AttemptKind::Initial,
            prior_failure_evidence: &[],
            emit_to: Some(&scratch),
            emit_context_to: None,
            prototype,
        },
    )?;
    if !dispatch_report.is_ready() {
        let _ = std::fs::remove_file(&scratch);
        for d in dispatch_report.diagnostics {
            report.push(d);
        }
        return Ok(report);
    }
    let dispatch: StageDispatch = std::fs::read_to_string(&scratch)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .ok_or_else(|| {
            RepoError::Message(format!(
                "{alias}/{stage_id}: the compiled dispatch could not be read back"
            ))
        })?;
    let dispatch_path = dir
        .join(DISPATCHES_DIR)
        .join(format!("{}.json", dispatch.dispatch_id));
    std::fs::rename(&scratch, &dispatch_path).map_err(|source| RepoError::Io {
        context: format!("could not move {scratch} to {dispatch_path}"),
        source,
    })?;

    // The run, under the smaller of the two bounds, with the gate's own
    // definition otherwise untouched.
    let wall = stage
        .wall_time_seconds
        .unwrap_or_else(|| repo.config.run.wall_time_seconds());
    let bound = def.timeout_secs.map_or(wall, |g| g.min(wall));
    let mut bounded = def.clone();
    bounded.timeout_secs = Some(bound);
    let started_at = now_rfc3339();
    // Each dispatch owns distinct evidence paths. Re-running the same gate
    // must not replace the preceding attempt's streams, run record or receipt.
    let runs_root = dir.join("gate-runs");
    std::fs::create_dir_all(&runs_root).map_err(|source| RepoError::Io {
        context: format!("could not create {runs_root}"),
        source,
    })?;
    let runs_dir = runs_root.join(&dispatch.dispatch_id);
    std::fs::create_dir(&runs_dir).map_err(|source| RepoError::Io {
        context: format!("could not reserve new service attempt directory {runs_dir}"),
        source,
    })?;
    let mut gate_run = crate::gate_cmd::run_gate(&bounded, repo, &runs_dir);
    gate_run.id = format!("GR-{}", dispatch.dispatch_id);
    if let Err(e) = crate::gate_cmd::persist_run(&gate_run, &runs_dir) {
        report.push(Diagnostic::error(
            "gate-run.not-persisted",
            key.to_owned(),
            e,
        ));
    }
    let subject = vec![format!("dispatch:{}", dispatch.dispatch_digest)];
    if gate_run.execution_status == openwarrant_core::ExecutionStatus::Completed {
        match crate::gate_cmd::receipt::mint(
            repo,
            def,
            &gate_run,
            &started_at,
            &gate_run.verdict.to_string(),
            crate::gate_cmd::receipt::Bindings {
                subject_digests: &subject,
                raw_evidence_refs: &[],
            },
            &runs_dir,
        ) {
            Ok(path) => report.push(Diagnostic::pass(
                "gate-run.receipt",
                format!(
                    "{key}: §44.6 receipt bound to the dispatch digest, written to {}",
                    repo.relative(&path)
                ),
            )),
            Err(e) => report.push(Diagnostic::error(
                "gate-run.receipt-failed",
                key.to_owned(),
                e.to_string(),
            )),
        }
    }
    let passed = gate_run.satisfies_required_pass();
    // By variant, not by the Display string: a renamed status is a compile
    // error here, not a silently reclassified run.
    let timed_out = gate_run.execution_status == openwarrant_core::ExecutionStatus::Timeout;
    if !passed && gate_run.execution_status != openwarrant_core::ExecutionStatus::Completed {
        report.push(Diagnostic::warn(
            "run.no-receipt",
            key.to_owned(),
            format!(
                "{alias}/{stage_id}: the run did not complete (execution {}), so no §44.6 receipt \\
                 is minted; the persisted run record and the submission's Blocker are the evidence",
                gate_run.execution_status
            ),
        ));
    }
    let (rule, action, blockers) = if passed {
        ("run.passed", RequestedNextAction::Verify, vec![])
    } else {
        let why = if timed_out {
            format!(
                "gate://{key} exceeded its {bound}s bound (stage wall_time {wall}s, gate timeout {:?})",
                def.timeout_secs
            )
        } else {
            format!(
                "gate://{key} finished {} with verdict {}",
                gate_run.execution_status, gate_run.verdict
            )
        };
        (
            if timed_out {
                "run.timeout"
            } else {
                "run.failed"
            },
            RequestedNextAction::Block,
            vec![Blocker {
                id: "B-001".to_owned(),
                condition_ref: format!("gate://{key}"),
                reason: why,
                owner_ref: format!("agent://{}", repo.performer()),
                required_to_unblock: format!("a passing run of gate://{key} under the bound"),
            }],
        )
    };
    let submission = StageSubmission {
        dispatch_id: dispatch.dispatch_id.clone(),
        attempt_id: dispatch.attempt_id.clone(),
        contract_digest: dispatch.contract_digest.clone(),
        stage_id: stage_id.to_owned(),
        claims: vec![PerformerClaim {
            id: "C-001".to_owned(),
            statement: format!(
                "gate://{key} ran under dispatch {} with verdict {}; the receipt is the evidence, this sentence is not",
                dispatch.dispatch_id, gate_run.verdict
            ),
        }],
        artifact_refs: vec![repo.relative(&dispatch_path)],
        blockers,
        requested_next_action: Some(action),
        ..StageSubmission::default()
    };
    let actor = format!("service://{key}");
    record_submission(repo, &dir, &uuid, &submission, &actor, &mut report)?;
    let line = format!(
        "{alias}/{stage_id}: gate://{key} {} (execution {}, verdict {}) under {bound}s",
        if passed {
            "passed"
        } else if timed_out {
            "timed out"
        } else {
            "failed"
        },
        gate_run.execution_status,
        gate_run.verdict
    );
    if passed {
        report.push(Diagnostic::pass(rule, line));
    } else {
        report.push(Diagnostic::error(rule, repo.relative(&dispatch_path), line));
    }
    Ok(report)
}

/// `war submit <alias> <file>`: ingest an external Stage Submission.
pub fn submit(repo: &Repository, alias: &str, file: &Utf8Path) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let uuid = loaded
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
        .ok_or_else(|| RepoError::Message(format!("{alias}: the manifest does not validate")))?;
    let text = std::fs::read_to_string(file).map_err(|source| RepoError::Io {
        context: format!("could not read {file}"),
        source,
    })?;
    let value: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            report.push(Diagnostic::error(
                "submission.malformed",
                file.to_string(),
                format!("not JSON: {e}"),
            ));
            return Ok(report);
        }
    };
    // §51.2 first, on the raw text: a submission that requests its own
    // completion is refused before it is even shaped into a record.
    if let Some(action) = value.get("requested_next_action").and_then(|a| a.as_str())
        && let Err(e) = StageSubmission::validate_requested_action("submission", action)
    {
        report.push(Diagnostic::error(
            "submission.self-completion",
            file.to_string(),
            format!("{alias}: {e}; a performer's submission may ask to continue, be verified, block, amend or cancel — never to be resolved. Nothing was written"),
        ));
        return Ok(report);
    }
    let submission: StageSubmission = match serde_json::from_value(value) {
        Ok(s) => s,
        Err(e) => {
            report.push(Diagnostic::error(
                "submission.malformed",
                file.to_string(),
                format!("not a Stage Submission: {e}"),
            ));
            return Ok(report);
        }
    };
    if let Err(e) = submission.validate() {
        report.push(Diagnostic::error(
            "submission.malformed",
            file.to_string(),
            e.to_string(),
        ));
        return Ok(report);
    }
    let known = compiled_dispatch_ids(&dir)?;
    if !known.contains(&submission.dispatch_id) {
        report.push(Diagnostic::error(
            "submission.unknown-dispatch",
            file.to_string(),
            format!(
                "{alias}: no `dispatch.compiled` journal event names dispatch {:?}; a submission \
                 answers a Dispatch this Warrant compiled, or it answers nothing. Nothing was written",
                submission.dispatch_id
            ),
        ));
        return Ok(report);
    }
    let actor = format!("agent://{}", repo.performer());
    record_submission(repo, &dir, &uuid, &submission, &actor, &mut report)?;
    Ok(report)
}
