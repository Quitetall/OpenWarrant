// SPDX-License-Identifier: Apache-2.0
//! `war plan` — the drafting seam (§71.3, §74, §75.2) with something on the
//! other side.
//!
//! Until OW-WAR-0042's successor landed here, `war plan` emitted a request and
//! stopped; a Draft Proposal could be validated but never applied, because v1
//! operations carried no payload. Two paths now reach an applied draft, and
//! both end in the same gauntlet:
//!
//! - **The harness is the agent.** A Claude Code session (or any agent reading
//!   AGENTS.md) writes a v2 proposal JSON and runs
//!   `war plan --proposal p.json --reviewed --apply`.
//! - **A configured drafter.** `openwarrant.toml [plan] drafter_argv = […]`
//!   names a process; `war plan "<sentence>" --draft` pipes the request to it
//!   on stdin and reads the proposal from its stdout, then the same
//!   `--proposal` path applies it.
//!
//! Either way, §74.4's eight steps run in order and `--apply` refuses while
//! review is outstanding. §74.5: the model never writes a file. The drafter
//! process is watched for that — `git status --porcelain` before and after —
//! and a drafter that touched the tree is refused, its proposal discarded.
//!
//! What is recorded: `docs/warrants/<alias>/plan/{request,proposal,pipeline,
//! drafter}.json`, and journal events `plan.requested`, `plan.proposed`,
//! `plan.applied`, so an applied draft carries its own provenance — which is
//! the evidence OW-WAR-0042 asked for and could not get.

// OW-WAR-0141: the intake half, a child of this module because the module
// list in `lib.rs` is not that Warrant's to change (the precedent is
// `repo.rs`'s `compat`).
#[path = "intake.rs"]
pub mod intake;

// OW-WAR-0148 M7: a sentence to typed records and the ticket that implements
// them, a child of this module for the same reason.
#[path = "plan_records.rs"]
pub mod records;

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::Profile;
use openwarrant_core::drafting::{APPLICATION_STEPS, ApplicationPipeline, AtomOperation};
use openwarrant_core::drafting_v2::{API_VERSION_V2, DraftProposalV2};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

pub const REQUEST_API: &str = "oh.war/draft-request/v1";
pub const INTERVIEW_SCHEMA: &str = "oh.war/interview/v1";
pub const EVENT_REQUESTED: &str = "plan.requested";
pub const EVENT_PROPOSED: &str = "plan.proposed";
pub const EVENT_APPLIED: &str = "plan.applied";

/// The request handed to a drafting agent (§74.1, §75.2). v1's fields, plus
/// the answers a human gave to an earlier interview round.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub api_version: String,
    pub user_request: String,
    pub namespace: String,
    pub profile: String,
    pub assurance: String,
    #[serde(default)]
    pub existing_warrants: Vec<String>,
    #[serde(default)]
    pub existing_adrs: Vec<String>,
    /// Interview answers by question id (§74.6).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub answers: std::collections::BTreeMap<String, String>,
}

pub fn request(
    repo: &Repository,
    sentence: &str,
    profile: &str,
    assurance: &str,
    answers: &std::collections::BTreeMap<String, String>,
) -> Result<Request, RepoError> {
    // v1's request struct is the canonical shape (show.rs); this is that plus
    // the answers, with `existing_adrs` finally populated.
    let v1 = crate::show::plan::DraftRequest {
        api_version: REQUEST_API.to_owned(),
        user_request: sentence.to_owned(),
        namespace: repo.config.project.namespace.as_str().to_owned(),
        profile: profile.to_owned(),
        assurance: assurance.to_owned(),
        existing_warrants: repo
            .warrant_dirs()?
            .iter()
            .filter_map(|d| d.file_name().map(ToOwned::to_owned))
            .collect(),
        existing_adrs: repo
            .load_adrs()?
            .records
            .iter()
            .map(|a| a.local_alias.clone())
            .collect(),
    };
    Ok(Request {
        api_version: v1.api_version,
        user_request: v1.user_request,
        namespace: v1.namespace,
        profile: v1.profile,
        assurance: v1.assurance,
        existing_warrants: v1.existing_warrants,
        existing_adrs: v1.existing_adrs,
        answers: answers.clone(),
    })
}

