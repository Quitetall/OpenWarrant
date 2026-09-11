// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war sign` — the human's half of every seam, as one command at a terminal.
//!
//! # Why this exists
//!
//! Every authority act in this repository is a two-half seam: an agent emits a
//! request, a human ingests a signed response. That shape is right (§27.2) and
//! it was, in practice, three steps: emit the request to a file, hand-write a
//! response with six fields and one `[[judgment]]` block per residual risk,
//! then ingest it. Nobody hand-writes fifty-seven of those. What happened
//! instead is that an agent drafted every response and the human said "sign
//! it" — which is a signature in name only, and the friction of the three
//! steps was the only thing stopping the agent from running the third too.
//!
//! `war sign` collapses the human's part to one screen and one keystroke,
//! and replaces friction with a control that is actually a control:
//!
//! - **It refuses without a terminal.** An agent's shell has no TTY, so the
//!   command cannot be run from inside one. This is a speed bump against an
//!   agent completing the loop by accident or by "helpfulness", not a
//!   cryptographic guarantee; a pseudo-terminal defeats it.
//! - **`--ssh-sign` is the stronger form, and the one that works from inside
//!   an agent session.** Measured: Claude Code's `!` prefix runs in the agent's
//!   own shell and has no TTY, so the terminal path refuses it — correctly, and
//!   uselessly for a human sitting in that session. `--ssh-sign` signs the
//!   response file's bytes with `ssh-keygen -Y sign` under namespace
//!   `oh.war/response`, verifies at once against a human-written
//!   `docs/authority/allowed_signers`, and refuses if it does not verify. No
//!   prompt, no TTY: the human act is the ssh agent's confirmation dialog,
//!   which exists only if the key was loaded with `ssh-add -c`. Without `-c`
//!   the AI agent's shell can reach the same socket and sign as the human, and
//!   `war` cannot tell the two apart. That is the one thing the operator must
//!   get right, and the docs say so where the key is configured.
//! - **It shows what is being signed, not the TOML.** Title, revision, what
//!   changed (for an amendment), obligations, every residual risk with its
//!   consequence, the digest. Those are the things the signer is accepting.
//!   Everything else in a response is boilerplate.
//! - **The drafted wording is a template, and says so.** `authorize.rs` is
//!   right that pre-filling the meaning would make the signer a signatory to
//!   text an agent wrote. This is the other case: text a deterministic tool
//!   rendered from the record's own facts, shown to the signer on a terminal
//!   before they confirm, and recorded as `signed_via = "tty"`. That is more
//!   honest than the paragraph-per-response practice it replaces.
//! - **It reuses every existing refusal.** On `y` it writes the response to
//!   `docs/authority/responses/` and calls the same ingest that a hand-written
//!   response goes through. Agent-by-kind, stale digest, self-authorization,
//!   missing amendment, unsupported outcome: nothing here bypasses them.
//!
//! There is deliberately no `--yes`. A flag that skipped the prompt would be
//! the first thing an agent reached for.
//!
//! # What this could not touch, and why
//!
//! `signed_via = "tty"` is a field on the authorization response only.
//! `resolution_cmd.rs`, `sas.rs` and `conformance/plant.sh` are pinned
//! deliverables of resolved Warrants (0059, 0058, 0063); editing them raises
//! `deliverable.digest-drift`, and the act that would let them move for a
//! reason is OW-WAR-0064, not yet authorized. So a resolution or SAS
//! acceptance signed here carries its provenance in the `meaning` text, and
//! the no-terminal refusal is proved by a unit test rather than a §92 plant.
//! Both move into their proper places when 0064 lands.

use std::io::{IsTerminal, Write};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_core::{
    Independence, JudgmentAuthority, epistemic::Judgment, resolution::CommonOutcome,
    sas::SasRevisionState,
};

use crate::{
    authorize::{self, AuthorizationRequest, AuthorizationResponse},
    diagnostic::{Diagnostic, Report},
    repo::{RepoError, Repository},
    resolution_cmd::{self, ResolutionRequest, ResolutionResponse},
    sas::{self, AcceptRequest, AcceptResponse},
};

/// How the signature reached the record. Written into the response and the
/// journal so a later reader can tell a terminal confirmation from a
/// hand-written file — or, later, from an ssh signature.
pub const CHANNEL_TTY: &str = "tty";
/// Signed with `ssh-keygen -Y sign`; the `.sig` sidecar beside the response
/// verifies against `docs/authority/allowed_signers`.
pub const CHANNEL_SSH: &str = "ssh";
/// The `-n` namespace every signature is made and verified under. A signature
/// made for another purpose with the same key does not verify here.
pub const SSH_NAMESPACE: &str = "oh.war/response";

/// One act awaiting a human.
#[derive(Debug)]
pub enum Pending {
    Authorize {
        alias: String,
        request: AuthorizationRequest,
        /// Present when this would be revision N+1 under an amendment.
        amendment: Option<AmendmentSummary>,
        revision: u32,
    },
    Resolve {
        alias: String,
        request: ResolutionRequest,
        profile: String,
    },
    Accept {
        version: String,
        request: AcceptRequest,
    },
    /// OW-WAR-0064 — a resolved Warrant's delivered artifact has moved and a
    /// human must say why, or restore it.
    Correct {
        alias: String,
        deliverable_id: String,
        request: crate::correct::CorrectionRequest,
    },
}

/// What an amendment says it changed, read for display only. Validation of the
/// amendment itself belongs to `authorize::ingest`, which counts and checks
/// the records; this reads the human-facing lines so the signer sees them.
#[derive(Debug, Clone, Default)]
pub struct AmendmentSummary {
    pub id: String,
    pub reason: String,
    pub changes: Vec<(String, String, String)>,
}

/// What the signer chose, beyond `y`.
#[derive(Debug, Clone)]
pub struct Options {
    /// Sign as this actor. Required when the register lists more than one
    /// eligible human.
    pub actor: Option<String>,
    /// Appended to the templated meaning, verbatim.
    pub meaning: Option<String>,
    /// A resolution outcome when §38.6 does not permit `satisfied`.
    pub outcome: Option<CommonOutcome>,
    /// §101.3 — required when the SAS revision is architecture-changing.
    pub adr_ref: Option<String>,
    pub independence: Independence,
    /// Open `$EDITOR` on the drafted response before the prompt.
    pub edit: bool,
    /// Sign every pending act, prompting once each.
    pub all: bool,
    /// Render the screen and stop: no prompt, no terminal needed, nothing
    /// written. For reading what a signature would say from anywhere.
    pub show: bool,
    /// Sign the response file with the actor's ssh key via `ssh-keygen -Y
    /// sign`, verified at once against `docs/authority/allowed_signers`. No
    /// terminal needed: the human act is the agent's confirmation dialog
    /// (`ssh-add -c`), which no shell can answer.
    pub ssh_sign: bool,
    /// Verify an existing response's `.sig` sidecar and stop. Writes nothing.
    pub verify: bool,
    /// For a correction: what kind of change it admits (OW-WAR-0064).
    pub kind: Option<openwarrant_core::correction::CorrectionKind>,
}

impl Default for Options {
    /// `separate_role` is the honest default for a sole-owner repository:
    /// §27.4 says role separation by one person is not organizational
    /// independence, and `None` would claim the signer did the work.
    fn default() -> Self {
        Self {
            actor: None,
            meaning: None,
            outcome: None,
            adr_ref: None,
            independence: Independence::SeparateRole,
            edit: false,
            all: false,
            show: false,
            ssh_sign: false,
            verify: false,
            kind: None,
        }
    }
}

impl Options {
    /// Which channel a signature made under these options records.
    fn channel(&self) -> &'static str {
        if self.ssh_sign {
            CHANNEL_SSH
        } else {
            CHANNEL_TTY
        }
    }
}

/// The version string written into every drafted meaning, so a record says
/// which template produced it.
fn tool_version() -> String {
    format!("war sign {}", env!("CARGO_PKG_VERSION"))
}

