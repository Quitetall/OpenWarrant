// SPDX-License-Identifier: Apache-2.0
//! `war mark` — the assurance mark (OW-ADR-0025, OW-WAR-0135).
//!
//! CONTINGENT. This module is built against OW-ADR-0025's recommendation:
//! Q-001 (a), a mark DERIVED from records already signed, with (c)'s file as
//! a cache that `--verify` recomputes; Q-002, baseline v1 as
//! `docs/assurance/baseline-v1.toml` proposes. Until the owner accepts the
//! ADR, that file's `status` is `proposed`, and a baseline that is not
//! `accepted` earns no mark: [`evaluate`] refuses by name
//! (`mark.baseline-not-accepted`), whatever the requirements say.
//!
//! # What a mark is
//!
//! A reading of authority already exercised, against a named baseline. It
//! has no input of its own: every field of an `oh.war/mark/v1` statement is
//! a value a record already holds, or the digest of a file in the tree. So
//! nobody can grant one, anyone can recompute one, and a mark cannot exist
//! over a resolution a human did not sign (BL-001 and BL-002 read the signed
//! record and its attestation, and nothing stands in for them).
//!
//! # Evaluation
//!
//! Each requirement of the baseline (plus any repository extension) names a
//! `check`. Each check answers exactly one of met, unmet, or UNKNOWN. A mark
//! exists only when every one is met; UNKNOWN is never met (Law 15) and is
//! reported apart from unmet. A `check` this build does not implement is
//! UNKNOWN, never dropped.
//!
//! # Strengthening, not weakening (Q45)
//!
//! `openwarrant.toml [mark]` names the baseline (`baseline = "v1"`) and an
//! optional extension file (`extra = "<path>"`). A repository's copy of v1 is
//! compared with the v1 this build ships ([`CANONICAL_V1`]): a requirement
//! removed, its check changed, marked optional, or a lower independence
//! floor, is a different mark, and the tool refuses to call it v1
//! (`mark.baseline-weakened`, each item by name). An extension adds
//! requirements; one that collides with a baseline id or lowers the floor is
//! refused the same way.
//!
//! # Recording and verifying (Q-001 (c) as a cache)
//!
//! `--record` writes the statement to `<warrant>/mark-<baseline>.json`, and
//! only when a mark is earned. A written mark is never trusted: `--verify`
//! recomputes every field from the records and names each one that moved;
//! an in-scope change to the candidate (BL-005) is named `candidate`.

use std::collections::BTreeMap;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_core::attestation::{Envelope, Statement};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

/// The statement's schema.
pub const SCHEMA: &str = "oh.war/mark/v1";
/// The evaluation `--json` returns, earned or not.
pub const EVALUATION_SCHEMA: &str = "oh.war/mark-evaluation/v1";
/// A baseline file's schema, and an extension's.
pub const BASELINE_SCHEMA: &str = "oh.war/assurance-baseline/v1";
/// Where a repository's baselines live: `baseline-<id>.toml`.
pub const BASELINE_DIR: &str = "docs/assurance";
/// Baseline v1 as this build ships it: the reference a repository's copy is
/// compared with, so v1 cannot be weakened in place.
pub const CANONICAL_V1: &str = include_str!("../../../docs/assurance/baseline-v1.toml");

/// The checks this build implements, by the name a baseline uses.
pub const CHECKS: [&str; 7] = [
    "resolution.satisfied",
    "resolution.attested",
    "obligations.independently_established",
    "resolution.located",
    "acceptance.unchanged",
    "evidence.admissible",
    "deliverables.content_addressed",
];

// ── The baseline as data ────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct Baseline {
    pub schema: String,
    pub id: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub independence_floor: Option<String>,
    /// An extension names the baseline it strengthens.
    #[serde(default)]
    pub extends: Option<String>,
    #[serde(default, rename = "requirement")]
    pub requirements: Vec<Requirement>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Requirement {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub check: String,
    /// A requirement a baseline marks optional is a relaxed one.
    #[serde(default)]
    pub optional: bool,
}

impl Baseline {
    fn parse(text: &str, what: &str) -> Result<Self, String> {
        let b: Self = toml::from_str(text).map_err(|e| format!("{what} does not parse: {e}"))?;
        if b.schema != BASELINE_SCHEMA {
            return Err(format!(
                "{what}: schema {:?}, expected {BASELINE_SCHEMA:?}",
                b.schema
            ));
        }
        if b.id.trim().is_empty() {
            return Err(format!("{what} has no id"));
        }
        if let Some(f) = &b.independence_floor
            && floor_rank(f).is_none()
        {
            return Err(format!(
                "{what}: independence_floor {f:?} is not basic, controlled or high_assurance"
            ));
        }
        let mut seen: Vec<&str> = Vec::new();
        for r in &b.requirements {
            if r.id.trim().is_empty() || r.check.trim().is_empty() {
                return Err(format!("{what}: a requirement has no id or no check"));
            }
            if seen.contains(&r.id.as_str()) {
                return Err(format!("{what}: requirement {} appears twice", r.id));
            }
            seen.push(&r.id);
        }
        Ok(b)
    }
}

/// §46.3's levels, ordered. `None` for a name that is none of them.
fn floor_rank(level: &str) -> Option<u8> {
    match level {
        "basic" => Some(0),
        "controlled" => Some(1),
        "high" | "high_assurance" => Some(2),
        _ => None,
    }
}