/// What a drafter run looked like, recorded beside the proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrafterRun {
    pub name: String,
    pub argv: Vec<String>,
    pub started_at: String,
    pub finished_at: String,
    pub exit_status: String,
    /// Last 64 KiB of stderr — §75.2 "bounded diagnostics".
    pub stderr_tail: String,
    /// `git status --porcelain` lines that appeared during the run. Must be
    /// empty: §74.5, the model never writes a file.
    pub tree_delta: Vec<String>,
}

fn porcelain(repo: &Repository) -> Result<BTreeSet<String>, RepoError> {
    let out = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&repo.root)
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git status".to_owned(),
            source,
        })?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Run the configured drafter: request on stdin, proposal on stdout. The
/// request is a Warrant draft request ([`Request`]) or a records request
/// ([`records::Request`]); the drafter tells them apart by `api_version`.
pub fn run_drafter<R: Serialize>(
    repo: &Repository,
    request: &R,
) -> Result<(String, DrafterRun), RepoError> {
    let policy = &repo.config.plan;
    let Some(program) = policy.drafter_argv.first() else {
        return Err(RepoError::Message(
            "no drafter is configured: set [plan] drafter_argv in openwarrant.toml, or hand the \
             request to an agent yourself and return its proposal with --proposal"
                .to_owned(),
        ));
    };
    let before = porcelain(repo)?;
    let started = Instant::now();
    let started_at = crate::gate_cmd::receipt::now_rfc3339_public();
    let mut child = Command::new(program)
        .args(&policy.drafter_argv[1..])
        .current_dir(&repo.root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| RepoError::Io {
            context: format!("could not run the drafter {program}"),
            source,
        })?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        let body = serde_json::to_string_pretty(request)
            .map_err(|e| RepoError::Message(format!("could not render the request: {e}")))?;
        // A drafter that exits before reading is reported by its status, not
        // by a broken pipe here.
        let _ = stdin.write_all(body.as_bytes());
    }
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.read_to_string(&mut out);
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        (out, err)
    });
    let deadline = Duration::from_secs(policy.timeout_secs());
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|source| RepoError::Io {
            context: "could not wait for the drafter".to_owned(),
            source,
        })? {
            break Some(s);
        }
        if started.elapsed() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let (out, err) = reader.join().unwrap_or_default();
    let tail_start = err.len().saturating_sub(64 * 1024);
    let run = DrafterRun {
        name: if policy.drafter_name.is_empty() {
            program.clone()
        } else {
            policy.drafter_name.clone()
        },
        argv: policy.drafter_argv.clone(),
        started_at,
        finished_at: crate::gate_cmd::receipt::now_rfc3339_public(),
        exit_status: status.map_or_else(
            || format!("killed after {}s", deadline.as_secs()),
            |s| s.to_string(),
        ),
        stderr_tail: String::from_utf8_lossy(&err[tail_start..]).into_owned(),
        tree_delta: porcelain(repo)?.difference(&before).cloned().collect(),
    };
    if status.is_none() {
        return Err(RepoError::Message(format!(
            "plan.drafter-timeout: the drafter did not finish within {}s and was killed",
            deadline.as_secs()
        )));
    }
    if !run.tree_delta.is_empty() {
        return Err(RepoError::Message(format!(
            "plan.drafter-wrote-files: the drafter changed the working tree, which §74.5 \
             forbids — {} path(s): {}. Its proposal is discarded",
            run.tree_delta.len(),
            run.tree_delta.join(", ")
        )));
    }
    if !status.is_some_and(|s| s.success()) {
        return Err(RepoError::Message(format!(
            "plan.drafter-failed: the drafter exited {}; stderr tail:\n{}",
            run.exit_status, run.stderr_tail
        )));
    }
    Ok((out, run))
}