/// Everything awaiting a signature, in alias order, SAS revisions last.
pub fn pending(repo: &Repository) -> Result<Vec<Pending>, RepoError> {
    // Loaded once, up front, and NOT swallowed. Every per-Warrant request
    // below reads the register, and their errors are skipped so one broken
    // Warrant does not hide the rest — which meant a broken register (a
    // duplicate key in roles.toml, found the first time it was hand-edited)
    // read as "nothing awaits a signature". A trust root that will not parse
    // is the loudest thing this command can say, not the quietest.
    repo.load_authority_register().map_err(|e| {
        RepoError::Message(format!("docs/authority/roles.toml will not load; nothing can be signed or listed until it does: {e}"))
    })?;
    let mut out = Vec::new();
    let mut dirs = repo.warrant_dirs()?;
    dirs.sort();
    for dir in dirs {
        let Some(alias) = dir.file_name().map(str::to_owned) else {
            continue;
        };
        let Ok(one) = repo.load_warrant(&dir) else {
            continue;
        };
        if one.validated.is_none() || one.basis.is_none() {
            continue;
        }
        let Ok(request) = authorize::request(repo, &alias) else {
            continue;
        };
        let authorization = repo.load_authorization(&dir)?;
        let authorized_current = authorization
            .as_ref()
            .is_some_and(|a| authorize::authorizes_current_contract(a, &request.contract_digest));
        if !authorized_current {
            let revision = authorization
                .as_ref()
                .map_or(1, |a| a.revision.revision + 1);
            let amendment = if revision > 1 {
                read_latest_amendment(&dir)
            } else {
                None
            };
            out.push(Pending::Authorize {
                alias,
                request,
                amendment,
                revision,
            });
            continue;
        }
        if repo.load_resolution(&dir)?.is_some() {
            // Resolved: the only act left is a correction, and only when a
            // content-addressed deliverable no longer matches its chain head.
            let deliverables = repo.load_deliverables(&dir)?;
            for d in deliverables.records.iter().filter(|d| d.content_addressed) {
                if let Ok(request) = crate::correct::request(repo, &alias, &d.id)
                    && request.drift
                {
                    out.push(Pending::Correct {
                        alias: alias.clone(),
                        deliverable_id: d.id.clone(),
                        request,
                    });
                }
            }
            continue;
        }
        let Ok(request) = resolution_cmd::request(repo, &alias) else {
            continue;
        };
        if request.requirements_met {
            let profile = one
                .validated
                .as_ref()
                .map(|v| v.raw.profile.clone())
                .unwrap_or_default();
            out.push(Pending::Resolve {
                alias,
                request,
                profile,
            });
        }
    }
    for rev in repo.load_sas_revisions()? {
        if rev.state == SasRevisionState::Proposed {
            let request = sas::accept_request(repo, &rev.version)?;
            out.push(Pending::Accept {
                version: rev.version,
                request,
            });
        }
    }
    Ok(out)
}

/// The most recent `amendments/AM-*.yaml`, read line-wise for its `reason`
/// and `semantic_diff` entries.
fn read_latest_amendment(dir: &Utf8Path) -> Option<AmendmentSummary> {
    let amendments = dir.join("amendments");
    let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(&amendments)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
        .filter(|p| p.extension() == Some("yaml"))
        .collect();
    files.sort();
    let path = files.pop()?;
    let text = std::fs::read_to_string(&path).ok()?;
    let mut summary = AmendmentSummary {
        id: path.file_stem().unwrap_or_default().to_owned(),
        ..Default::default()
    };
    let unquote = |s: &str| s.trim().trim_matches('"').to_owned();
    let (mut element, mut before) = (None::<String>, None::<String>);
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(v) = t.strip_prefix("reason:") {
            summary.reason = unquote(v);
        } else if let Some(v) = t.strip_prefix("- element:") {
            element = Some(unquote(v));
        } else if let Some(v) = t.strip_prefix("before:") {
            before = Some(unquote(v));
        } else if let Some(v) = t.strip_prefix("after:")
            && let (Some(e), Some(b)) = (element.take(), before.take())
        {
            summary.changes.push((e, b, unquote(v)));
        }
    }
    Some(summary)
}

/// `war sign` with no target: the list. Needs no terminal; it writes nothing.
pub fn list(repo: &Repository) -> Result<Vec<Pending>, RepoError> {
    pending(repo)
}

/// Render one pending act as a line for the list.
#[must_use]
pub fn line(p: &Pending) -> String {
    match p {
        Pending::Authorize {
            alias,
            request,
            amendment,
            revision,
        } => format!(
            "{alias}  rev {revision}  authorize  {}{}",
            request.title,
            amendment
                .as_ref()
                .map(|a| format!("  [{}]", a.id))
                .unwrap_or_default()
        ),
        Pending::Resolve { alias, request, .. } => format!(
            "{alias}  resolve  would resolve {}  {}",
            if request.would_resolve_satisfied == Some(true) {
                "SATISFIED"
            } else {
                "not satisfied"
            },
            request.title
        ),
        Pending::Accept { version, request } => format!(
            "SAS {version}  accept  {} requirement(s){}",
            request.requirement_count,
            if request.adr_required {
                "  (architecture-changing: --adr required)"
            } else {
                ""
            }
        ),
        Pending::Correct {
            alias,
            deliverable_id,
            request,
        } => format!(
            "{alias}/{deliverable_id}  correct  {} drifted (correction {})  {}",
            request.target_ref, request.next_sequence, request.title
        ),
    }
}

/// The screen the signer sees. Facts from the record; nothing recommended.
fn screen(p: &Pending, actor: &str, role: &str) -> String {
    let mut s = String::new();
    match p {
        Pending::Authorize {
            alias,
            request,
            amendment,
            revision,
        } => {
            s.push_str(&format!(
                "┌ {alias} · authorize revision {revision} · {}\n│ {}\n",
                request.assurance_level, request.title
            ));
            if let Some(a) = amendment {
                s.push_str(&format!(
                    "│ amendment {}: {}\n",
                    a.id,
                    first_sentence(&a.reason)
                ));
                for (e, b, af) in &a.changes {
                    s.push_str(&format!("│   {e}\n│     before: {b}\n│     after:  {af}\n"));
                }
            }
            s.push_str(&format!(
                "│ {} obligation(s)  ·  {} residual risk(s)\n",
                request.obligations.len(),
                request.residual_risks.len()
            ));
            for r in &request.residual_risks {
                s.push_str(&format!(
                    "│   {}  {}\n│       if false: {}\n",
                    r.assumption_id, r.statement, r.consequence_if_false
                ));
            }
            s.push_str(&format!(
                "│ contract sha256:{}  ·  covers {}\n",
                &request.contract_digest[..16],
                request.contract_coverage.join(", ")
            ));
        }
        Pending::Resolve { alias, request, .. } => {
            s.push_str(&format!(
                "┌ {alias} · resolve · revision {}\n│ {}\n",
                request.contract_revision, request.title
            ));
            s.push_str(&format!(
                "│ thirteen requirements: {}  ·  §38.6 would resolve satisfied: {}\n",
                if request.requirements_met {
                    "all met"
                } else {
                    "UNMET"
                },
                request
                    .would_resolve_satisfied
                    .map_or("n/a".to_owned(), |b| b.to_string())
            ));
            s.push_str(&format!(
                "│ established {}  ·  unestablished {}\n",
                request.established.len(),
                request.unestablished.len()
            ));
            for u in &request.unestablished {
                s.push_str(&format!("│   unestablished: {u}\n"));
            }
            s.push_str(&format!(
                "│ permitted outcomes: {}\n│ contract sha256:{}\n",
                request.permitted_outcomes.join(", "),
                &request.contract_digest[..16]
            ));
        }
        Pending::Accept { version, request } => {
            s.push_str(&format!(
                "┌ SAS {version} · accept · sha256:{}\n│ predecessor {}  ·  {} requirement(s)  ·  architecture-changing: {}\n",
                &request.sha256[..16],
                request.predecessor.as_deref().unwrap_or("none"),
                request.requirement_count,
                request.architecture_changing
            ));
            s.push_str(&format!(
                "│ §106 diff: +{} −{} ~{}\n",
                request.diff.added.len(),
                request.diff.removed.len(),
                request.diff.retitled.len()
            ));
        }
        Pending::Correct {
            alias,
            deliverable_id,
            request,
        } => {
            s.push_str(&format!(
                "┌ {alias} · correct {deliverable_id} · correction {} of a RESOLVED Warrant\n│ {}\n│ {}\n",
                request.next_sequence, request.title, request.target_ref
            ));
            s.push_str(&format!(
                "│ recorded   {}\n│ supersedes {}{}\n│ now        {}\n",
                request.recorded_digest,
                request.chain_head,
                if request.prior_corrections > 0 {
                    format!(
                        "  (after {} prior correction(s))",
                        request.prior_corrections
                    )
                } else {
                    String::new()
                },
                request.current_digest
            ));
            s.push_str(
                "│ the pin in deliverables.toml is not edited; the superseded digest stays on record\n",
            );
        }
    }
    s.push_str(&format!("└ Sign as {actor} ({role})? [y/N] "));
    s
}