/// What in a baseline would make it less than the v1 this build ships.
/// Empty when `repo_v1` holds every canonical requirement, unchanged and not
/// optional, at a floor no lower.
fn weakened(canonical: &Baseline, repo_v1: &Baseline) -> Vec<String> {
    let mut out = Vec::new();
    for c in &canonical.requirements {
        match repo_v1.requirements.iter().find(|r| r.id == c.id) {
            None => out.push(format!("{} removed", c.id)),
            Some(r) if r.check != c.check => out.push(format!(
                "{} relaxed: check {:?} in place of {:?}",
                c.id, r.check, c.check
            )),
            Some(r) if r.optional && !c.optional => {
                out.push(format!("{} relaxed: marked optional", c.id));
            }
            Some(_) => {}
        }
    }
    let want = canonical.independence_floor.as_deref().unwrap_or("basic");
    let have = repo_v1.independence_floor.as_deref().unwrap_or("basic");
    if floor_rank(have) < floor_rank(want) {
        out.push(format!("independence_floor relaxed: {have} below {want}"));
    }
    out
}

/// `[mark]` in `openwarrant.toml`, read straight from the file: the
/// configuration struct ignores tables it does not know.
fn mark_config(repo: &Repository) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(repo.root.join("openwarrant.toml")) else {
        return (None, None);
    };
    let Ok(value) = toml::from_str::<toml::Value>(&text) else {
        return (None, None);
    };
    let table = value.get("mark");
    let get = |k: &str| {
        table
            .and_then(|t| t.get(k))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
    };
    (get("baseline"), get("extra"))
}

/// A baseline file as read, with where it came from.
#[derive(Debug, Clone)]
struct Loaded {
    baseline: Baseline,
    /// Repository-relative path, or `(built in)` for the shipped v1.
    path: String,
    sha256: String,
}

fn load_file(repo: &Repository, rel: &str, what: &str) -> Result<Loaded, String> {
    let bytes = std::fs::read(repo.root.join(rel)).map_err(|e| format!("{what} {rel}: {e}"))?;
    let baseline = Baseline::parse(&String::from_utf8_lossy(&bytes), &format!("{what} {rel}"))?;
    Ok(Loaded {
        baseline,
        path: rel.to_owned(),
        sha256: format!("sha256:{}", sha256_hex(&bytes)),
    })
}