/// §74.4 steps 1–4 for a v2 proposal, plus the interview (§74.6). Steps 5–6
/// are recorded only when `reviewed` says a human did them.
pub fn validate_v2(
    json: &str,
    reviewed: bool,
    answers: &BTreeSet<String>,
    known_refs: &BTreeSet<String>,
) -> Result<(DraftProposalV2, ApplicationPipeline), RepoError> {
    let mut pipeline = ApplicationPipeline::default();
    let proposal: DraftProposalV2 = serde_json::from_str(json)
        .map_err(|e| RepoError::Message(format!("draft proposal did not parse: {e}")))?;
    pipeline.complete(APPLICATION_STEPS[0]);
    if proposal.api_version != API_VERSION_V2 {
        return Err(RepoError::Message(format!(
            "plan.v1-has-no-payloads: api_version {:?} is not {API_VERSION_V2}. A v1 proposal \
             validates (`war plan --proposal` without --apply) but carries no operation \
             payloads, so it cannot be applied",
            proposal.api_version
        )));
    }
    pipeline.complete(APPLICATION_STEPS[1]);
    proposal
        .validate()
        .map_err(|e| RepoError::Message(e.to_string()))?;
    pipeline.complete(APPLICATION_STEPS[2]);
    // §74.6 / §91.8 test 58 — a blocker with no answer stops the run.
    if let Err(e) = openwarrant_core::drafting::require_blockers_answered(
        &proposal.unresolved_questions,
        answers,
    ) {
        return Err(RepoError::Message(format!(
            "plan.interview-required: {e}. Answer with --answer <id>=<text>; the questions are:\n{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": INTERVIEW_SCHEMA,
                "questions": openwarrant_core::drafting::minimum_question_set(&proposal.unresolved_questions),
            }))
            .unwrap_or_default()
        )));
    }
    // §74.4 step 3 / §91.8 test 55 — an invented `war://` blocks.
    let invented: Vec<&str> = proposal
        .cited_warrants()
        .into_iter()
        .filter(|r| !known_refs.contains(*r))
        .collect();
    if !invented.is_empty() {
        return Err(RepoError::Message(format!(
            "the proposal cites {} source reference(s) this repository cannot resolve: {}",
            invented.len(),
            invented
                .iter()
                .map(|r| format!("{r:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    pipeline.complete(APPLICATION_STEPS[3]);
    if reviewed {
        pipeline.complete(APPLICATION_STEPS[4]);
        pipeline.complete(APPLICATION_STEPS[5]);
    }
    Ok((proposal, pipeline))
}

/// Every `war://` the corpus can resolve.
pub fn known_refs(repo: &Repository) -> Result<BTreeSet<String>, RepoError> {
    let mut known = BTreeSet::new();
    for dir in repo.warrant_dirs()? {
        // A Warrant that fails to LOAD is not a Warrant that does not exist.
        // Swallowing the error would report a real reference as invented — the
        // wrong diagnosis, and the more alarming one, for a corrupt file.
        let loaded = repo.load_warrant(&dir).map_err(|e| {
            RepoError::Message(format!(
                "cannot resolve proposal references: {} failed to load ({e}). Refusing to \
                 report a reference as invented when the corpus could not be read.",
                repo.relative(&dir)
            ))
        })?;
        if let Some(v) = loaded.validated {
            known.insert(format!("war://{}", v.uuid));
        }
    }
    Ok(known)
}

/// What `--apply` produced.
#[derive(Debug, Clone, Serialize)]
pub struct Applied {
    pub alias: String,
    pub dir: String,
    pub operations_applied: usize,
    pub adrs_proposed: Vec<String>,
    pub pipeline: ApplicationPipeline,
    /// Who stood in §74.4's review step: a person (`reviewed`) or the
    /// repository's `[intake] policy_approval` (`policy`).
    pub review: Review,
    /// The ticket this Warrant was drafted from, when there was one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intake: Option<intake::IntakeRecord>,
}

fn frontmatter(uuid: &str, role: &str, ordinal: u32) -> String {
    format!(
        "---\nschema: oh.war/atom/v1\nwarrant_uuid: {uuid}\nrole: {role}\njurisdiction: authored\norder: {ordinal}\nclassification: internal\n---\n\n"
    )
}

/// The seven §74.3 operations, applied inside `docs/warrants/<alias>/` and
/// `docs/adr/atoms/` and nowhere else. Never a raw file write from the model:
/// every byte here is placed by this function from a validated request.
pub fn apply(
    repo: &Repository,
    proposal: &DraftProposalV2,
    pipeline: &mut ApplicationPipeline,
    request: &Request,
    run: Option<&DrafterRun>,
    proposal_json: &str,
) -> Result<(Applied, Report), RepoError> {
    apply_with(
        repo,
        proposal,
        pipeline,
        request,
        run,
        proposal_json,
        Review::Reviewed,
        None,
    )
}

/// [`apply`], naming who stood in the review step and the ticket, if any,
/// the request came from. The ticket's record lands at `plan/intake.json`
/// beside `plan/request.json`; nothing else about the application changes.
#[allow(clippy::too_many_arguments)]
pub fn apply_with(
    repo: &Repository,
    proposal: &DraftProposalV2,
    pipeline: &mut ApplicationPipeline,
    request: &Request,
    run: Option<&DrafterRun>,
    proposal_json: &str,
    review: Review,
    intake_record: Option<&intake::IntakeRecord>,
) -> Result<(Applied, Report), RepoError> {
    pipeline
        .may_apply()
        .map_err(|e| RepoError::Message(format!("§74.4: {e}")))?;
    let profile: Profile = proposal
        .proposed_identity
        .profile
        .parse()
        .map_err(|e| RepoError::Message(format!("proposed profile: {e}")))?;
    let dir = crate::new::run(repo, &proposal.proposed_identity.title, profile)?;
    let alias = dir.file_name().unwrap_or_default().to_owned();
    let manifest_path = dir.join("manifest.toml");
    let manifest_text =
        std::fs::read_to_string(&manifest_path).map_err(|source| RepoError::Io {
            context: format!("could not read {manifest_path}"),
            source,
        })?;
    let uuid = manifest_text
        .lines()
        .find_map(|l| {
            l.strip_prefix("uuid = ")
                .map(|v| v.trim_matches('"').to_owned())
        })
        .ok_or_else(|| RepoError::Message(format!("{alias}: manifest has no uuid")))?;
    let mut manifest_append = String::new();
    let mut adrs = Vec::new();
    let mut applied = 0usize;
    // A refusal below must not leave a half-made Warrant behind: the alias
    // was allocated by `war new` seconds ago and nothing else references it.
    let rollback = |why: RepoError| -> RepoError {
        let _ = std::fs::remove_dir_all(&dir);
        why
    };
    let result: Result<(), RepoError> = (|| {
        for op in &proposal.operations {
            match op.op {
                AtomOperation::CreateAtom => {
                    let path = dir.join("atoms").join(&op.path);
                    // `war new` just created this directory, so anything already
                    // at this path is the profile's stub and the proposal's body
                    // replaces it. A stub is already declared in the manifest; a
                    // new atom is appended to it.
                    let declared = manifest_text.contains(&format!("path = \"atoms/{}\"", op.path));
                    // Markdown atoms carry frontmatter; structured atoms (YAML,
                    // OW-ADR-0003) are declared by the manifest alone and any
                    // `---` inside them is a stream marker the reader refuses.
                    let bytes = if op.path.ends_with(".md") {
                        frontmatter(&uuid, &op.role, op.ordinal) + &op.body
                    } else {
                        op.body.clone()
                    };
                    std::fs::write(&path, bytes).map_err(|source| RepoError::Io {
                        context: format!("could not write {path}"),
                        source,
                    })?;
                    if !declared {
                        manifest_append.push_str(&format!(
                        "[[atoms]]\nordinal = {}\nrole = \"{}\"\npath = \"atoms/{}\"\nrequired = true\n\n",
                        op.ordinal, op.role, op.path
                    ));
                    }
                    applied += 1;
                }
                AtomOperation::ReviseAtom => {
                    // §28.7 — an authorized contract is immutable. A brand-new
                    // Warrant cannot be authorized yet, so this is reachable only
                    // through a future "revise an existing draft" path; refuse
                    // rather than pretend.
                    return Err(RepoError::Message(format!(
                        "plan.op-unsupported: revise_atom targets an existing atom; `--apply` \
                     creates a NEW Warrant. Revise a draft by hand or re-propose ({})",
                        op.target
                    )));
                }
                AtomOperation::RetireAtom
                | AtomOperation::AddBinding
                | AtomOperation::RemoveBinding => {
                    return Err(RepoError::Message(format!(
                        "plan.op-unsupported: {} is not applicable to a newly created Warrant in \
                     this build; the operation is validated, recorded, and refused here by name",
                        op.op
                    )));
                }
                AtomOperation::AddRelation => {
                    match op.relation_kind.as_str() {
                        "implements" => manifest_append.push_str(&format!(
                            "[[implements]]\nref = \"{}\"\ncontribution = \"partial\"\n\n",
                            op.relation_ref
                        )),
                        "roadmap" => manifest_append
                            .push_str(&format!("[[roadmap]]\nref = \"{}\"\n\n", op.relation_ref)),
                        "parent" => manifest_append
                            .push_str(&format!("[[parents]]\nref = \"{}\"\n\n", op.relation_ref)),
                        other => {
                            return Err(RepoError::Message(format!(
                                "unknown relation kind {other:?}"
                            )));
                        }
                    }
                    applied += 1;
                }
                AtomOperation::ProposeAdr => {
                    let adr_alias = propose_adr(repo, &uuid, &op.adr_title, &op.body)?;
                    adrs.push(adr_alias);
                    applied += 1;
                }
            }
        }
        Ok(())
    })();
    result.map_err(rollback)?;
    if !manifest_append.is_empty() {
        // Append-only: the manifest keeps its comments and order.
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&manifest_path)
            .map_err(|source| RepoError::Io {
                context: format!("could not append to {manifest_path}"),
                source,
            })?;
        f.write_all(b"\n")
            .and_then(|()| f.write_all(manifest_append.as_bytes()))
            .map_err(|source| RepoError::Io {
                context: format!("could not append to {manifest_path}"),
                source,
            })?;
    }
    pipeline.complete(APPLICATION_STEPS[6]);

    // The record: what was asked, what came back, how, and the gauntlet.
    let plan_dir = dir.join("plan");
    std::fs::create_dir_all(&plan_dir).map_err(|source| RepoError::Io {
        context: format!("could not create {plan_dir}"),
        source,
    })?;
    let write = |name: &str, body: String| -> Result<(), RepoError> {
        let p = plan_dir.join(name);
        std::fs::write(&p, body).map_err(|source| RepoError::Io {
            context: format!("could not write {p}"),
            source,
        })
    };
    write(
        "request.json",
        serde_json::to_string_pretty(request).unwrap_or_default(),
    )?;
    write("proposal.json", proposal_json.to_owned())?;
    if let Some(record) = intake_record {
        write(
            intake::RECORD_FILE,
            serde_json::to_string_pretty(record).unwrap_or_default(),
        )?;
    }
    if let Some(r) = run {
        write(
            "drafter.json",
            serde_json::to_string_pretty(r).unwrap_or_default(),
        )?;
    }
    let performer = repo.performer();
    let req_digest = openwarrant_compiler::sha256_hex(
        serde_json::to_string(request)
            .unwrap_or_default()
            .as_bytes(),
    );
    let prop_digest = openwarrant_compiler::sha256_hex(proposal_json.as_bytes());
    crate::journal_cmd::record(
        &dir,
        &uuid,
        EVENT_REQUESTED,
        &format!("agent://{performer}"),
        &serde_json::json!({"request_digest": format!("sha256:{req_digest}")}).to_string(),
    )?;
    crate::journal_cmd::record(
        &dir,
        &uuid,
        EVENT_PROPOSED,
        &format!(
            "agent://{}",
            run.map_or(performer.as_str(), |r| r.name.as_str())
        ),
        &serde_json::json!({"proposal_digest": format!("sha256:{prop_digest}"), "operations": proposal.operations.len()}).to_string(),
    )?;

    // Step 8: check the Warrant that now exists. Its own atoms only: the
    // corpus projections are stale the moment a Warrant is added, and
    // recompiling them is the caller's next act (`war compile`), not a side
    // effect of drafting.
    let report = crate::check::run(repo, Some(&alias), false)?;
    if report.is_ready() {
        pipeline.complete(APPLICATION_STEPS[7]);
    } else {
        pipeline.fail(APPLICATION_STEPS[7], "war check reported errors");
    }
    write(
        "pipeline.json",
        serde_json::to_string_pretty(&PipelineRecord { pipeline, review }).unwrap_or_default(),
    )?;
    crate::journal_cmd::record(
        &dir,
        &uuid,
        EVENT_APPLIED,
        &format!("agent://{performer}"),
        &serde_json::json!({"operations_applied": applied, "adrs": adrs, "check_ready": report.is_ready()}).to_string(),
    )?;
    let mut report = report;
    report.push(Diagnostic::pass(
        "plan.applied",
        format!(
            "{alias}: {applied} operation(s) applied, {} ADR(s) proposed; request, proposal and pipeline recorded under plan/; run `war admin compile` to project it",
            adrs.len()
        ),
    ));
    Ok((
        Applied {
            alias,
            dir: repo.relative(&dir),
            operations_applied: applied,
            adrs_proposed: adrs,
            pipeline: pipeline.clone(),
            review,
            intake: intake_record.cloned(),
        },
        report,
    ))
}

/// A proposed ADR (§74.7): `docs/adr/atoms/<NS>-ADR-NNNN-<slug>.md`, status
/// proposed, governing the new Warrant. A human accepts it later.
fn propose_adr(
    repo: &Repository,
    warrant_uuid: &str,
    title: &str,
    body: &str,
) -> Result<String, RepoError> {
    let dir = repo.adr_atoms_dir();
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let ns = repo.config.project.namespace.as_str().to_owned();
    let existing = repo.load_adrs()?;
    let next = existing
        .records
        .iter()
        .filter_map(|a| a.local_alias.rsplit('-').next()?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
        + 1;
    let alias = format!("{ns}-ADR-{next:04}");
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(6)
        .collect::<Vec<_>>()
        .join("-");
    let path: Utf8PathBuf = dir.join(format!("{alias}-{slug}.md"));
    let adr_uuid = uuid_v4_like();
    let text = format!(
        "---\nschema: oh.war/atom/v1\nadr_uuid: {adr_uuid}\nlocal_alias: {alias}\nrole: adr\njurisdiction: bound\norder: 30\nclassification: internal\nstatus: proposed\ngoverns:\n  - \"war://{warrant_uuid}\"\n---\n\n# ADR {}: {title}\n\n## Status\n\nProposed by a drafting agent through `war plan --apply`; adopted when a human accepts it.\n\n{body}\n",
        alias.replace("-ADR-", "-")
    );
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| RepoError::Io {
            context: format!("could not create {path}"),
            source,
        })?;
    f.write_all(text.as_bytes())
        .map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    Ok(alias)
}

/// A random-looking id for an ADR atom, minted from a UUIDv7 so it needs no
/// extra dependency and stays unique.
fn uuid_v4_like() -> String {
    openwarrant_core::WarUuid::mint().to_string()
}

/// Where a `--draft` proposal is written when `--out` is not given.
pub fn default_out(repo: &Repository) -> Utf8PathBuf {
    repo.root
        .join("docs/warrants/generated")
        .join(format!("draft-proposal-{}.json", std::process::id()))
}

#[must_use]
pub fn scratch_note(path: &Utf8Path) -> String {
    format!(
        "# proposal written to {path}; review it, then `war plan --proposal {path} --reviewed --apply`"
    )
}

// ---------------------------------------------------------------------------
// Intake (OW-WAR-0141): a ticket or a sentence, handed to the same gauntlet.
// ---------------------------------------------------------------------------

/// Who stood in §74.4's step 6, "require review or policy approval".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    /// `--reviewed`: the caller says a person read the semantic diff.
    Reviewed,
    /// `[intake] policy_approval = true` on an intake apply. Nobody read the
    /// proposal, and the record says so; the human's review is the
    /// authorization of the contract, which still waits for them.
    Policy,
    /// Neither. `--apply` is refused at §74.4's review step.
    None,
}

impl Review {
    #[must_use]
    pub const fn completes_the_step(self) -> bool {
        !matches!(self, Self::None)
    }
}

/// `plan/pipeline.json`: the gauntlet's state, and who stood in its review.
#[derive(Serialize)]
struct PipelineRecord<'a> {
    #[serde(flatten)]
    pipeline: &'a ApplicationPipeline,
    review: Review,
}

/// What `war plan` was handed in place of, or as, a sentence.
#[derive(Debug, Clone)]
pub struct Intake {
    /// The sentence the draft request carries.
    pub sentence: String,
    /// `github-<n>` or `sentence-<digest>`: where a question waits.
    pub key: String,
    pub issue: Option<intake::Issue>,
    /// `--issue` (fetched) rather than `--issue-file`.
    pub fetched: bool,
    /// Answers a human already gave to this input's questions, by id.
    pub answers: BTreeMap<String, String>,
}

impl Intake {
    #[must_use]
    pub fn record(&self) -> Option<intake::IntakeRecord> {
        self.issue.as_ref().map(intake::Issue::record)
    }
}

/// Read the input: `--issue <id>`, `--issue-file <path>`, or the sentence.
/// Nothing is written here; a refusal leaves the tree as it was.
pub fn resolve_intake(
    repo: &Repository,
    sentence: &str,
    issue: Option<&str>,
    issue_file: Option<&Utf8Path>,
) -> Result<Option<Intake>, RepoError> {
    if (issue.is_some() || issue_file.is_some()) && !sentence.trim().is_empty() {
        return Err(RepoError::Message(
            "plan.intake-and-sentence: give a sentence or an issue, not both; the issue's title \
             and body are the request"
                .to_owned(),
        ));
    }
    let (issue, fetched) = match (issue, issue_file) {
        (Some(_), Some(_)) => {
            return Err(RepoError::Message(
                "plan.intake-twice: --issue and --issue-file name two inputs; give one".to_owned(),
            ));
        }
        (Some(id), None) => (Some(intake::fetch(repo, id)?), true),
        (None, Some(path)) => (Some(intake::read_file(path)?), false),
        (None, None) => (None, false),
    };
    let (sentence, key) = match &issue {
        Some(i) => (i.sentence(), i.key()),
        None if sentence.trim().is_empty() => return Ok(None),
        None => (sentence.to_owned(), intake::sentence_key(sentence)),
    };
    let answers = if intake::existing_dir(repo, &key).is_some() {
        crate::questions::load(repo, &key)?
            .into_iter()
            .filter_map(|q| q.answer.map(|a| (q.id, a.answer)))
            .collect()
    } else {
        BTreeMap::new()
    };
    Ok(Some(Intake {
        sentence,
        key,
        issue,
        fetched,
        answers,
    }))
}

/// Who stands in the review step. An intake apply — a proposal from the
/// configured drafter, or one carrying a ticket — is under policy when the
/// repository says so, and then `--reviewed` is not recorded: a claim that a
/// person read a proposal nobody read is the false record U-001 forbids.
pub fn review_of(
    repo: &Repository,
    intake: Option<&Intake>,
    drafted: bool,
    reviewed: bool,
) -> Result<Review, RepoError> {
    let intake_path = intake.is_some_and(|i| i.issue.is_some() || drafted);
    if intake_path && repo.intake_policy()?.is_some_and(|p| p.policy_approval) {
        return Ok(Review::Policy);
    }
    Ok(if reviewed {
        Review::Reviewed
    } else {
        Review::None
    })
}

/// The command that drafts this input again once its questions are
/// answered. The answers are read from `docs/intake/<key>/questions/`, so
/// the command carries none of them.
pub fn redraft_command(repo: &Repository, i: &Intake, request: &Request) -> String {
    let input = match (&i.issue, i.fetched) {
        (Some(issue), true) => format!("--issue {}", issue.number),
        (Some(_), false) => format!(
            "--issue-file {}",
            intake::shell_quote(&format!(
                "{}/{}/{}",
                intake::STORE,
                i.key,
                intake::ISSUE_FILE
            ))
        ),
        (None, _) => intake::shell_quote(i.sentence.trim()),
    };
    let mut cmd = format!("war plan {input}");
    if request.profile != "delivery" {
        cmd.push_str(&format!(
            " --profile {}",
            intake::shell_quote(&request.profile)
        ));
    }
    if request.assurance != "basic" {
        cmd.push_str(&format!(
            " --assurance {}",
            intake::shell_quote(&request.assurance)
        ));
    }
    let policy = repo
        .intake_policy()
        .ok()
        .flatten()
        .is_some_and(|p| p.policy_approval);
    cmd.push_str(if policy {
        " --draft --apply"
    } else {
        " --draft --reviewed --apply"
    });
    cmd
}

pub const INTAKE_QUESTIONS_SCHEMA: &str = "oh.war/intake-questions/v1";

/// What a thin input produced: questions, and no Warrant.
#[derive(Debug, Clone, Serialize)]
pub struct Asked {
    pub schema: &'static str,
    pub key: String,
    pub dir: String,
    pub questions: Vec<String>,
    pub redraft: String,
}

/// §74.6 on the intake path (U-002). When the drafter answered with a
/// blocker nobody has answered yet, the question goes to
/// `docs/intake/<key>/` — beside the request and, for a ticket, its intake
/// record — and no alias is allocated: a Warrant with nothing in it is the
/// invented Warrant this path exists to prevent. `war plan questions`, `war
/// answer` and `war next` read it from there.
///
/// `None` when this is not that case, and the caller reports the error as
/// it always did.
pub fn record_questions(
    repo: &Repository,
    intake: Option<&Intake>,
    run: Option<&DrafterRun>,
    request: &Request,
    proposal_json: &str,
    error: &RepoError,
) -> Result<Option<(Report, Asked)>, RepoError> {
    let Some(i) = intake else {
        return Ok(None);
    };
    if run.is_none() && i.issue.is_none() {
        return Ok(None);
    }
    if !error.to_string().starts_with("plan.interview-required") {
        return Ok(None);
    }
    let proposal: DraftProposalV2 = serde_json::from_str(proposal_json)
        .map_err(|e| RepoError::Message(format!("draft proposal did not parse: {e}")))?;
    let open: Vec<_> =
        openwarrant_core::drafting::minimum_question_set(&proposal.unresolved_questions)
            .into_iter()
            .filter(|q| !i.answers.contains_key(&q.id))
            .collect();
    if open.is_empty() {
        return Ok(None);
    }
    if let Some(bad) = open.iter().find(|q| !intake::valid_question_id(&q.id)) {
        return Err(RepoError::Message(format!(
            "plan.question-id: the drafter's question id {:?} is not a file name (letters, \
             digits, `-`, `_`). Nothing was written",
            bad.id
        )));
    }
    let dir = intake::store_dir(repo, &i.key);
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let write = |name: &str, body: String| -> Result<(), RepoError> {
        let p = dir.join(name);
        std::fs::write(&p, body).map_err(|source| RepoError::Io {
            context: format!("could not write {p}"),
            source,
        })
    };
    write(
        "request.json",
        serde_json::to_string_pretty(request).unwrap_or_default(),
    )?;
    if let Some(issue) = &i.issue {
        write(
            intake::RECORD_FILE,
            serde_json::to_string_pretty(&issue.record()).unwrap_or_default(),
        )?;
        write(
            intake::ISSUE_FILE,
            serde_json::to_string_pretty(issue).unwrap_or_default(),
        )?;
    }
    let redraft = redraft_command(repo, i, request);
    let asker = format!(
        "agent://{}",
        run.map_or_else(|| repo.performer(), |r| r.name.clone())
    );
    let mut report = Report::default();
    let mut ids = Vec::new();
    for q in &open {
        let path =
            crate::questions::record_intake(repo, &i.key, &q.id, &q.question, &asker, &redraft)?;
        report.push(Diagnostic::warn(
            "plan.question-recorded",
            repo.relative(&path),
            format!(
                "{}/{}: {} — no Warrant was drafted and no alias allocated",
                i.key, q.id, q.question
            ),
        ));
        ids.push(q.id.clone());
    }
    report.note(format!(
        "A human answers: `war plan answer {} <id> \"<answer>\" --as <actor>`. Then the input is \
         drafted again with its answers: `{redraft}`",
        i.key
    ));
    Ok(Some((
        report,
        Asked {
            schema: INTAKE_QUESTIONS_SCHEMA,
            key: i.key.clone(),
            dir: repo.relative(&dir),
            questions: ids,
            redraft,
        },
    )))
}