fn first_sentence(s: &str) -> String {
    let end = s.find(". ").map_or(s.len(), |i| i + 1);
    s[..end].to_owned()
}

/// The templated meaning. Says what it is.
fn provenance() -> String {
    format!(
        "Presented on a terminal by `{}` and confirmed by the signer; the decision \
         is the signer's and the template is the tool's.",
        tool_version()
    )
}

/// Draft the response for one pending act. Pure: no I/O, so it is testable.
pub fn draft(p: &Pending, actor: &str, opts: &Options, now: &str) -> Result<Drafted, String> {
    let extra = opts
        .meaning
        .as_deref()
        .map(|m| format!(" {m}"))
        .unwrap_or_default();
    match p {
        Pending::Authorize { alias, request, .. } => {
            let judgment = request
                .residual_risks
                .iter()
                .enumerate()
                .map(|(i, r)| Judgment {
                    id: format!("J-{:03}", i + 1),
                    kind: "residual_risk_acceptance".to_owned(),
                    statement: r.statement.clone(),
                    actor: actor.to_owned(),
                    acting_role: "risk_acceptor".to_owned(),
                    meaning: format!(
                        "Accepting residual risk {} for {alias}: if the assumption is false, \
                         the consequence is: {} {}",
                        r.assumption_id,
                        r.consequence_if_false,
                        provenance()
                    ),
                    basis_refs: vec![format!("assumption://{}", r.assumption_id)],
                    authority: JudgmentAuthority::Authorized,
                    limitations: vec![],
                })
                .collect::<Vec<_>>();
            let response = AuthorizationResponse {
                schema: authorize::RESPONSE_SCHEMA.to_owned(),
                warrant: alias.clone(),
                contract_digest: request.contract_digest.clone(),
                authorizer: actor.to_owned(),
                acting_role: "authorizer".to_owned(),
                meaning: format!(
                    "Authorizing {alias} ({}) at this contract digest means the signer accepts \
                     its intent, scope and {} obligation(s) as the work to be done and judged, \
                     and accepts each of the {} residual risk(s) recorded below with its stated \
                     consequence. It does not resolve the Warrant; resolution still requires \
                     the §56.1 evidence.{extra} {}",
                    request.title,
                    request.obligations.len(),
                    judgment.len(),
                    provenance()
                ),
                effective_time: now.to_owned(),
                policy_basis: None,
                independence: opts.independence,
                judgment,
                signed_via: Some(opts.channel().to_owned()),
            };
            Ok(Drafted::Authorize(response))
        }
        Pending::Resolve {
            alias,
            request,
            profile,
        } => {
            let outcome = match (request.would_resolve_satisfied, opts.outcome) {
                (_, Some(o)) => o,
                (Some(true), None) => CommonOutcome::Satisfied,
                _ => {
                    return Err(format!(
                        "{alias}: §38.6 does not permit `satisfied` ({} obligation(s) \
                         unestablished); pass --outcome not_satisfied|cancelled|blocked",
                        request.unestablished.len()
                    ));
                }
            };
            if !request.permitted_outcomes.contains(&outcome.to_string()) {
                return Err(format!(
                    "{alias}: outcome {outcome} is not permitted here; permitted: {}",
                    request.permitted_outcomes.join(", ")
                ));
            }
            let profile_outcome = match (profile.as_str(), outcome) {
                ("delivery", CommonOutcome::Satisfied) => "delivered".to_owned(),
                _ => outcome.to_string(),
            };
            Ok(Drafted::Resolve(ResolutionResponse {
                schema: resolution_cmd::RESPONSE_SCHEMA.to_owned(),
                warrant: alias.clone(),
                contract_digest: request.contract_digest.clone(),
                resolved_by: actor.to_owned(),
                acting_role: "resolver".to_owned(),
                common_outcome: outcome,
                profile_outcome,
                meaning: format!(
                    "Resolving {alias} ({}) {outcome} at revision {} means the signer accepts \
                     the §56.1 record as it stands: {} obligation(s) established, {} not, every \
                     required gate with an admissible result, and the assurance case as \
                     snapshotted.{extra} {}",
                    request.title,
                    request.contract_revision,
                    request.established.len(),
                    request.unestablished.len(),
                    provenance()
                ),
                effective_time: now.to_owned(),
            }))
        }
        Pending::Accept { version, request } => {
            if request.adr_required && opts.adr_ref.is_none() {
                return Err(format!(
                    "SAS {version} is architecture-changing; §101.3 requires --adr <ref>"
                ));
            }
            Ok(Drafted::Accept(AcceptResponse {
                schema: sas::ACCEPT_RESPONSE_SCHEMA.to_owned(),
                version: version.clone(),
                sha256: request.sha256.clone(),
                accepted_by: actor.to_owned(),
                acting_role: "authorizer".to_owned(),
                meaning: format!(
                    "Accepting SAS revision {version} at this digest means the signer adopts \
                     it as the governing specification: {} requirement(s), §106 diff against \
                     {} of +{} −{} ~{}. Every later Warrant traces to this revision until it \
                     is superseded.{extra} {}",
                    request.requirement_count,
                    request.predecessor.as_deref().unwrap_or("no predecessor"),
                    request.diff.added.len(),
                    request.diff.removed.len(),
                    request.diff.retitled.len(),
                    provenance()
                ),
                effective_time: now.to_owned(),
                adr_ref: opts.adr_ref.clone(),
            }))
        }
        Pending::Correct {
            alias,
            deliverable_id,
            request,
        } => {
            // Nothing is guessed: the kind and the reason are the human's
            // whole contribution to a correction, and both are required.
            let Some(kind) = opts.kind else {
                return Err(format!(
                    "{alias}/{deliverable_id}: a correction needs --kind behaviour-change|added-refusal"
                ));
            };
            let Some(reason) = opts.meaning.as_deref().filter(|m| !m.trim().is_empty()) else {
                return Err(format!(
                    "{alias}/{deliverable_id}: a correction needs a reason; pass --meaning \"…\""
                ));
            };
            Ok(Drafted::Correct(crate::correct::CorrectionResponse {
                schema: crate::correct::RESPONSE_SCHEMA.to_owned(),
                warrant: alias.clone(),
                deliverable_id: deliverable_id.clone(),
                superseded_digest: request.chain_head.clone(),
                new_digest: request.current_digest.clone(),
                reason: format!("{reason} {}", provenance()),
                kind,
                corrected_by: actor.to_owned(),
                acting_role: "authorizer".to_owned(),
                effective_time: now.to_owned(),
                signed_via: Some(opts.channel().to_owned()),
            }))
        }
    }
}

/// A drafted response, typed by seam.
#[derive(Debug)]
pub enum Drafted {
    Authorize(AuthorizationResponse),
    Resolve(ResolutionResponse),
    Accept(AcceptResponse),
    Correct(crate::correct::CorrectionResponse),
}

impl Drafted {
    fn render(&self) -> Result<String, RepoError> {
        let r = match self {
            Self::Authorize(r) => toml::to_string_pretty(r),
            Self::Resolve(r) => toml::to_string_pretty(r),
            Self::Accept(r) => toml::to_string_pretty(r),
            Self::Correct(r) => toml::to_string_pretty(r),
        };
        r.map_err(|e| RepoError::Message(format!("could not render the response: {e}")))
    }

    fn file_stem(&self) -> String {
        match self {
            Self::Authorize(r) => r.warrant.clone(),
            Self::Resolve(r) => r.warrant.clone(),
            Self::Accept(r) => format!("SAS-{}", r.version),
            Self::Correct(r) => format!("{}.{}.correction", r.warrant, r.deliverable_id),
        }
    }

    /// The digest this response binds to — what distinguishes "the same act
    /// again" from "an earlier revision's record".
    fn digest(&self) -> &str {
        match self {
            Self::Authorize(r) => &r.contract_digest,
            Self::Resolve(r) => &r.contract_digest,
            Self::Accept(r) => &r.sha256,
            Self::Correct(r) => &r.new_digest,
        }
    }
}