// ── Results ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Met,
    Unmet,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequirementResult {
    pub id: String,
    pub check: String,
    pub result: Answer,
    pub detail: String,
    /// `baseline` or the extension's id.
    pub from: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDigest {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarrantRef {
    pub alias: String,
    pub uuid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionRef {
    pub id: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineRef {
    pub id: String,
    pub path: String,
    pub sha256: String,
    pub independence_floor: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<ExtensionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObligationRef {
    pub id: String,
    pub verdict: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Met {
    pub id: String,
    pub result: String,
}

/// `oh.war/mark/v1`. No timestamp and no tool build: recomputing it over
/// unchanged records yields the same bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    pub schema: String,
    pub warrant: WarrantRef,
    pub baseline: BaselineRef,
    pub resolution: FileDigest,
    pub attestation: FileDigest,
    pub commit: String,
    pub contract_digest: String,
    pub obligations: Vec<ObligationRef>,
    pub requirements: Vec<Met>,
}

impl Mark {
    /// The bytes `--record` writes and `--verify` compares.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
        s.push('\n');
        s.into_bytes()
    }
}

/// What `war mark` found, earned or not.
#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub schema: &'static str,
    pub warrant: String,
    pub baseline: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_status: Option<String>,
    pub requirements: Vec<RequirementResult>,
    /// Why no mark can be earned against this baseline at all, when it cannot.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub baseline_refusals: Vec<String>,
    /// The statement, only when every requirement is met under an accepted,
    /// unweakened baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,
    /// `--verify`: the fields that moved since the recorded mark.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<String>,
}

// ── The records one Warrant offers ──────────────────────────────────────────

struct Subject<'a> {
    repo: &'a Repository,
    alias: String,
    dir: Utf8PathBuf,
    rel_dir: String,
    one: crate::repo::Loaded,
    record: Option<crate::resolution_cmd::ResolutionRecord>,
    resolution_bytes: Option<Vec<u8>>,
    /// Filled by `resolution.attested`, for the statement.
    attestation: Option<FileDigest>,
    /// Filled by `obligations.independently_established`.
    obligations: Vec<ObligationRef>,
}

fn sha(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

fn is_commit_sha(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

type Outcome = (Answer, String);

fn met(d: impl Into<String>) -> Outcome {
    (Answer::Met, d.into())
}
fn unmet(d: impl Into<String>) -> Outcome {
    (Answer::Unmet, d.into())
}
fn unknown(d: impl Into<String>) -> Outcome {
    (Answer::Unknown, d.into())
}

const NO_RESOLUTION: &str =
    "no resolution.toml: implementation finished, unreviewed (Q47); its absence is the answer";

impl Subject<'_> {
    fn run(&mut self, check: &str, floor: &str) -> Outcome {
        match check {
            "resolution.satisfied" => self.satisfied(),
            "resolution.attested" => self.attested(),
            "obligations.independently_established" => self.established(floor),
            "resolution.located" => self.located(),
            "acceptance.unchanged" => self.unchanged(),
            "evidence.admissible" => self.admissible(),
            "deliverables.content_addressed" => self.content_addressed(),
            other => unknown(format!(
                "check {other:?} is not implemented by this build of war; UNKNOWN, never met \
                 and never dropped"
            )),
        }
    }

    fn satisfied(&self) -> Outcome {
        let Some(r) = &self.record else {
            return unmet(NO_RESOLUTION);
        };
        let outcome = r.resolution.common_outcome.to_string();
        let standing = r.resolution.standing.to_string();
        if outcome != "satisfied" {
            return unmet(format!("resolved {outcome}, not satisfied"));
        }
        if standing != "valid" {
            return unmet(format!("standing {standing}, not valid"));
        }
        met(format!(
            "resolved satisfied, standing valid ({})",
            r.resolution.id
        ))
    }

    fn attested(&mut self) -> Outcome {
        let Some(r) = &self.record else {
            return unmet(NO_RESOLUTION);
        };
        // `person://<actor>` is how a resolution names its resolver.
        let actor = r
            .resolution
            .resolved_by_ref
            .trim_start_matches("person://")
            .to_owned();
        let register = match self.repo.load_authority_register() {
            Ok(reg) => reg,
            Err(e) => return unmet(format!("the role register cannot be read: {e}")),
        };
        let human = register.assignments.iter().any(|a| {
            a.actor == actor && a.actor_kind == openwarrant_core::contract::ActorKind::Human
        });
        if !human {
            return unmet(format!(
                "resolved_by_ref {actor:?} is not a human in docs/authority/roles.toml (§27.2; \
                 no delegated act earns the mark)"
            ));
        }
        let rel_resolution = format!("{}/resolution.toml", self.rel_dir);
        let now = self.resolution_bytes.as_deref().map(sha256_hex);
        let candidates = self.resolve_attestations(&rel_resolution);
        if candidates.is_empty() {
            return unmet(format!(
                "no resolve attestation names {rel_resolution}: a TTY signature has no key, \
                 and a resolution signed before OW-ADR-0015 has none"
            ));
        }
        // The attestation whose resolution subject is the file on disk; the
        // latest one otherwise, which then fails as drift, by name.
        let chosen = candidates
            .iter()
            .find(|(_, digest, _)| Some(digest) == now.as_ref())
            .or_else(|| candidates.last())
            .cloned();
        let Some((path, _, predicate_actor)) = chosen else {
            return unmet("no resolve attestation");
        };
        let rel = self.repo.relative(&path);
        let mut report = Report::default();
        crate::attest::verify_file(self.repo, &path, &mut report);
        if let Some(e) = report
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error)
        {
            return unmet(format!("{rel}: {} — {}", e.rule, e.message));
        }
        if let Some(u) = report
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Unknown)
        {
            return unknown(format!("{rel}: {} — {}", u.rule, u.message));
        }
        if !report
            .diagnostics
            .iter()
            .any(|d| d.rule == "attest.verified")
        {
            return unknown(format!("{rel}: the attestation reported no verdict"));
        }
        if predicate_actor != actor {
            return unmet(format!(
                "{rel} is signed by {predicate_actor:?} and the resolution names {actor:?}"
            ));
        }
        let Ok(bytes) = std::fs::read(&path) else {
            return unknown(format!("{rel} could not be read back"));
        };
        self.attestation = Some(FileDigest {
            path: rel.clone(),
            sha256: sha(&bytes),
        });
        met(format!("{rel} verifies, signed by {actor} (human)"))
    }

    /// Resolve attestations that name this Warrant's resolution: the
    /// Warrant's own `resolve-<n>.dsse.json`, then any batch envelope
    /// (OW-WAR-0072). Each with the digest it attests for the resolution and
    /// the predicate's actor, oldest first.
    fn resolve_attestations(&self, rel_resolution: &str) -> Vec<(Utf8PathBuf, String, String)> {
        let mut own: Vec<(u64, Utf8PathBuf)> =
            crate::attest::list(self.repo, &self.dir.join("attestations"))
                .into_iter()
                .filter_map(|p| {
                    let n = p
                        .file_name()?
                        .strip_prefix("resolve-")?
                        .strip_suffix(".dsse.json")?
                        .parse::<u64>()
                        .ok()?;
                    Some((n, p))
                })
                .collect();
        own.sort();
        let mut files: Vec<Utf8PathBuf> = own.into_iter().map(|(_, p)| p).collect();
        if let Ok(batch) = crate::attest::dir_for(self.repo, "batch", "") {
            files.extend(crate::attest::list(self.repo, &batch));
        }
        let mut out = Vec::new();
        for path in files {
            let Some(statement) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|t| serde_json::from_str::<Envelope>(&t).ok())
                .and_then(|e| e.open().ok())
                .and_then(|p| serde_json::from_slice::<Statement<serde_json::Value>>(&p).ok())
            else {
                continue;
            };
            let Some(digest) = statement
                .subject
                .iter()
                .find(|s| s.name == rel_resolution)
                .and_then(|s| s.digest.get("sha256").cloned())
            else {
                continue;
            };
            let actor = statement
                .predicate
                .get("actor")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            out.push((path, digest, actor));
        }
        out
    }

    fn established(&mut self, floor: &str) -> Outcome {
        let declared = crate::resolve::declared_obligations(&self.one);
        if declared.is_empty() {
            return unmet(
                "the Warrant declares no obligation, so nothing was independently established",
            );
        }
        let records = match self.repo.load_verifications(&self.one.dir) {
            Ok(records) => records,
            Err(e) => return unknown(format!("verification records could not be read: {e}")),
        };
        let current = crate::verify::current_records(self.repo, &self.one, &records.records);
        let mut unbound = Vec::new();
        let mut problems = Vec::new();
        let mut refs = Vec::new();
        for id in &declared {
            let rel = format!("{}/verifications/{id}.toml", self.rel_dir);
            let Ok(bytes) = std::fs::read(self.repo.root.join(&rel)) else {
                problems.push(format!("{id}: no verdict"));
                continue;
            };
            let v: openwarrant_core::verification::Verification =
                match toml::from_str(&String::from_utf8_lossy(&bytes)) {
                    Ok(v) => v,
                    Err(e) => {
                        problems.push(format!("{id}: {rel} is not a verification: {e}"));
                        continue;
                    }
                };
            if v.obligation != *id {
                problems.push(format!("{id}: {rel} is about {}", v.obligation));
                continue;
            }
            if v.disposition != openwarrant_core::Disposition::Established {
                problems.push(format!(
                    "{id}: disposition {}, not established",
                    v.disposition.as_str()
                ));
                continue;
            }
            if let Err(e) = v.admissible_for(floor) {
                problems.push(format!("{id}: not admissible at {floor}: {e}"));
                continue;
            }
            if !current.contains(&v) {
                unbound.push(format!(
                    "{id}: no matching reviewed subject for current work"
                ));
                continue;
            }
            refs.push(ObligationRef {
                id: id.clone(),
                verdict: rel,
                sha256: sha(&bytes),
            });
        }
        refs.sort_by(|a, b| a.id.cmp(&b.id));
        self.obligations = refs;
        if problems.is_empty() && !unbound.is_empty() {
            unknown(unbound.join("; "))
        } else if problems.is_empty() {
            met(format!(
                "{} obligation(s) established by an independent verdict admissible at {floor}",
                declared.len()
            ))
        } else {
            unmet(problems.join("; "))
        }
    }

    fn located(&self) -> Outcome {
        let Some(r) = &self.record else {
            return unmet(NO_RESOLUTION);
        };
        let Some(l) = &r.locator else {
            return unknown(
                "no [locator]: resolved before OW-ADR-0021, so which commit was accepted was \
                 never recorded",
            );
        };
        if !is_commit_sha(&l.commit_sha) {
            return unknown(format!(
                "locator commit_sha {:?} is not forty lowercase hex",
                l.commit_sha
            ));
        }
        if !l.worktree_clean {
            return unmet(format!(
                "worktree_clean = false: {} were not committed at {}",
                if l.paths_dirty.is_empty() {
                    "some delivered bytes".to_owned()
                } else {
                    l.paths_dirty.join(", ")
                },
                &l.commit_sha[..12]
            ));
        }
        met(format!("accepted at {}, committed", &l.commit_sha[..12]))
    }

    fn unchanged(&self) -> Outcome {
        if self.record.is_none() {
            return unmet(NO_RESOLUTION);
        }
        let assessed = match crate::acceptance::assess(self.repo, "HEAD", None) {
            Ok((_, c)) => c,
            Err(e) => return unknown(format!("acceptance could not be assessed: {e}")),
        };
        let Some(a) = assessed.warrants.iter().find(|a| a.warrant == self.alias) else {
            return unknown("acceptance::assess returned nothing for this Warrant");
        };
        match a.state {
            crate::acceptance::State::Unchanged => met(format!(
                "unchanged at HEAD{} ({})",
                if a.reverified_at.is_some() {
                    ", re-verified"
                } else {
                    ""
                },
                a.scope
            )),
            crate::acceptance::State::Moved => {
                let mut parts: Vec<String> = a.in_scope.clone();
                parts.extend(a.pinned.iter().cloned());
                unmet(format!(
                    "{}: the candidate moved: {}",
                    crate::acceptance::MOVED,
                    parts.join(", ")
                ))
            }
            crate::acceptance::State::Unknown | crate::acceptance::State::Landed => {
                unknown(format!(
                    "{}: {}",
                    crate::acceptance::UNKNOWN,
                    a.reason.clone().unwrap_or_default()
                ))
            }
        }
    }

    fn admissible(&self) -> Outcome {
        let Some(r) = &self.record else {
            return unmet(NO_RESOLUTION);
        };
        let refs = &r.resolution.gate_run_refs;
        if refs.is_empty() {
            return met("the resolution relied on no gate run");
        }
        let evidence = match crate::evidence::load(self.repo, &self.dir) {
            Ok(e) => e,
            Err(e) => return unmet(format!("the gate runs cannot be read: {e}")),
        };
        let digest = Some(r.resolution.contract_digest.as_str());
        let mut bad = Vec::new();
        let mut unsure = Vec::new();
        for reference in refs {
            let Some(e) = evidence
                .iter()
                .find(|e| self.repo.relative(&e.receipt_path) == *reference)
            else {
                bad.push(format!("{reference}: missing"));
                continue;
            };
            match crate::evidence::standing(e, digest) {
                crate::evidence::Standing::Admissible => {}
                crate::evidence::Standing::ReuseUnknown(why) => {
                    unsure.push(format!("{reference}: {why}"));
                }
                crate::evidence::Standing::Stale(why)
                | crate::evidence::Standing::ReceiptInvalid(why)
                | crate::evidence::Standing::NotAPass(why) => {
                    bad.push(format!("{reference}: {why}"));
                }
            }
        }
        if !bad.is_empty() {
            return unmet(bad.join("; "));
        }
        if !unsure.is_empty() {
            return unknown(unsure.join("; "));
        }
        met(format!(
            "{} relied-on receipt(s) admissible against {}",
            refs.len(),
            r.resolution.contract_digest
        ))
    }

    fn content_addressed(&self) -> Outcome {
        let set = match self.repo.load_deliverables(&self.dir) {
            Ok(s) => s,
            Err(e) => return unmet(format!("deliverables.toml cannot be read: {e}")),
        };
        if set.records.is_empty() {
            return unmet("the Warrant declares no deliverable");
        }
        let loose: Vec<String> = set
            .records
            .iter()
            .filter(|d| !d.content_addressed || d.provenance.is_none())
            .map(|d| d.id.clone())
            .collect();
        if loose.is_empty() {
            met(format!(
                "{} deliverable(s), each content-addressed with a recorded digest",
                set.records.len()
            ))
        } else {
            unmet(format!(
                "not content-addressed with a recorded digest: {}",
                loose.join(", ")
            ))
        }
    }
}