/// Whether both ends of the conversation are a terminal.
#[must_use]
pub fn at_a_terminal() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// Who may sign, from the register — refusing to guess between two.
fn choose_actor(eligible: &[String], opts: &Options) -> Result<String, String> {
    if let Some(a) = &opts.actor {
        if eligible.iter().any(|e| e == a) {
            return Ok(a.clone());
        }
        return Err(format!(
            "{a} is not eligible to sign this; the register permits: {}",
            if eligible.is_empty() {
                "nobody".to_owned()
            } else {
                eligible.join(", ")
            }
        ));
    }
    match eligible {
        [] => Err(
            "nobody may sign this: docs/authority/roles.toml grants the role to no \
                   eligible human, or every holder proposed it (§27.2)"
                .to_owned(),
        ),
        [one] => Ok(one.clone()),
        many => Err(format!(
            "more than one eligible signer ({}); pass --as <actor>",
            many.join(", ")
        )),
    }
}

fn eligible(p: &Pending) -> &[String] {
    match p {
        Pending::Authorize { request, .. } => &request.eligible_authorizers,
        Pending::Resolve { request, .. } => &request.eligible_resolvers,
        Pending::Accept { request, .. } => &request.eligible_acceptors,
        Pending::Correct { request, .. } => &request.eligible_correctors,
    }
}

fn role(p: &Pending) -> &'static str {
    match p {
        Pending::Authorize { .. } | Pending::Accept { .. } | Pending::Correct { .. } => {
            "authorizer"
        }
        Pending::Resolve { .. } => "resolver",
    }
}

/// Which pending act a target names. A Warrant alias may have an authorization
/// AND a resolution pending in sequence; the first is what is signed now. A
/// correction is named `<alias>/<deliverable-id>`.
fn select<'a>(all: &'a [Pending], target: &str) -> Option<&'a Pending> {
    all.iter().find(|p| match p {
        Pending::Authorize { alias, .. } | Pending::Resolve { alias, .. } => alias == target,
        Pending::Accept { version, .. } => version == target || format!("SAS-{version}") == target,
        Pending::Correct {
            alias,
            deliverable_id,
            ..
        } => format!("{alias}/{deliverable_id}") == target,
    })
}

/// Ask on the terminal. `y`/`yes`, case-insensitive; anything else is no.
fn confirm(prompt: &str) -> Result<bool, RepoError> {
    let mut out = std::io::stdout();
    out.write_all(prompt.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|source| RepoError::Io {
            context: "could not write the prompt".to_owned(),
            source,
        })?;
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|source| RepoError::Io {
            context: "could not read the answer".to_owned(),
            source,
        })?;
    let a = line.trim().to_ascii_lowercase();
    Ok(a == "y" || a == "yes")
}

fn write_response(repo: &Repository, drafted: &Drafted) -> Result<Utf8PathBuf, RepoError> {
    let dir = repo.root.join("docs/authority/responses");
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    // Written under a DRAFT name. Only a `y` renames it to `.response.toml`,
    // so a crash, a Ctrl-C, or an `$EDITOR` that never returned leaves a file
    // that says "draft" in its name rather than one that reads as signed.
    let path = dir.join(format!("{}.draft.toml", drafted.file_stem()));
    std::fs::write(&path, drafted.render()?).map_err(|source| RepoError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    Ok(path)
}

/// The name a confirmed draft is renamed to. Same stem, different suffix, so
/// the two can never be confused and the draft never needs a cleanup pass to
/// be harmless.
/// What to do with a response already at the final path.
///
/// Two cases, and they must not be confused. A file carrying the SAME digest
/// as the act being signed is an unsent response — hand-written, or a prior
/// `war sign` that was refused — and overwriting it would destroy a record of
/// intent; it is refused, and the operator decides. A file carrying a
/// DIFFERENT digest is the signed response of an earlier revision, which is
/// history: it is renamed to carry its digest, never overwritten. The
/// responses directory is committed precisely so that signed decisions
/// travel; a rename that lost one would undo that.
fn retire_prior(final_path: &Utf8Path, current_digest: &str) -> Result<(), String> {
    if !final_path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(final_path)
        .map_err(|e| format!("{final_path} exists and could not be read: {e}"))?;
    // Parsed as TOML, not scanned by line: a formatting change in the
    // serializer must not turn "same digest" into "no digest found", because
    // that branch ARCHIVES, and archiving an unsent same-digest response is the
    // exact loss this function exists to prevent. Unparseable → refuse.
    let value: toml::Value = toml::from_str(&text)
        .map_err(|e| format!("{final_path} exists and is not valid TOML ({e}); not touched"))?;
    let prior = ["contract_digest", "sha256"]
        .iter()
        .find_map(|k| value.get(k).and_then(toml::Value::as_str))
        .ok_or_else(|| {
            format!(
                "{final_path} exists but carries no contract_digest or sha256; not touched — \
                 move it aside by hand"
            )
        })?;
    if prior == current_digest {
        return Err(format!(
            "{final_path} already holds a response for this exact digest. Ingest it \
             (`war authorize/resolve/sas accept … --response`) or remove it; it is not \
             overwritten"
        ));
    }
    let tag = if prior.len() >= 8 {
        &prior[..8]
    } else {
        "unknown"
    };
    let name = final_path
        .file_name()
        .unwrap_or_default()
        .trim_end_matches(".response.toml");
    let archived = final_path.with_file_name(format!("{name}.{tag}.response.toml"));
    if archived.exists() {
        return Err(format!(
            "{final_path} holds an earlier revision's response and {archived} already \
             exists; nothing is overwritten — move one aside by hand"
        ));
    }
    std::fs::rename(final_path, &archived)
        .map_err(|e| format!("could not retire {final_path} to {archived}: {e}"))?;
    // A signature travels with the file it signs.
    let sig = sig_path(final_path);
    if sig.is_file() {
        let archived_sig = sig_path(&archived);
        std::fs::rename(&sig, &archived_sig)
            .map_err(|e| format!("could not retire {sig} to {archived_sig}: {e}"))?;
    }
    Ok(())
}

/// The public key line for a principal in `docs/authority/allowed_signers`,
/// as `"<keytype> <base64>"`. OpenSSH's format: `principals [options] keytype
/// base64 [comment]`, one per line, `#` comments. Exactly one line may name
/// the principal: `ssh-keygen -Y verify` would accept any of several, so two
/// lines is refused here as a misconfiguration rather than resolved by order.
pub(crate) fn pubkey_for_principal(
    allowed_signers: &str,
    principal: &str,
) -> Result<String, String> {
    let matches: Vec<String> = allowed_signers
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let mut fields = l.split_whitespace();
            let principals = fields.next()?;
            if !principals.split(',').any(|p| p == principal) {
                return None;
            }
            // Skip `key=value` options until the key type. A key type is a bare
            // token: an option VALUE that happens to start with `ssh-` still
            // carries its `=` and is not one.
            let rest: Vec<&str> = fields.collect();
            let start = rest.iter().position(|f| {
                !f.contains('=')
                    && (f.starts_with("ssh-") || f.starts_with("sk-") || f.starts_with("ecdsa-"))
            })?;
            let keytype = rest.get(start)?;
            let b64 = rest.get(start + 1)?;
            Some(format!("{keytype} {b64}"))
        })
        .collect();
    match matches.as_slice() {
        [] => Err(format!(
            "no key for principal {principal:?}; add one by hand"
        )),
        [one] => Ok(one.clone()),
        // `ssh-keygen -Y verify` would accept ANY of them. Two lines for one
        // principal means one of them is not the human's, or the file was
        // edited twice; either way it is a misconfiguration to refuse, not a
        // choice to make silently by line order.
        many => Err(format!(
            "{} lines name principal {principal:?}; exactly one is required — a duplicate \
             principal is a misconfiguration, not a fallback",
            many.len()
        )),
    }
}

/// Sign `file` as `principal` and verify the result. Returns the `.sig` path.
///
/// The private key is never touched here: `ssh-keygen -Y sign -f <pubkey>`
/// asks the agent for it, and an agent loaded with `ssh-add -c` asks the human
/// through a confirmation dialog no shell can answer. `war` cannot check that
/// `-c` was used — that is the one thing the operator must get right, and the
/// docs say so. Verification against the allowed_signers file happens before
/// anything is renamed, so a signature that does not verify writes nothing.
fn ssh_sign_file(
    allowed_signers: &Utf8Path,
    principal: &str,
    file: &Utf8Path,
) -> Result<Utf8PathBuf, String> {
    let text = std::fs::read_to_string(allowed_signers).map_err(|e| {
        format!("{allowed_signers}: {e}. --ssh-sign needs that file, written by a human")
    })?;
    let pubkey = pubkey_for_principal(&text, principal)
        .map_err(|why| format!("{allowed_signers}: {why}"))?;
    // Unique per process so two signers of one file cannot delete each other's
    // key mid-sign, and named so a stray one (crash between write and cleanup)
    // reads as a temp file, not an authority artifact.
    let pub_path = Utf8PathBuf::from(format!("{file}.{}.tmp.pub", std::process::id()));
    std::fs::write(&pub_path, format!("{pubkey} {principal}\n"))
        .map_err(|e| format!("could not write {pub_path}: {e}"))?;
    let sign = std::process::Command::new("ssh-keygen")
        .args(["-Y", "sign", "-f"])
        .arg(&pub_path)
        .args(["-n", SSH_NAMESPACE])
        .arg(file)
        .output();
    let _ = std::fs::remove_file(&pub_path);
    let out = sign.map_err(|e| format!("could not run ssh-keygen: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen -Y sign refused ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let sig = sig_path(file);
    if !sig.is_file() {
        return Err(format!(
            "ssh-keygen reported success but {sig} does not exist"
        ));
    }
    if let Err(why) = ssh_verify_file(allowed_signers, principal, file) {
        let _ = std::fs::remove_file(&sig);
        return Err(why);
    }
    Ok(sig)
}

/// `ssh-keygen -Y verify` of `file` against its `.sig` sidecar.
fn ssh_verify_file(
    allowed_signers: &Utf8Path,
    principal: &str,
    file: &Utf8Path,
) -> Result<(), String> {
    let sig = sig_path(file);
    let input = std::fs::File::open(file).map_err(|e| format!("could not open {file}: {e}"))?;
    let out = std::process::Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(allowed_signers)
        .args(["-I", principal, "-n", SSH_NAMESPACE, "-s"])
        .arg(&sig)
        .stdin(input)
        .output()
        .map_err(|e| format!("could not run ssh-keygen: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "signature on {file} does not verify as {principal} ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// What an ssh-signed act attests to: the record it wrote and the response
/// that carried the signature, with the response's own fields as predicate.
fn attest_after(
    repo: &Repository,
    p: &Pending,
    actor: &str,
    response: &Utf8Path,
) -> Result<Utf8PathBuf, String> {
    let (act, target, record): (&str, String, Utf8PathBuf) = match p {
        Pending::Authorize { alias, .. } => (
            "authorize",
            alias.clone(),
            repo.warrant_dir(alias)
                .map_err(|e| e.to_string())?
                .join("authorization.toml"),
        ),
        Pending::Resolve { alias, .. } => (
            "resolve",
            alias.clone(),
            repo.warrant_dir(alias)
                .map_err(|e| e.to_string())?
                .join("resolution.toml"),
        ),
        Pending::Correct {
            alias,
            deliverable_id,
            ..
        } => {
            let cdir = repo
                .warrant_dir(alias)
                .map_err(|e| e.to_string())?
                .join("corrections");
            // The newest correction file for this deliverable is the one just written.
            let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(&cdir)
                .map_err(|e| format!("{cdir}: {e}"))?
                .filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|f| {
                    f.file_name().is_some_and(|n| {
                        n.starts_with(&format!("{deliverable_id}-")) && n.ends_with(".toml")
                    })
                })
                .collect();
            files.sort();
            let newest = files
                .pop()
                .ok_or_else(|| format!("no correction file under {cdir}"))?;
            ("correct", alias.clone(), newest)
        }
        Pending::Accept { version, .. } => (
            "sas-accept",
            version.clone(),
            repo.root
                .join(&repo.config.paths.sas)
                .join("revisions")
                .join(format!("{version}.toml")),
        ),
    };
    let response_text =
        std::fs::read_to_string(response).map_err(|e| format!("{response}: {e}"))?;
    let response_toml: toml::Value =
        toml::from_str(&response_text).map_err(|e| format!("{response}: {e}"))?;
    let mut predicate = serde_json::to_value(&response_toml).map_err(|e| e.to_string())?;
    if let serde_json::Value::Object(map) = &mut predicate {
        map.insert(
            "actor".to_owned(),
            serde_json::Value::String(actor.to_owned()),
        );
        map.insert("act".to_owned(), serde_json::Value::String(act.to_owned()));
    }
    let mut extra = Vec::new();
    if let Some(d) = response_toml
        .get("contract_digest")
        .and_then(toml::Value::as_str)
        .map(|d| d.strip_prefix("sha256:").unwrap_or(d))
        .filter(|d| d.len() == 64)
    {
        extra.push((format!("contract:{target}"), d.to_owned()));
    }
    let a = crate::attest::Attestable {
        act,
        target: &target,
        files: vec![
            Utf8PathBuf::from(repo.relative(&record)),
            Utf8PathBuf::from(repo.relative(response)),
        ],
        extra_subjects: extra,
        predicate,
        actor,
    };
    let written = crate::attest::emit(repo, &a).map_err(|e| e.to_string())?;
    // A Warrant act is journalled; a SAS acceptance has no journal.
    if act != "sas-accept" {
        let dir = repo.warrant_dir(&target).map_err(|e| e.to_string())?;
        if let Some(uuid) = repo
            .load_warrant(&dir)
            .ok()
            .and_then(|l| l.validated.map(|v| v.uuid.to_string()))
        {
            crate::journal_cmd::record(
                &dir,
                &uuid,
                "attestation.recorded",
                &format!("human://{actor}"),
                &serde_json::json!({"act": act, "path": repo.relative(&written)}).to_string(),
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(written)
}

/// `<file>.sig`, what `ssh-keygen -Y sign` writes beside its input.
fn sig_path(file: &Utf8Path) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{file}.sig"))
}

/// The ssh principal the register names for an actor.
pub(crate) fn principal_of(repo: &Repository, actor: &str) -> Result<String, String> {
    let register = repo.load_authority_register().map_err(|e| e.to_string())?;
    register
        .assignments
        .iter()
        .find(|a| a.actor == actor)
        .and_then(|a| a.ssh_principal.clone())
        .ok_or_else(|| {
            format!(
                "{actor} has no `ssh_principal` in docs/authority/roles.toml; --ssh-sign needs \
                 one, and only a human writes that file"
            )
        })
}

pub(crate) fn allowed_signers_path(repo: &Repository) -> Utf8PathBuf {
    repo.root.join("docs/authority/allowed_signers")
}

fn signed_path(draft: &Utf8Path) -> Utf8PathBuf {
    let name = draft
        .file_name()
        .unwrap_or_default()
        .trim_end_matches(".draft.toml");
    draft.with_file_name(format!("{name}.response.toml"))
}

/// `--edit`: hand the drafted file to `$EDITOR` before the prompt.
fn edit(path: &Utf8Path) -> Result<(), RepoError> {
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .map_err(|_| RepoError::Message("--edit needs $VISUAL or $EDITOR".to_owned()))?;
    // `VISUAL="code --wait"` is a program AND its arguments; treating the whole
    // string as the program name finds nothing. Split on whitespace, no shell:
    // a shell would turn the variable into an injection point. The cost is an
    // editor PATH containing a space, which then fails loudly ("could not run
    // /opt/My") rather than running something else; that trade is deliberate.
    let mut words = editor.split_whitespace();
    let Some(program) = words.next() else {
        return Err(RepoError::Message(
            "--edit: $VISUAL/$EDITOR is set but empty".to_owned(),
        ));
    };
    let status = std::process::Command::new(program)
        .args(words)
        .arg(path)
        .status()
        .map_err(|source| RepoError::Io {
            context: format!("could not run {editor}"),
            source,
        })?;
    if !status.success() {
        return Err(RepoError::Message(format!(
            "{editor} exited {status}; not signing"
        )));
    }
    Ok(())
}

/// `war sign <target>` / `war sign --all`. The terminal check comes first,
/// before any record is read, so a refusal tells the operator exactly one
/// thing and reads nothing it does not need. `--show` is the one path that
/// reads without a terminal, because it writes nothing and asks nothing.
pub fn run(repo: &Repository, target: Option<&str>, opts: &Options) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if opts.verify {
        return verify_existing(repo, target, opts);
    }
    if !opts.show && !opts.ssh_sign && !at_a_terminal() {
        report.push(Diagnostic::error(
            "sign.no-tty",
            "war sign".to_owned(),
            "a signature needs a hand: stdin and stdout must both be a terminal. This is \
             refused from a pipe or an agent's shell by design (§27.2); run it yourself, \
             or use `war sign --list` to see what is waiting"
                .to_owned(),
        ));
        return Ok(report);
    }
    let all = pending(repo)?;
    let chosen: Vec<&Pending> = match (target, opts.all) {
        (Some(t), _) => match select(&all, t) {
            Some(p) => vec![p],
            None => {
                report.push(Diagnostic::error(
                    "sign.nothing-pending",
                    t.to_owned(),
                    format!(
                        "{t}: nothing awaits a signature — not a Warrant needing \
                             authorization or resolution, nor a proposed SAS revision"
                    ),
                ));
                return Ok(report);
            }
        },
        (None, true) => all.iter().collect(),
        (None, false) => {
            report.push(Diagnostic::error(
                "sign.no-target",
                "war sign".to_owned(),
                "name a Warrant alias or SAS version, or pass --all; `war sign --list` shows \
                 what is waiting"
                    .to_owned(),
            ));
            return Ok(report);
        }
    };
    if chosen.is_empty() {
        report.push(Diagnostic::pass(
            "sign.nothing-pending",
            "nothing awaits a signature",
        ));
        return Ok(report);
    }
    if opts.show {
        for p in chosen {
            let actor = choose_actor(eligible(p), opts).unwrap_or_else(|_| "<signer>".to_owned());
            println!("{}", screen(p, &actor, role(p)).trim_end_matches("[y/N] "));
            report.push(Diagnostic::pass(
                "sign.shown",
                format!("{} — shown, not signed; nothing written", line(p)),
            ));
        }
        return Ok(report);
    }

    for p in chosen {
        let actor = match choose_actor(eligible(p), opts) {
            Ok(a) => a,
            Err(why) => {
                report.push(Diagnostic::error("sign.who", line(p), why));
                continue;
            }
        };
        let now = crate::gate_cmd::receipt::now_rfc3339_public();
        let drafted = match draft(p, &actor, opts, &now) {
            Ok(d) => d,
            Err(why) => {
                report.push(Diagnostic::error("sign.not-draftable", line(p), why));
                continue;
            }
        };
        let draft_path = write_response(repo, &drafted)?;
        if opts.edit {
            edit(&draft_path)?;
        }
        let confirmed = if opts.ssh_sign {
            // No prompt: the confirmation is the agent's dialog. The screen is
            // still printed so the signer sees what the dialog is for.
            println!("{}", screen(p, &actor, role(p)).trim_end_matches("[y/N] "));
            println!(
                "└ Signing as {actor} ({}) with ssh — confirm in the agent's dialog",
                role(p)
            );
            let principal = match principal_of(repo, &actor) {
                Ok(pr) => pr,
                Err(why) => {
                    let _ = std::fs::remove_file(&draft_path);
                    report.push(Diagnostic::error("sign.ssh-principal", line(p), why));
                    continue;
                }
            };
            match ssh_sign_file(&allowed_signers_path(repo), &principal, &draft_path) {
                Ok(_) => true,
                Err(why) => {
                    let _ = std::fs::remove_file(&draft_path);
                    report.push(Diagnostic::error("sign.ssh-refused", line(p), why));
                    continue;
                }
            }
        } else {
            println!("{}", screen(p, &actor, role(p)).trim_end_matches("[y/N] "));
            confirm(&format!("└ Sign as {actor} ({})? [y/N] ", role(p)))?
        };
        if !confirmed {
            // A declined signature leaves nothing that later reads as one. The
            // draft's NAME already says it is not a response; a cleanup that
            // fails is reported rather than swallowed, so the operator knows a
            // stray draft exists.
            match std::fs::remove_file(&draft_path) {
                Ok(()) => report.push(Diagnostic::warn(
                    "sign.declined",
                    line(p),
                    "not signed; nothing written".to_owned(),
                )),
                Err(e) => report.push(Diagnostic::warn(
                    "sign.declined",
                    line(p),
                    format!("not signed; could not remove the draft {draft_path}: {e}"),
                )),
            }
            continue;
        }
        // The rename IS the signature's moment on disk. Before it, a draft;
        // after it, a response the same ingest as a hand-written one reads.
        let path = signed_path(&draft_path);
        if let Err(why) = retire_prior(&path, drafted.digest()) {
            let _ = std::fs::remove_file(&draft_path);
            report.push(Diagnostic::error("sign.response-exists", line(p), why));
            continue;
        }
        std::fs::rename(&draft_path, &path).map_err(|source| RepoError::Io {
            context: format!("could not move {draft_path} to {path}"),
            source,
        })?;
        // The signature covers the BYTES, so renaming the file it signs does
        // not invalidate it; the sidecar just follows.
        let draft_sig = sig_path(&draft_path);
        if draft_sig.is_file() {
            let sig = sig_path(&path);
            std::fs::rename(&draft_sig, &sig).map_err(|source| RepoError::Io {
                context: format!("could not move {draft_sig} to {sig}"),
                source,
            })?;
        }
        let ingested = match (p, &drafted) {
            (Pending::Authorize { alias, .. }, _) => authorize::ingest(repo, alias, &path)?,
            (Pending::Resolve { alias, .. }, _) => resolution_cmd::ingest(repo, alias, &path)?,
            (Pending::Accept { version, .. }, _) => sas::accept_ingest(repo, version, &path)?,
            (
                Pending::Correct {
                    alias,
                    deliverable_id,
                    ..
                },
                _,
            ) => crate::correct::ingest(repo, alias, deliverable_id, &path)?,
        };
        let accepted = ingested.is_ready();
        for d in ingested.diagnostics {
            report.push(d);
        }
        for n in ingested.notes {
            report.note(n);
        }
        if accepted && opts.ssh_sign {
            // OW-ADR-0015: an ssh-signed act is attested with the same key.
            // The act stands whether or not this succeeds; a refusal here is
            // a WARN naming why, never a reason to undo an ingested record.
            match attest_after(repo, p, &actor, &path) {
                Ok(written) => report.push(Diagnostic::pass(
                    "attest.emitted",
                    format!(
                        "{}: attestation written to {}",
                        line(p),
                        repo.relative(&written)
                    ),
                )),
                Err(why) => report.push(Diagnostic::warn(
                    "attest.not-emitted",
                    line(p),
                    format!("the act is recorded; its attestation was not: {why}"),
                )),
            }
        }
        if !accepted {
            // Ingest refused it. The response file is the evidence of what was
            // attempted; the refusal is in the report. It is moved to a
            // `.refused.toml` name so the final path is free for the next
            // attempt — left in place, `retire_prior` would refuse the retry
            // as "already holds a response for this exact digest".
            let refused = path.with_file_name(format!(
                "{}.refused.toml",
                path.file_name()
                    .unwrap_or_default()
                    .trim_end_matches(".response.toml")
            ));
            let moved = std::fs::rename(&path, &refused);
            if moved.is_ok()
                && sig_path(&path).is_file()
                && let Err(e) = std::fs::rename(sig_path(&path), sig_path(&refused))
            {
                report.push(Diagnostic::warn(
                    "sign.refused",
                    line(p),
                    format!("the .sig sidecar could not follow to {refused}: {e}"),
                ));
            }
            report.push(Diagnostic::warn(
                "sign.refused",
                line(p),
                match moved {
                    Ok(()) => format!("ingest refused the signature; kept as {refused}"),
                    Err(e) => format!(
                        "ingest refused the signature; {path} could not be moved aside ({e}) \
                         and will block the next attempt until removed"
                    ),
                },
            ));
        }
    }
    Ok(report)
}

/// `war sign <target> --verify`: check a recorded response's signature against
/// the allowed_signers file as it stands NOW. Writes nothing. A response with
/// no `.sig` is reported as unsigned, not as failing — `signed_via = "tty"`
/// responses have none by design.
fn verify_existing(
    repo: &Repository,
    target: Option<&str>,
    opts: &Options,
) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let Some(t) = target else {
        report.push(Diagnostic::error(
            "sign.no-target",
            "war sign --verify".to_owned(),
            "name a Warrant alias or `SAS-<version>`".to_owned(),
        ));
        return Ok(report);
    };
    // A SAS target is one the revision records know, not one that looks like a
    // version; the namespace prefix is the repository's, not a literal.
    let bare = t.strip_prefix("SAS-").unwrap_or(t);
    // Unreadable revision records must not fail a WARRANT verify; they only
    // decide whether a bare version string is a SAS target.
    let is_sas = t.starts_with("SAS-")
        || repo
            .load_sas_revisions()
            .map(|revs| revs.iter().any(|r| r.version == bare))
            .unwrap_or(false);
    let stem = if is_sas {
        format!("SAS-{bare}")
    } else {
        t.to_owned()
    };
    let path = repo
        .root
        .join("docs/authority/responses")
        .join(format!("{stem}.response.toml"));
    if !path.is_file() {
        report.push(Diagnostic::error(
            "sign.no-response",
            path.to_string(),
            format!("{t}: no recorded response to verify"),
        ));
        return Ok(report);
    }
    if !sig_path(&path).is_file() {
        report.push(Diagnostic::warn(
            "sign.unsigned",
            path.to_string(),
            format!("{t}: the response carries no .sig sidecar (signed on a terminal, or by hand)"),
        ));
        return Ok(report);
    }
    let text = std::fs::read_to_string(&path).map_err(|source| RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    let value: toml::Value =
        toml::from_str(&text).map_err(|e| RepoError::Message(format!("{path}: {e}")))?;
    // The signer is read from the SIGNED bytes — each response type names
    // exactly one of these — never from a flag. A `--as` here would let the
    // caller pick whichever principal makes the signature verify.
    let actor = ["authorizer", "resolved_by", "accepted_by", "corrected_by"]
        .iter()
        .find_map(|k| value.get(k).and_then(toml::Value::as_str))
        .map(str::to_owned);
    let Some(actor) = actor else {
        report.push(Diagnostic::error(
            "sign.who",
            path.to_string(),
            "the response names no signer (no authorizer, resolved_by or accepted_by); it \
             cannot be verified as anyone"
                .to_owned(),
        ));
        return Ok(report);
    };
    if let Some(claimed) = &opts.actor
        && claimed != &actor
    {
        report.push(Diagnostic::error(
            "sign.who",
            path.to_string(),
            format!(
                "--as {claimed:?} but the signed response names {actor:?}; refusing to verify \
                     against a different principal"
            ),
        ));
        return Ok(report);
    }
    let principal = match principal_of(repo, &actor) {
        Ok(pr) => pr,
        Err(why) => {
            report.push(Diagnostic::error(
                "sign.ssh-principal",
                path.to_string(),
                why,
            ));
            return Ok(report);
        }
    };
    match ssh_verify_file(&allowed_signers_path(repo), &principal, &path) {
        Ok(()) => report.push(Diagnostic::pass(
            "sign.verified",
            format!("{t}: signature verifies as {principal} ({actor}) under {SSH_NAMESPACE}"),
        )),
        Err(why) => report.push(Diagnostic::error(
            "sign.not-verified",
            path.to_string(),
            why,
        )),
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authorize::{RequestedObligation, RequestedResidualRisk};

    #[test]
    fn a_principal_is_found_by_name_past_options_and_among_a_list() {
        let f = "# comment\n\
                 alice namespaces=\"oh.war/response\" ssh-ed25519 AAAAalice alice@host\n\
                 bob,carol ssh-ed25519 AAAAbob\n\
                 dave sk-ssh-ed25519@openssh.com AAAAdave\n";
        assert_eq!(
            pubkey_for_principal(f, "alice").unwrap(),
            "ssh-ed25519 AAAAalice"
        );
        assert_eq!(
            pubkey_for_principal(f, "carol").unwrap(),
            "ssh-ed25519 AAAAbob"
        );
        assert_eq!(
            pubkey_for_principal(f, "dave").unwrap(),
            "sk-ssh-ed25519@openssh.com AAAAdave"
        );
        assert!(pubkey_for_principal(f, "erin").is_err());
        assert!(
            pubkey_for_principal(f, "ali").is_err(),
            "no prefix matching"
        );
        // Two lines for one principal: refused, not first-wins.
        let dup = "alice ssh-ed25519 AAAAreal\nalice ssh-ed25519 AAAAmallory\n";
        let err = pubkey_for_principal(dup, "alice").unwrap_err();
        assert!(err.contains("2 lines"), "{err}");
        // An option whose value starts with `ssh-` is not a key type.
        let opt = "alice cert-authority=ssh-ed25519 ssh-ed25519 AAAAalice\n";
        assert_eq!(
            pubkey_for_principal(opt, "alice").unwrap(),
            "ssh-ed25519 AAAAalice"
        );
        // No recognisable key type at all: nothing found.
        assert!(pubkey_for_principal("alice foo AAAA\n", "alice").is_err());
    }

    /// The whole pipeline against a throwaway key — sign, verify, and the
    /// three ways it must refuse: wrong principal, a byte changed after
    /// signing, and a key the allowed_signers file does not list. `ssh-keygen`
    /// signs with `-f <private key>` directly here, so no agent is needed; the
    /// agent (and its `-c` dialog) is the deployment, not the mechanism.
    #[test]
    fn ssh_sign_then_verify_and_every_refusal() {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-sign-ssh-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let key = dir.join("id");
        let generated = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "test", "-f"])
            .arg(&key)
            .status()
            .expect("ssh-keygen present");
        assert!(generated.success());
        let pubkey = std::fs::read_to_string(dir.join("id.pub")).unwrap();
        let allowed = dir.join("allowed_signers");
        std::fs::write(
            &allowed,
            format!("tester namespaces=\"{SSH_NAMESPACE}\" {pubkey}"),
        )
        .unwrap();

        let file = dir.join("OW-WAR-0001.draft.toml");
        std::fs::write(&file, "schema = \"x\"\ncontract_digest = \"ab\"\n").unwrap();
        // Sign with the private key directly (the pub file the helper writes
        // would route through an agent; here we call ssh-keygen ourselves to
        // produce the sidecar, then exercise OUR verify path on it).
        let s = std::process::Command::new("ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(&key)
            .args(["-n", SSH_NAMESPACE])
            .arg(&file)
            .status()
            .unwrap();
        assert!(s.success());
        assert!(sig_path(&file).is_file());
        ssh_verify_file(&allowed, "tester", &file).expect("verifies");

        // Wrong principal: refused.
        assert!(ssh_verify_file(&allowed, "someone", &file).is_err());
        // Renaming does not break it — the signature is over bytes.
        let renamed = dir.join("OW-WAR-0001.response.toml");
        std::fs::rename(&file, &renamed).unwrap();
        std::fs::rename(sig_path(&file), sig_path(&renamed)).unwrap();
        ssh_verify_file(&allowed, "tester", &renamed).expect("still verifies");
        // One byte changed after signing: refused.
        std::fs::write(&renamed, "schema = \"x\"\ncontract_digest = \"ac\"\n").unwrap();
        assert!(ssh_verify_file(&allowed, "tester", &renamed).is_err());
        // A key allowed_signers does not list: refused at sign time by name.
        assert!(
            ssh_sign_file(&allowed, "nobody", &renamed)
                .unwrap_err()
                .contains("no key for principal")
        );
        // No temp .pub left behind by any of the above.
        let leftovers = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".tmp.pub"))
            .count();
        assert_eq!(leftovers, 0);
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn authorize_pending(risks: usize) -> Pending {
        Pending::Authorize {
            alias: "OW-WAR-0001".to_owned(),
            revision: 1,
            amendment: None,
            request: AuthorizationRequest {
                schema: authorize::REQUEST_SCHEMA.to_owned(),
                warrant: "OW-WAR-0001".to_owned(),
                title: "t".to_owned(),
                assurance_level: "basic".to_owned(),
                contract_digest: "ab".repeat(32),
                contract_coverage: vec!["intent".to_owned()],
                proposer: "claude".to_owned(),
                obligations: vec![RequestedObligation {
                    id: "OBL-001".to_owned(),
                    statement: "it works".to_owned(),
                }],
                residual_risks: (0..risks)
                    .map(|i| RequestedResidualRisk {
                        assumption_id: format!("A-{:03}", i + 1),
                        statement: "assumed".to_owned(),
                        consequence_if_false: "bad".to_owned(),
                        judgment_ref: String::new(),
                    })
                    .collect(),
                eligible_authorizers: vec!["Brian Lam".to_owned()],
            },
        }
    }

    /// `cargo test` has no terminal, which is the case this exists to refuse.
    /// The refusal must come BEFORE any record is read: `run` on the real
    /// repository, from a pipe, returns exactly one diagnostic and touches no
    /// file. (A §92 plant is the proper form; `conformance/plant.sh` is a
    /// pinned deliverable of OW-WAR-0063 and cannot move until OW-WAR-0064.)
    #[test]
    fn without_a_terminal_the_refusal_is_the_only_diagnostic_and_nothing_is_written() {
        assert!(!at_a_terminal(), "tests run without a TTY");
        let root = camino::Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize_utf8()
            .expect("repository root");
        let repo = Repository::open(root.clone()).expect("opens");
        let responses = root.join("docs/authority/responses");
        let before = std::fs::read_dir(&responses)
            .map(|d| d.count())
            .unwrap_or(0);
        let report = run(&repo, Some("OW-WAR-0001"), &Options::default()).expect("runs");
        assert_eq!(report.diagnostics.len(), 1, "{:?}", report.diagnostics);
        assert_eq!(report.diagnostics[0].rule, "sign.no-tty");
        assert!(!report.is_ready());
        let after = std::fs::read_dir(&responses)
            .map(|d| d.count())
            .unwrap_or(0);
        assert_eq!(before, after, "a refused signature writes nothing");
    }

    #[test]
    fn one_judgment_per_residual_risk_each_citing_its_assumption() {
        let p = authorize_pending(3);
        let opts = Options::default();
        let Drafted::Authorize(r) =
            draft(&p, "Brian Lam", &opts, "2026-09-11T00:00:00Z").expect("drafts")
        else {
            panic!("wrong seam");
        };
        assert_eq!(r.judgment.len(), 3);
        assert_eq!(r.judgment[2].id, "J-003");
        assert_eq!(r.judgment[2].basis_refs, vec!["assumption://A-003"]);
        assert_eq!(r.judgment[2].acting_role, "risk_acceptor");
        assert_eq!(r.judgment[2].authority, JudgmentAuthority::Authorized);
        assert!(
            r.judgment[2].meaning.contains("bad"),
            "{}",
            r.judgment[2].meaning
        );
        assert_eq!(r.signed_via.as_deref(), Some("tty"));
        assert_eq!(r.contract_digest, "ab".repeat(32));
        assert!(r.meaning.contains("war sign"), "provenance names the tool");
        for j in &r.judgment {
            j.validate().expect("a drafted judgment must validate");
        }
    }

    #[test]
    fn the_signers_own_words_are_appended_not_replaced() {
        let p = authorize_pending(0);
        let opts = Options {
            meaning: Some("I read the residual risks.".to_owned()),
            ..Options::default()
        };
        let Drafted::Authorize(r) = draft(&p, "Brian Lam", &opts, "t").expect("drafts") else {
            panic!();
        };
        assert!(r.meaning.contains("I read the residual risks."));
        assert!(r.meaning.contains("Authorizing OW-WAR-0001"));
    }

    #[test]
    fn a_resolution_that_cannot_be_satisfied_needs_an_outcome_and_refuses_to_guess() {
        let p = Pending::Resolve {
            alias: "OW-WAR-0002".to_owned(),
            profile: "delivery".to_owned(),
            request: ResolutionRequest {
                schema: resolution_cmd::REQUEST_SCHEMA.to_owned(),
                warrant: "OW-WAR-0002".to_owned(),
                title: "t".to_owned(),
                contract_digest: "cd".repeat(32),
                contract_revision: 1,
                requirements_met: true,
                unmet: vec![],
                would_resolve_satisfied: Some(false),
                established: vec![],
                unestablished: vec!["OBL-001".to_owned()],
                permitted_outcomes: vec![
                    "not_satisfied".to_owned(),
                    "cancelled".to_owned(),
                    "blocked".to_owned(),
                ],
                gate_run_refs: vec![],
                judgment_refs: vec![],
                residual_risk_refs: vec![],
                eligible_resolvers: vec!["Brian Lam".to_owned()],
            },
        };
        let err = draft(&p, "Brian Lam", &Options::default(), "t").expect_err("must not guess");
        assert!(err.contains("--outcome"), "{err}");
        let opts = Options {
            outcome: Some(CommonOutcome::Satisfied),
            ..Options::default()
        };
        let err = draft(&p, "Brian Lam", &opts, "t").expect_err("satisfied is not permitted");
        assert!(err.contains("not permitted"), "{err}");
        let opts = Options {
            outcome: Some(CommonOutcome::NotSatisfied),
            ..Options::default()
        };
        let Drafted::Resolve(r) = draft(&p, "Brian Lam", &opts, "t").expect("drafts") else {
            panic!();
        };
        assert_eq!(r.profile_outcome, "not_satisfied");
    }

    #[test]
    fn a_draft_and_its_signed_name_share_a_stem_and_never_a_suffix() {
        let d = camino::Utf8PathBuf::from("/r/docs/authority/responses/OW-WAR-0064.draft.toml");
        let s = signed_path(&d);
        assert_eq!(
            s.as_str(),
            "/r/docs/authority/responses/OW-WAR-0064.response.toml"
        );
        let sas = camino::Utf8PathBuf::from("/r/x/SAS-0.1.0-draft.4.draft.toml");
        assert_eq!(
            signed_path(&sas).as_str(),
            "/r/x/SAS-0.1.0-draft.4.response.toml"
        );
    }

    #[test]
    fn an_existing_response_is_refused_for_the_same_digest_and_retired_for_a_prior_one() {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-sign-prior-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let final_path = dir.join("OW-WAR-0040.response.toml");
        // Nothing there: fine.
        assert!(retire_prior(&final_path, "aa").is_ok());
        // Same digest: refused, file untouched — against a REAL response
        // shape, with judgments after the scalar, so a serializer that moves
        // things around still parses.
        let real = AuthorizationResponse {
            schema: authorize::RESPONSE_SCHEMA.to_owned(),
            warrant: "OW-WAR-0040".to_owned(),
            contract_digest: "aabb".to_owned(),
            authorizer: "Brian Lam".to_owned(),
            acting_role: "authorizer".to_owned(),
            meaning: "m".to_owned(),
            effective_time: "t".to_owned(),
            policy_basis: None,
            independence: Independence::SeparateRole,
            judgment: vec![Judgment {
                id: "J-001".to_owned(),
                kind: "residual_risk_acceptance".to_owned(),
                statement: "s".to_owned(),
                actor: "Brian Lam".to_owned(),
                acting_role: "risk_acceptor".to_owned(),
                meaning: "m".to_owned(),
                basis_refs: vec!["assumption://A-001".to_owned()],
                authority: JudgmentAuthority::Authorized,
                limitations: vec![],
            }],
            signed_via: None,
        };
        std::fs::write(&final_path, toml::to_string_pretty(&real).unwrap()).unwrap();
        let err = retire_prior(&final_path, "aabb").expect_err("must refuse");
        assert!(err.contains("not overwritten"), "{err}");
        assert!(final_path.is_file());
        // Not TOML, or TOML with no digest: refused, never archived.
        std::fs::write(&final_path, "not = [toml\n").unwrap();
        assert!(retire_prior(&final_path, "zz").is_err());
        std::fs::write(&final_path, "schema = \"x\"\n").unwrap();
        assert!(retire_prior(&final_path, "zz").is_err());
        assert!(final_path.is_file());
        // Different digest: retired under its own digest, path freed.
        std::fs::write(
            &final_path,
            "schema = \"x\"\ncontract_digest = \"d9f4e90b4a789dec\"\n",
        )
        .unwrap();
        retire_prior(&final_path, "cd42487d").expect("retires");
        assert!(!final_path.exists());
        assert!(dir.join("OW-WAR-0040.d9f4e90b.response.toml").is_file());
        // A second retirement onto the same archive name is refused, not clobbered.
        std::fs::write(
            &final_path,
            "schema = \"x\"\ncontract_digest = \"d9f4e90b4a789dec\"\n",
        )
        .unwrap();
        assert!(retire_prior(&final_path, "cd42487d").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn two_eligible_signers_is_a_refusal_not_a_guess() {
        let two = vec!["A".to_owned(), "B".to_owned()];
        assert!(choose_actor(&two, &Options::default()).is_err());
        assert_eq!(
            choose_actor(
                &two,
                &Options {
                    actor: Some("B".to_owned()),
                    ..Options::default()
                }
            )
            .unwrap(),
            "B"
        );
        assert!(choose_actor(&[], &Options::default()).is_err());
        assert!(
            choose_actor(
                &two,
                &Options {
                    actor: Some("C".to_owned()),
                    ..Options::default()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn an_amendment_summary_reads_reason_and_each_change() {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-sign-am-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("amendments")).unwrap();
        std::fs::write(
            dir.join("amendments/AM-001.yaml"),
            "id: \"AM-001\"\nreason: \"First. Second sentence.\"\nsemantic_diff:\n  - element: \"obligations\"\n    before: \"b\"\n    after: \"a\"\n",
        )
        .unwrap();
        let s = read_latest_amendment(&dir).expect("reads");
        assert_eq!(s.id, "AM-001");
        assert_eq!(s.reason, "First. Second sentence.");
        assert_eq!(
            s.changes,
            vec![("obligations".to_owned(), "b".to_owned(), "a".to_owned())]
        );
        assert_eq!(first_sentence(&s.reason), "First.");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