// ── The command ─────────────────────────────────────────────────────────────

/// Where `--record` writes, and `--verify` reads by default.
#[must_use]
pub fn record_path(dir: &Utf8Path, baseline: &str) -> Utf8PathBuf {
    dir.join(format!("mark-{baseline}.json"))
}

/// `war mark <alias> [--baseline <id>]`: evaluate, and emit or refuse.
pub fn evaluate(
    repo: &Repository,
    alias: &str,
    baseline: Option<&str>,
) -> Result<(Report, Evaluation), RepoError> {
    let mut report = Report::default();
    let dir = repo.warrant_dir(alias)?;
    let rel_dir = repo.relative(&dir);
    let one = repo.load_warrant(&dir)?;
    let uuid = one
        .validated
        .as_ref()
        .map(|v| v.uuid.to_string())
        .ok_or_else(|| {
            RepoError::Message(format!(
                "{alias}: the manifest did not validate, so there is no Warrant to mark"
            ))
        })?;
    let record = repo.load_resolution(&dir)?;
    let resolution_bytes = std::fs::read(dir.join("resolution.toml")).ok();

    let (config_baseline, config_extra) = mark_config(repo);
    let id = baseline
        .map(str::to_owned)
        .or(config_baseline)
        .unwrap_or_else(|| "v1".to_owned());
    let mut ev = Evaluation {
        schema: EVALUATION_SCHEMA,
        warrant: alias.to_owned(),
        baseline: id.clone(),
        baseline_path: None,
        baseline_status: None,
        requirements: Vec::new(),
        baseline_refusals: Vec::new(),
        mark: None,
        recorded: None,
        moved: Vec::new(),
    };

    // The baseline: the repository's file, or for v1 the one this build ships.
    let rel = format!("{BASELINE_DIR}/baseline-{id}.toml");
    let base = if repo.root.join(&rel).is_file() {
        load_file(repo, &rel, "baseline")
    } else if id == "v1" {
        Baseline::parse(CANONICAL_V1, "the built-in baseline v1").map(|baseline| Loaded {
            baseline,
            path: "(built in)".to_owned(),
            sha256: sha(CANONICAL_V1.as_bytes()),
        })
    } else {
        Err(format!("no baseline {id:?}: {rel} does not exist"))
    };
    let base = match base {
        Ok(b) => b,
        Err(why) => {
            report.push(Diagnostic::error("mark.baseline-unreadable", rel, why));
            return Ok((report, ev));
        }
    };
    ev.baseline_path = Some(base.path.clone());
    ev.baseline_status = Some(base.baseline.status.clone());
    if base.baseline.id != id {
        ev.baseline_refusals.push(format!(
            "{} names itself {:?}, not {id:?}",
            base.path, base.baseline.id
        ));
    }
    if base.baseline.status != "accepted" {
        ev.baseline_refusals.push(format!(
            "baseline {id} is {:?}, not accepted: it is not in force, and a baseline not in \
             force earns no mark (OW-ADR-0025{})",
            base.baseline.status,
            if id == "v1" {
                " is not accepted by the owner"
            } else {
                ""
            }
        ));
    }
    let mut floor = base
        .baseline
        .independence_floor
        .clone()
        .unwrap_or_else(|| "basic".to_owned());
    if id == "v1" {
        match Baseline::parse(CANONICAL_V1, "the built-in baseline v1") {
            Ok(canonical) => {
                for w in weakened(&canonical, &base.baseline) {
                    ev.baseline_refusals
                        .push(format!("weakened, so not v1: {w}"));
                }
                if canonical.status != "accepted" && base.baseline.status == "accepted" {
                    report.push(Diagnostic::warn(
                        "mark.baseline-canonical-proposed",
                        base.path.clone(),
                        format!(
                            "{} says accepted; the baseline v1 this build ships says {:?} \
                             (OW-ADR-0025 not accepted). The repository's copy is what is \
                             evaluated",
                            base.path, canonical.status
                        ),
                    ));
                }
            }
            Err(e) => ev.baseline_refusals.push(e),
        }
    }

    // The extension, if the repository declares one.
    let mut extensions: Vec<(Loaded, Vec<Requirement>)> = Vec::new();
    if let Some(extra) = config_extra {
        match load_file(repo, &extra, "baseline extension") {
            Ok(ext) => {
                if ext.baseline.extends.as_deref() != Some(id.as_str()) {
                    ev.baseline_refusals.push(format!(
                        "{extra} extends {:?}, not {id:?}",
                        ext.baseline.extends.as_deref().unwrap_or("nothing")
                    ));
                }
                for r in &ext.baseline.requirements {
                    if base.baseline.requirements.iter().any(|b| b.id == r.id) {
                        ev.baseline_refusals.push(format!(
                            "weakened, so not {id}: {extra} redefines {} (an extension adds \
                             requirements; it does not replace one)",
                            r.id
                        ));
                    }
                }
                if let Some(f) = &ext.baseline.independence_floor {
                    if floor_rank(f) < floor_rank(&floor) {
                        ev.baseline_refusals.push(format!(
                            "weakened, so not {id}: {extra} lowers independence_floor to {f}"
                        ));
                    } else {
                        floor.clone_from(f);
                    }
                }
                let reqs = ext.baseline.requirements.clone();
                extensions.push((ext, reqs));
            }
            Err(why) => ev.baseline_refusals.push(why),
        }
    }

    // Evaluate every requirement, whatever the baseline's standing: what is
    // met is worth saying even when no mark can follow.
    let mut s = Subject {
        repo,
        alias: alias.to_owned(),
        dir: dir.clone(),
        rel_dir: rel_dir.clone(),
        one,
        record,
        resolution_bytes,
        attestation: None,
        obligations: Vec::new(),
    };
    let mut all: Vec<(Requirement, String)> = base
        .baseline
        .requirements
        .iter()
        .cloned()
        .map(|r| (r, "baseline".to_owned()))
        .collect();
    for (ext, reqs) in &extensions {
        all.extend(reqs.iter().cloned().map(|r| (r, ext.baseline.id.clone())));
    }
    for (r, from) in all {
        let (result, detail) = s.run(&r.check, &floor);
        ev.requirements.push(RequirementResult {
            id: r.id.clone(),
            check: r.check.clone(),
            result,
            detail,
            from,
        });
    }

    for r in &ev.requirements {
        let line = format!("{} ({}): {}", r.id, r.check, r.detail);
        report.push(match r.result {
            Answer::Met => Diagnostic::pass("mark.requirement-met", format!("{alias} {line}")),
            Answer::Unmet => Diagnostic::error(
                "mark.requirement-unmet",
                format!("{rel_dir}/"),
                format!("{alias} {line}"),
            ),
            Answer::Unknown => Diagnostic::unknown(
                "mark.requirement-unknown",
                format!("{rel_dir}/"),
                format!("{alias} {} ({}): UNKNOWN — {}", r.id, r.check, r.detail),
            ),
        });
    }
    for why in &ev.baseline_refusals {
        let rule = if why.starts_with("weakened") {
            "mark.baseline-weakened"
        } else if why.contains("not accepted") {
            "mark.baseline-not-accepted"
        } else {
            "mark.baseline-invalid"
        };
        report.push(Diagnostic::error(rule, base.path.clone(), why.clone()));
    }

    let unmet_ids: Vec<&str> = ev
        .requirements
        .iter()
        .filter(|r| r.result == Answer::Unmet)
        .map(|r| r.id.as_str())
        .collect();
    let unknown_ids: Vec<&str> = ev
        .requirements
        .iter()
        .filter(|r| r.result == Answer::Unknown)
        .map(|r| r.id.as_str())
        .collect();
    if unmet_ids.is_empty() && unknown_ids.is_empty() && ev.baseline_refusals.is_empty() {
        let (Some(record), Some(bytes), Some(attestation)) =
            (&s.record, &s.resolution_bytes, s.attestation.clone())
        else {
            report.push(Diagnostic::unknown(
                "mark.refused",
                format!("{rel_dir}/"),
                format!("{alias}: every requirement met and a binding is missing; no mark"),
            ));
            return Ok((report, ev));
        };
        let mark = Mark {
            schema: SCHEMA.to_owned(),
            warrant: WarrantRef {
                alias: alias.to_owned(),
                uuid,
            },
            baseline: BaselineRef {
                id: id.clone(),
                path: base.path.clone(),
                sha256: base.sha256.clone(),
                independence_floor: floor,
                extensions: extensions
                    .iter()
                    .map(|(e, _)| ExtensionRef {
                        id: e.baseline.id.clone(),
                        path: e.path.clone(),
                        sha256: e.sha256.clone(),
                    })
                    .collect(),
            },
            resolution: FileDigest {
                path: format!("{rel_dir}/resolution.toml"),
                sha256: sha(bytes),
            },
            attestation,
            commit: record
                .locator
                .as_ref()
                .map(|l| l.commit_sha.clone())
                .unwrap_or_default(),
            contract_digest: record.resolution.contract_digest.clone(),
            obligations: s.obligations.clone(),
            requirements: ev
                .requirements
                .iter()
                .map(|r| Met {
                    id: r.id.clone(),
                    result: "met".to_owned(),
                })
                .collect(),
        };
        let named = std::iter::once(id.clone())
            .chain(mark.baseline.extensions.iter().map(|e| e.id.clone()))
            .collect::<Vec<_>>()
            .join(" + ");
        report.push(Diagnostic::pass(
            "mark.earned",
            format!(
                "{alias}: {named} mark — {} requirement(s) met; resolution {}, commit {}. It \
                 says one accepted result at this commit and scope, against this baseline; not \
                 the codebase, not a release, not every line read",
                mark.requirements.len(),
                mark.resolution.sha256,
                &mark.commit[..std::cmp::min(12, mark.commit.len())]
            ),
        ));
        ev.mark = Some(mark);
    } else {
        let mut parts = Vec::new();
        if !unmet_ids.is_empty() {
            parts.push(format!("unmet {}", unmet_ids.join(", ")));
        }
        if !unknown_ids.is_empty() {
            parts.push(format!("UNKNOWN {}", unknown_ids.join(", ")));
        }
        if !ev.baseline_refusals.is_empty() {
            parts.push(format!("baseline {id} cannot be earned (above)"));
        }
        let severity = if unmet_ids.is_empty() && ev.baseline_refusals.is_empty() {
            Severity::Unknown
        } else {
            Severity::Error
        };
        report.push(Diagnostic::new(
            severity,
            "mark.refused",
            Some(format!("{rel_dir}/")),
            format!("{alias}: no {id} mark — {}", parts.join("; ")),
        ));
    }
    report.note(
        "A mark is derived (OW-ADR-0025 Q-001 (a), as recommended; the ADR is proposed): it \
         reads the signed resolution and adds no authority of its own.",
    );
    Ok((report, ev))
}

/// `war mark <alias> --record`: write the statement, only when earned.
pub fn record(
    repo: &Repository,
    alias: &str,
    baseline: Option<&str>,
) -> Result<(Report, Evaluation), RepoError> {
    let (mut report, mut ev) = evaluate(repo, alias, baseline)?;
    let dir = repo.warrant_dir(alias)?;
    let path = record_path(&dir, &ev.baseline);
    let rel = repo.relative(&path);
    let Some(mark) = &ev.mark else {
        report.push(Diagnostic::error(
            "mark.not-recorded",
            rel.clone(),
            format!("{alias}: no mark was earned, so nothing is written to {rel}"),
        ));
        return Ok((report, ev));
    };
    let tmp = path.with_extension("json.war-tmp");
    std::fs::write(&tmp, mark.to_bytes())
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|source| RepoError::Io {
            context: format!("could not write {rel}"),
            source,
        })?;
    report.push(Diagnostic::pass(
        "mark.recorded",
        format!(
            "{rel} written; a cache that `war mark {alias} --verify` recomputes, never trusted"
        ),
    ));
    ev.recorded = Some(rel);
    Ok((report, ev))
}

/// `war mark <alias> --verify [--file <mark>]`: recompute every binding and
/// name each one that moved.
pub fn verify(
    repo: &Repository,
    alias: &str,
    baseline: Option<&str>,
    file: Option<&Utf8Path>,
) -> Result<(Report, Evaluation), RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let (now_report, mut ev) = evaluate(repo, alias, baseline)?;
    let path = file.map_or_else(|| record_path(&dir, &ev.baseline), Utf8Path::to_path_buf);
    let path = if path.is_absolute() {
        path
    } else {
        repo.root.join(path)
    };
    let rel = repo.relative(&path);
    let mut report = Report::default();
    let Ok(text) = std::fs::read_to_string(&path) else {
        report.push(Diagnostic::unknown(
            "mark.no-record",
            rel.clone(),
            format!(
                "{alias}: no recorded mark at {rel}, so there is nothing to verify; `war mark \
                 {alias}` computes one and `--record` writes it"
            ),
        ));
        return Ok((report, ev));
    };
    let recorded: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            report.push(Diagnostic::error(
                "mark.stale",
                rel.clone(),
                format!("{rel} is not JSON: {e}"),
            ));
            return Ok((report, ev));
        }
    };
    ev.recorded = Some(rel.clone());

    // The requirements that do not hold today, each by name; BL-005's check
    // is the candidate.
    let mut moved: Vec<(String, String)> = Vec::new();
    for r in &ev.requirements {
        if r.result == Answer::Met {
            continue;
        }
        let name = if r.check == "acceptance.unchanged" {
            "candidate".to_owned()
        } else {
            format!("requirement {}", r.id)
        };
        let state = if r.result == Answer::Unknown {
            "UNKNOWN"
        } else {
            "unmet"
        };
        moved.push((
            name,
            format!("{} ({}) is {state} today: {}", r.id, r.check, r.detail),
        ));
    }
    for why in &ev.baseline_refusals {
        moved.push(("baseline".to_owned(), why.clone()));
    }

    // Every field, recomputed from the records as they are.
    let now = recompute(repo, alias, &ev);
    let fields = [
        "schema",
        "warrant",
        "baseline",
        "resolution",
        "attestation",
        "commit",
        "contract_digest",
        "obligations",
        "requirements",
    ];
    let empty = serde_json::Value::Null;
    for f in fields {
        let was = recorded.get(f).unwrap_or(&empty);
        // With no mark today, only the bindings that could be rebuilt are
        // compared; the requirements that failed are named above.
        let Some(is) = now.get(f) else {
            continue;
        };
        if was != is && !moved.iter().any(|(n, _)| n == f) {
            moved.push((
                f.to_owned(),
                format!("recorded {}, recomputed {}", compact(was), compact(is)),
            ));
        }
    }
    if let Some(obj) = recorded.as_object() {
        for k in obj.keys() {
            if !fields.contains(&k.as_str()) {
                moved.push((k.clone(), "a field no mark carries".to_owned()));
            }
        }
    }

    // What the evaluation said about the baseline, kept beside the verdict.
    for d in now_report.diagnostics {
        if d.rule == "mark.baseline-canonical-proposed" {
            report.push(d);
        }
    }
    if moved.is_empty() {
        report.push(Diagnostic::pass(
            "mark.verified",
            format!(
                "{rel}: every binding recomputes from the records today ({} requirement(s) met)",
                ev.requirements.len()
            ),
        ));
    } else {
        for (name, detail) in &moved {
            report.push(Diagnostic::error(
                "mark.stale",
                rel.clone(),
                format!("{alias}: `{name}` moved — {detail}"),
            ));
        }
    }
    ev.moved = moved.into_iter().map(|(n, _)| n).collect();
    Ok((report, ev))
}

/// The statement's fields as they are today, earned or not: each binding
/// that still has a value, `null` where there is none.
fn recompute(repo: &Repository, alias: &str, ev: &Evaluation) -> serde_json::Value {
    if let Some(m) = &ev.mark {
        return serde_json::to_value(m).unwrap_or_default();
    }
    // No mark today: rebuild what can be, so the fields that moved are named
    // beside the requirements that failed.
    let mut out = BTreeMap::new();
    out.insert("schema", serde_json::json!(SCHEMA));
    if let Ok(dir) = repo.warrant_dir(alias) {
        let rel_dir = repo.relative(&dir);
        if let Ok(bytes) = std::fs::read(dir.join("resolution.toml")) {
            out.insert(
                "resolution",
                serde_json::json!({ "path": format!("{rel_dir}/resolution.toml"), "sha256": sha(&bytes) }),
            );
        }
        if let Ok(Some(r)) = repo.load_resolution(&dir) {
            out.insert(
                "contract_digest",
                serde_json::json!(r.resolution.contract_digest),
            );
            if let Some(l) = r.locator {
                out.insert("commit", serde_json::json!(l.commit_sha));
            }
        }
    }
    serde_json::to_value(out).unwrap_or_default()
}

fn compact(v: &serde_json::Value) -> String {
    let s = v.to_string();
    if s.len() > 160 {
        format!("{}…", &s[..s.floor_char_boundary(160)])
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(text: &str) -> Baseline {
        Baseline::parse(text, "test").expect("parses")
    }

    #[test]
    fn the_shipped_v1_parses_and_every_check_is_implemented() {
        let v1 = b(CANONICAL_V1);
        assert_eq!(v1.id, "v1");
        assert_eq!(v1.requirements.len(), 6);
        for r in &v1.requirements {
            assert!(
                CHECKS.contains(&r.check.as_str()),
                "{} is not implemented",
                r.check
            );
        }
    }

    /// The shipped v1 is proposed or accepted, nothing else: the owner's
    /// acceptance flips the file, and no code has to change with it.
    #[test]
    fn the_shipped_v1_is_proposed_or_accepted() {
        let status = b(CANONICAL_V1).status;
        assert!(status == "proposed" || status == "accepted", "{status}");
    }

    #[test]
    fn v1_is_not_weaker_than_itself() {
        let v1 = b(CANONICAL_V1);
        assert!(weakened(&v1, &v1).is_empty());
    }

    /// The refusals: a removed requirement, a changed check, an optional one,
    /// and a lower floor are each named.
    #[test]
    fn a_weakened_v1_is_named() {
        let v1 = b(CANONICAL_V1);
        let removed = CANONICAL_V1.replacen("id = \"BL-004\"", "id = \"BL-904\"", 1);
        let w = weakened(&v1, &b(&removed));
        assert!(w.iter().any(|x| x == "BL-004 removed"), "{w:?}");
        let changed = CANONICAL_V1.replacen(
            "check = \"resolution.attested\"",
            "check = \"resolution.satisfied\"",
            1,
        );
        assert!(
            weakened(&v1, &b(&changed))
                .iter()
                .any(|x| x.starts_with("BL-002 relaxed"))
        );
        let optional = CANONICAL_V1.replacen(
            "check = \"resolution.located\"",
            "check = \"resolution.located\"\noptional = true",
            1,
        );
        assert!(
            weakened(&v1, &b(&optional))
                .iter()
                .any(|x| x == "BL-004 relaxed: marked optional")
        );
        let basic = CANONICAL_V1.replacen(
            "independence_floor = \"controlled\"",
            "independence_floor = \"basic\"",
            1,
        );
        assert!(
            weakened(&v1, &b(&basic))
                .iter()
                .any(|x| x.starts_with("independence_floor relaxed"))
        );
    }

    #[test]
    fn a_baseline_that_names_a_requirement_twice_is_refused() {
        let twice = format!(
            "{CANONICAL_V1}\n[[requirement]]\nid = \"BL-001\"\ncheck = \"resolution.satisfied\"\n"
        );
        assert!(Baseline::parse(&twice, "t").is_err());
        let floor = CANONICAL_V1.replacen(
            "independence_floor = \"controlled\"",
            "independence_floor = \"lenient\"",
            1,
        );
        assert!(Baseline::parse(&floor, "t").is_err());
    }

    #[test]
    fn floors_are_ordered() {
        assert!(floor_rank("basic") < floor_rank("controlled"));
        assert!(floor_rank("controlled") < floor_rank("high_assurance"));
        assert_eq!(floor_rank("high"), floor_rank("high_assurance"));
        assert_eq!(floor_rank("lenient"), None);
    }

    #[test]
    fn a_mark_serializes_the_same_twice() {
        let m = Mark {
            schema: SCHEMA.to_owned(),
            warrant: WarrantRef {
                alias: "X-WAR-0001".into(),
                uuid: "u".into(),
            },
            baseline: BaselineRef {
                id: "v1".into(),
                path: "p".into(),
                sha256: "sha256:0".into(),
                independence_floor: "controlled".into(),
                extensions: vec![],
            },
            resolution: FileDigest {
                path: "r".into(),
                sha256: "sha256:1".into(),
            },
            attestation: FileDigest {
                path: "a".into(),
                sha256: "sha256:2".into(),
            },
            commit: "c".repeat(40),
            contract_digest: "sha256:3".into(),
            obligations: vec![],
            requirements: vec![],
        };
        assert_eq!(m.to_bytes(), m.clone().to_bytes());
        let back: Mark = serde_json::from_slice(&m.to_bytes()).expect("round trip");
        assert_eq!(back, m);
    }
}
