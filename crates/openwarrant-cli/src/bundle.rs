// SPDX-License-Identifier: Apache-2.0
//! `war verify <alias> --bundle` / `--run` — the verification bundle and a
//! configured verifier (slice C3; SAS §46, §75.2).
//!
//! The request `war verify` emits names obligations; a blind verifier then
//! has to go and find the atoms, the deliverables, the receipts and the
//! plants by itself, in a context that is not the performer's. Sessions did
//! that with a script. The bundle is that script inside the tool: canonical
//! JSON (`oh.war/verification-bundle/v2`) carrying the request with the
//! authorized contract digest filled in, every basis atom, deliverable bytes,
//! the plants that name the alias, the `#[test]` names in Rust deliverables,
//! the committed gate runs with what they printed, and the prior
//! verifications — and its own token estimate. Written to
//! `verifications/bundle-<digest>.json` under a digest domain of its own.
//!
//! # Bounded (t-9f7e)
//!
//! A bundle carrying every deliverable of a large Warrant was 241k tokens
//! (OW-WAR-0112) and the verifier timed out on it. Every bundle now has a
//! reading budget, `[verify] max_bundle_tokens`, measured over its whole
//! canonical JSON:
//!
//! - A Warrant whose bundle fits is sent **whole** (`scope = "warrant"`), as
//!   before: one bundle, one verifier call, each deliverable whole under
//!   `max_excerpt_bytes` else its head; a gate stream over that cap is
//!   excerpted as below (its tail, the sections of this Warrant's plants,
//!   the obligations' terms, every FAIL/ERROR line), never cut to its tail.
//! - One that does not fit is split into **one bundle per obligation**
//!   (`scope = "obligation"`). Each carries the deliverables that obligation
//!   names — by `obligation_refs` in `deliverables.toml`, or by path in its
//!   statement, scope or evidence — and lists the others it does not carry,
//!   by digest. The budget is shared over the carried files and gate outputs
//!   by water-filling: a file that fits its share is carried whole; one that
//!   does not is carried as numbered-line **excerpts** (its head, then the
//!   lines naming the obligation's backticked terms), always with the whole
//!   file's sha256, byte and line counts and `truncated: true`.
//!
//! Nothing the obligation names is dropped silently: `obligation_evidence`
//! says, per obligation, which named paths were carried and which were not
//! (and why), and for each named term where it is shown and where the budget
//! cut it. Every choice is a function of the tree and the configuration, so
//! the same inputs give the same bytes and digest. The bundle carries nothing
//! the performer said: the same sources as before, fewer of their bytes.
//!
//! `--run` hands each bundle path to `[verify] verifier_argv`, one call per
//! bundle, each under the deadline, and ingests what each prints through the
//! unchanged `verify::ingest` — so every refusal that seam has
//! (self-verification, wrong Warrant, malformed envelope) applies to a
//! configured verifier too. A response answering an obligation its bundle did
//! not carry is refused unread.

use camino::Utf8PathBuf;
use openwarrant_compiler::digest::sha256_hex;
use openwarrant_compiler::{DigestDomain, sha256_digest};
use serde::Serialize;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};
use crate::verify::RequestedObligation;

pub const SCHEMA: &str = "oh.war/verification-bundle/v2";

/// Lines of a file's head always offered first in an excerpt: the module
/// documentation, which says what the file is.
const HEAD_LINES: usize = 30;
/// Around each line naming a term: a little before, more after (a test's
/// body, a function's).
const BEFORE: usize = 4;
const AFTER: usize = 20;
/// The tail of a gate stream kept as `text` when the stream is excerpted:
/// the totals line and the last plants.
const TAIL_BYTES: usize = 2048;

#[derive(Debug, Clone, Serialize)]
pub struct BundledAtom {
    pub path: String,
    pub role: String,
    pub sha256: String,
    pub text: String,
}

/// Consecutive lines of a file, numbered from 1, exactly as in the file.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Excerpt {
    pub start_line: usize,
    pub end_line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundledDeliverable {
    pub id: String,
    pub title: String,
    pub target_ref: String,
    /// False when the file could not be read; `error` says why, and there is
    /// no `text` — never an empty string, which would read as an empty file.
    pub present: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Of the WHOLE file, whatever is carried.
    pub sha256: String,
    pub bytes: u64,
    pub lines: usize,
    /// True when fewer than all the file's bytes are carried.
    pub truncated: bool,
    /// The whole file; or, in a warrant-scope bundle, its head under
    /// `max_excerpt_bytes`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// In an obligation-scope bundle, the lines carried of a file too large
    /// for its share of the budget.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub excerpts: Vec<Excerpt>,
    /// Why this file is in an obligation-scope bundle: `obligation_refs`, or
    /// the path in the obligation that names it.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub carried_because: Vec<String>,
    /// `#[test]` function names, for a Rust source deliverable (whole file).
    pub test_names: Vec<String>,
}

/// A deliverable of the Warrant an obligation-scope bundle does not carry:
/// named so a verifier knows it exists, with its digest.
#[derive(Debug, Clone, Serialize)]
pub struct NotCarried {
    pub id: String,
    pub target_ref: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BundledPlant {
    pub file: String,
    pub line: usize,
    pub text: String,
}

/// A path an obligation names, and what became of it.
#[derive(Debug, Clone, Serialize)]
pub struct NamedPath {
    pub term: String,
    /// Deliverable ids it resolved to, carried in this bundle.
    pub carried: Vec<String>,
    /// Set when nothing was carried for it, saying why.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub absent: Option<String>,
}

/// A backticked term an obligation names, and where its bytes are.
#[derive(Debug, Clone, Serialize)]
pub struct NamedTerm {
    pub term: String,
    /// Carried items whose carried bytes contain it.
    pub shown_in: Vec<String>,
    /// Carried items whose WHOLE bytes contain it but whose carried bytes do
    /// not: the budget cut it.
    pub cut_from: Vec<String>,
}

/// What one obligation names, resolved against this bundle.
#[derive(Debug, Clone, Serialize)]
pub struct ObligationEvidence {
    pub obligation: String,
    pub selection: String,
    pub named_paths: Vec<NamedPath>,
    pub terms: Vec<NamedTerm>,
}

/// Exact required review inputs. UTF-8 is readable; binary bytes remain lossless.
/// Missing input stays explicit. These fixed inputs are never excerpted.
#[derive(Debug, Clone, Serialize)]
pub struct RequiredSource {
    pub path: String,
    pub kind: String,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Bundle {
    pub schema: String,
    pub warrant: String,
    /// `warrant`: every obligation, one bundle. `obligation`: one of several.
    pub scope: String,
    pub authorized_contract_digest: String,
    pub request: crate::verify::VerificationRequest,
    pub obligation_evidence: Vec<ObligationEvidence>,
    pub atoms: Vec<BundledAtom>,
    pub required_sources: Vec<RequiredSource>,
    pub deliverables: Vec<BundledDeliverable>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub deliverables_not_carried: Vec<NotCarried>,
    pub plants: Vec<BundledPlant>,
    pub gate_runs: Vec<serde_json::Value>,
    pub prior_verifications: Vec<openwarrant_core::verification::Verification>,
    pub budget_tokens: u64,
    /// True when the bundle could not be brought under the budget (its
    /// fixed parts — atoms, request, records — exceed it): written and sent
    /// anyway, and said.
    pub over_budget: bool,
    /// Over the bundle's whole canonical JSON, computed with this field 0.
    pub estimated_tokens: u64,
    pub token_method: String,
}

/// `#[test]` function names, by a line scan that needs no parser.
fn test_names(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut armed = false;
    for line in source.lines() {
        let t = line.trim();
        if t == "#[test]" {
            armed = true;
            continue;
        }
        if armed && let Some(rest) = t.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                out.push(name);
            }
            armed = false;
        } else if armed && !t.starts_with("//") && !t.starts_with('#') {
            armed = false;
        }
    }
    out
}

/// The plant lines naming `alias`, across the battery.
fn plants_naming(repo: &Repository, alias: &str) -> Vec<BundledPlant> {
    let mut out = Vec::new();
    let dir = repo.root.join("conformance/plants.d");
    let mut files: Vec<Utf8PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| Utf8PathBuf::from_path_buf(e.path()).ok())
                .filter(|p| p.extension() == Some("sh"))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        for (i, line) in text.lines().enumerate() {
            if line.contains(alias) && (line.contains("plant") || line.contains("printf 'ok")) {
                out.push(BundledPlant {
                    file: repo.relative(&f),
                    line: i + 1,
                    text: line.trim().to_owned(),
                });
            }
        }
    }
    out
}

// ── what an obligation names ────────────────────────────────────────────────

const PATH_EXTENSIONS: &[&str] = &[
    "rs", "sh", "md", "toml", "yaml", "yml", "json", "jsonl", "ts", "txt", "lock", "py", "js",
    "html", "css",
];

/// A repository path, a directory (trailing `/`), or neither.
fn path_like(s: &str) -> bool {
    if s.is_empty()
        || s.contains(char::is_whitespace)
        || s.contains("://")
        || s.contains(['@', '=', '<', '>', '*', '$', '(', ')', '"', '\''])
        || s.starts_with('-')
    {
        return false;
    }
    if let Some(dir) = s.strip_suffix('/') {
        return dir.chars().any(char::is_alphanumeric);
    }
    let name = s.rsplit('/').next().unwrap_or(s);
    match name.rsplit_once('.') {
        Some((stem, ext)) => !stem.is_empty() && PATH_EXTENSIONS.contains(&ext),
        None => false,
    }
}

fn deliverable_id_like(s: &str) -> bool {
    s.strip_prefix("D-")
        .is_some_and(|rest| rest.len() >= 3 && rest.chars().take(3).all(|c| c.is_ascii_digit()))
}

/// The paths and terms an obligation names, in the order it names them.
/// Paths: backticked spans, or bare words holding a `/`, that look like a
/// repository path or directory, or a deliverable id. Terms: every other
/// backticked span of four characters or more, and within a span holding
/// spaces, each word of six or more carrying `.`, `_`, `:` or `-` (a rule
/// id, a function path).
fn named(o: &RequestedObligation) -> (Vec<String>, Vec<String>) {
    let text = format!("{}\n{}\n{}", o.statement, o.scope, o.evidence);
    let trim = |w: &str| -> String {
        let w = w.trim_matches(|c: char| ",;:()[]\"'.".contains(c));
        let w = w.strip_suffix("'s").unwrap_or(w);
        w.strip_prefix("./").unwrap_or(w).to_owned()
    };
    let push = |v: &mut Vec<String>, s: String| {
        if !v.contains(&s) {
            v.push(s);
        }
    };
    let mut paths: Vec<String> = Vec::new();
    let mut terms: Vec<String> = Vec::new();
    for (i, span) in text.split('`').enumerate() {
        if i % 2 == 1 {
            let s = span.trim();
            let bare = s.strip_prefix("./").unwrap_or(s);
            if path_like(bare) || deliverable_id_like(bare) {
                push(&mut paths, bare.to_owned());
                continue;
            }
            if s.chars().count() >= 4 {
                push(&mut terms, s.to_owned());
            }
            if s.contains(char::is_whitespace) {
                for w in s.split_whitespace() {
                    let w = trim(w);
                    if path_like(&w) {
                        push(&mut paths, w);
                    } else if w.chars().count() >= 6
                        && w.starts_with(char::is_alphanumeric)
                        && w.contains(['.', '_', ':', '-'])
                        && w.chars()
                            .all(|c| c.is_alphanumeric() || "._:-/".contains(c))
                    {
                        push(&mut terms, w);
                    }
                }
            }
        } else {
            for w in span.split_whitespace() {
                let w = trim(w);
                if path_like(&w) && w.contains('/') {
                    push(&mut paths, w);
                }
            }
        }
    }
    (paths, terms)
}

fn names_deliverable(term: &str, d: &openwarrant_core::deliverable::Deliverable) -> bool {
    let tr = d.target_ref.as_str();
    if deliverable_id_like(term) {
        return d.id == term;
    }
    if term.ends_with('/') {
        return tr.starts_with(term) || tr.contains(&format!("/{term}"));
    }
    tr == term || tr.ends_with(&format!("/{term}"))
}

// ── excerpts ────────────────────────────────────────────────────────────────

/// Picks whole lines within a byte allowance, in the order offered. A range
/// that does not fit stops where it runs out; later, smaller ranges may still
/// fit. Deterministic: the result depends only on the lines and the offers.
struct Picker<'a> {
    lines: Vec<&'a str>,
    take: Vec<bool>,
    left: usize,
}

impl<'a> Picker<'a> {
    fn new(text: &'a str, allowance: usize) -> Self {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let take = vec![false; lines.len()];
        Self {
            lines,
            take,
            left: allowance,
        }
    }

    fn offer(&mut self, from: usize, to: usize) {
        for i in from..to.min(self.lines.len()) {
            if self.take[i] {
                continue;
            }
            let n = self.lines[i].len();
            if n > self.left {
                return;
            }
            self.take[i] = true;
            self.left -= n;
        }
    }

    /// Offer windows around each line containing a term, one match of each
    /// term per round, so no one term's many hits crowd out the rest.
    fn offer_terms(&mut self, terms: &[String], before: usize, after: usize) {
        let hits: Vec<Vec<usize>> = terms
            .iter()
            .map(|t| {
                self.lines
                    .iter()
                    .enumerate()
                    .filter(|(_, l)| l.contains(t.as_str()))
                    .map(|(i, _)| i)
                    .collect()
            })
            .collect();
        let rounds = hits.iter().map(Vec::len).max().unwrap_or(0);
        for r in 0..rounds {
            // The line naming the term before its surroundings: a window of
            // long lines must not crowd out the line it is around.
            for h in &hits {
                if let Some(&i) = h.get(r) {
                    self.offer(i, i + 1);
                }
            }
            for h in &hits {
                if let Some(&i) = h.get(r) {
                    self.offer(i.saturating_sub(before), i + after + 1);
                }
            }
        }
    }

    /// The first `lines` lines, but no more than `bytes` of them.
    fn offer_head(&mut self, lines: usize, bytes: usize) {
        let mut used = 0;
        for i in 0..lines.min(self.lines.len()) {
            used += self.lines[i].len();
            if used > bytes {
                return;
            }
            self.offer(i, i + 1);
        }
    }

    fn excerpts(&self) -> Vec<Excerpt> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.lines.len() {
            if !self.take[i] {
                i += 1;
                continue;
            }
            let start = i;
            let mut text = String::new();
            while i < self.lines.len() && self.take[i] {
                text.push_str(self.lines[i]);
                i += 1;
            }
            out.push(Excerpt {
                start_line: start + 1,
                end_line: i,
                text,
            });
        }
        out
    }
}

/// Windows of a file no whole line of which fits: canonical JSON is one
/// line (SECTIONS.json, 77 KB), and whole-line excerpts carried nothing of
/// it but its digest (t-3293). The head, a window around the first hit of
/// each term, and the tail, within `allowance` bytes, cut on char
/// boundaries; each names the line it lies in.
fn byte_windows(text: &str, allowance: usize, terms: &[String]) -> Vec<Excerpt> {
    let len = text.len();
    let edge = allowance / 4;
    let mut spans: Vec<(usize, usize)> = vec![(0, edge.min(len))];
    let per = if terms.is_empty() {
        0
    } else {
        (allowance / 2) / terms.len()
    };
    for t in terms {
        if let Some(at) = text.find(t.as_str()) {
            let from = at.saturating_sub(per / 4);
            spans.push((from, (from + per.max(t.len())).min(len)));
        }
    }
    spans.push((len.saturating_sub(edge), len));
    spans.sort_unstable();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (a, b) in spans {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    let floor = |mut i: usize| {
        while i > 0 && !text.is_char_boundary(i) {
            i -= 1;
        }
        i
    };
    let line_of = |i: usize| text[..i].matches('\n').count() + 1;
    let mut left = allowance;
    let mut out = Vec::new();
    for (a, b) in merged {
        let (a, b) = (floor(a), floor(b.min(a + left)));
        if b <= a {
            continue;
        }
        left -= b - a;
        out.push(Excerpt {
            start_line: line_of(a),
            end_line: line_of(floor(b - 1).max(a)),
            text: text[a..b].to_owned(),
        });
        if left == 0 {
            break;
        }
    }
    out
}

/// Give each item at most its size, the budget shared evenly among those
/// that want more than the rest can spare (smallest first; ties by order).
fn water_fill(sizes: &[usize], total: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by_key(|&i| (sizes[i], i));
    let mut out = vec![0; sizes.len()];
    let mut left = total;
    for (k, &i) in order.iter().enumerate() {
        let share = left / (sizes.len() - k);
        out[i] = sizes[i].min(share);
        left -= out[i];
    }
    out
}

// ── the sources, read once ──────────────────────────────────────────────────

struct Stream {
    bytes: Option<Vec<u8>>,
    recorded: Option<String>,
}

struct Run {
    gate: String,
    run_id: String,
    run: serde_json::Value,
    receipt: Option<serde_json::Value>,
    stdout: Stream,
    stderr: Stream,
}

struct File {
    record: openwarrant_core::deliverable::Deliverable,
    read: Result<Vec<u8>, String>,
}

struct Sources {
    alias: String,
    authorized_contract_digest: String,
    request: crate::verify::VerificationRequest,
    atoms: Vec<BundledAtom>,
    required_sources: Vec<RequiredSource>,
    files: Vec<File>,
    runs: Vec<Run>,
    plants: Vec<BundledPlant>,
    /// What a gate stream's `== … ==` section header must contain for the
    /// section to be offered: the alias, and each header a delivered plant
    /// file echoes (a plant's banner need not name the Warrant).
    heads: Vec<String>,
    /// The check names this Warrant's plants print (`ok    <name>`), read
    /// from its delivered plant files and any plant file an obligation names:
    /// a plant need not print a header of its own, and its lines then sit
    /// under another plant's section (t-7aca).
    checks: Vec<String>,
    prior: Vec<openwarrant_core::verification::Verification>,
    cap: usize,
    budget: u64,
    root: Utf8PathBuf,
    warrant_dir: Utf8PathBuf,
}

fn required_sources(
    repo: &Repository,
    subject: &crate::verify::ReviewedSubject,
) -> Result<Vec<RequiredSource>, RepoError> {
    let mut required = std::collections::BTreeMap::new();
    let mut found = std::collections::BTreeSet::new();
    for (path, digest) in &subject.gate_inputs {
        required.insert(path.clone(), ("gate-input", digest.clone()));
    }
    for (path, digest) in &subject.gate_evidence {
        required.insert(path.clone(), ("gate-evidence", digest.clone()));
    }
    for (path, digest) in &subject.fixtures {
        required.insert(path.clone(), ("fixture", digest.clone()));
    }
    let directory = repo.root.join(&repo.config.paths.gates);
    if directory.exists() {
        for entry in directory
            .read_dir_utf8()
            .map_err(|e| RepoError::Message(e.to_string()))?
        {
            let path = entry
                .map_err(|e| RepoError::Message(e.to_string()))?
                .into_path();
            if !matches!(path.extension(), Some("yaml" | "yml")) {
                continue;
            }
            let relative = repo.relative(&path);
            let Some(bytes) = crate::verify::file_bytes(repo, &relative)? else {
                continue;
            };
            let Some(doc) = std::str::from_utf8(&bytes)
                .ok()
                .and_then(|s| openwarrant_core::structured::parse(s).ok())
            else {
                continue;
            };
            let key = format!(
                "{}@{}",
                doc.scalar("gate_id").unwrap_or_default(),
                doc.scalar("version").unwrap_or_default()
            );
            if let Some(digest) = subject.gate_definitions.get(&key) {
                if !found.insert(key.clone()) {
                    return Err(RepoError::Message(format!(
                        "verify.subject-stale: duplicate required gate {key} during bundle capture"
                    )));
                }
                required.insert(relative, ("gate-definition", digest.clone()));
            }
        }
    }
    for (key, digest) in &subject.gate_definitions {
        if digest != "missing" && !found.contains(key) {
            return Err(RepoError::Message(format!(
                "verify.subject-stale: required gate {key} disappeared during bundle capture"
            )));
        }
    }
    let mut out = Vec::new();
    for (path, (kind, expected)) in required {
        let read = crate::verify::file_bytes(repo, &path)?;
        let actual = read
            .as_ref()
            .map(|b| format!("sha256:{}", sha256_hex(b)))
            .unwrap_or_else(|| "missing".to_owned());
        if actual != expected {
            return Err(RepoError::Message(format!(
                "verify.subject-stale: required source {path} changed during bundle capture"
            )));
        }
        let (text, bytes) = match read {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(text) => (Some(text), None),
                Err(e) => (None, Some(e.into_bytes())),
            },
            None => (None, None),
        };
        out.push(RequiredSource {
            path,
            kind: kind.to_owned(),
            sha256: actual,
            present: text.is_some() || bytes.is_some(),
            text,
            bytes,
        });
    }
    Ok(out)
}

fn load(repo: &Repository, alias: &str, performer: &str) -> Result<Sources, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let request = crate::verify::request(repo, alias, performer)?;
    let required_sources = required_sources(repo, &request.reviewed_subject)?;
    let authorized_contract_digest = repo
        .load_authorization(&dir)
        .ok()
        .flatten()
        .map(|a| a.revision.contract_digest)
        .unwrap_or_default();
    let mut atoms = Vec::new();
    if let Some(basis) = &one.basis {
        for a in &basis.atoms {
            atoms.push(BundledAtom {
                path: a.source.clone(),
                role: a.role.clone(),
                sha256: sha256_hex(&a.bytes),
                text: String::from_utf8_lossy(&a.bytes).into_owned(),
            });
        }
    }
    let files: Vec<File> = repo
        .load_deliverables(&dir)
        .map(|set| {
            set.records
                .into_iter()
                .map(|record| {
                    let read = std::fs::read(repo.root.join(&record.target_ref))
                        .map_err(|e| format!("unreadable: {e}"));
                    File { record, read }
                })
                .collect()
        })
        .unwrap_or_default();
    // OW-WAR-0146: what each gate printed travels with the run. A verifier
    // deciding from the bundle alone cannot settle an obligation whose
    // evidence is a gate's output if the output is not there — the first
    // blind run (OW-WAR-0004) said so for every obligation.
    let runs = crate::evidence::load(repo, &dir)
        .map(|ev| {
            ev.iter()
                .map(|e| {
                    let receipt = e.receipt.as_ref().map(carried_receipt);
                    let stream = |name: &str| {
                        let from_receipt = receipt
                            .as_ref()
                            .and_then(|r| r[format!("{name}_ref")].as_str())
                            .map(|rel| repo.root.join(rel));
                        let beside = Utf8PathBuf::from(
                            e.run_path
                                .as_str()
                                .replace(".run.toml", &format!(".{name}.txt")),
                        );
                        Stream {
                            bytes: std::fs::read(from_receipt.unwrap_or(beside)).ok(),
                            recorded: receipt
                                .as_ref()
                                .and_then(|r| r[format!("{name}_digest")].as_str())
                                .map(str::to_owned),
                        }
                    };
                    Run {
                        gate: e.run.gate.clone(),
                        run_id: e.run.id.clone(),
                        run: serde_json::to_value(&e.run).unwrap_or_default(),
                        receipt: receipt.clone(),
                        stdout: stream("stdout"),
                        stderr: stream("stderr"),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let prior = repo
        .load_verifications(&dir)
        .map(|v| v.records)
        .unwrap_or_default();
    let mut heads = vec![alias.to_owned()];
    for f in &files {
        if !f.record.target_ref.starts_with("conformance/") {
            continue;
        }
        if let Ok(bytes) = &f.read {
            heads.extend(echoed_headers(&String::from_utf8_lossy(bytes)));
        }
    }
    heads.sort();
    heads.dedup();
    let mut plant_sources: Vec<String> = files
        .iter()
        .filter(|f| f.record.target_ref.starts_with("conformance/"))
        .filter_map(|f| f.read.as_ref().ok())
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .collect();
    for o in &request.obligations {
        for path in named(o).0 {
            let named_plant = path.starts_with("conformance/plants.d/")
                || (path.ends_with(".sh") && !path.contains('/'));
            if !named_plant {
                continue;
            }
            let full = if path.contains('/') {
                repo.root.join(&path)
            } else {
                repo.root.join("conformance/plants.d").join(&path)
            };
            if let Ok(text) = std::fs::read_to_string(&full) {
                heads.extend(echoed_headers(&text));
                plant_sources.push(text);
            }
        }
    }
    heads.sort();
    heads.dedup();
    let mut checks: Vec<String> = plant_sources.iter().flat_map(|t| check_names(t)).collect();
    checks.sort();
    checks.dedup();
    Ok(Sources {
        heads,
        checks,
        alias: alias.to_owned(),
        authorized_contract_digest,
        request,
        atoms,
        required_sources,
        files,
        runs,
        plants: plants_naming(repo, alias),
        prior,
        cap: repo.config.verify.max_excerpt_bytes(),
        budget: repo.config.verify.max_bundle_tokens(),
        root: repo.root.clone(),
        warrant_dir: dir,
    })
}

/// The names a plant file gives its checks: the first argument of a
/// `<prefix>_ok`, `_fail`, `_expect` or `_pass` call, and of a `printf 'ok
/// %-34s …' "name"`. One with a shell expansion in it is skipped; its
/// printed form is unknown.
fn check_names(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        let mut rest = line;
        while let Some(at) = rest.find(['_', '\'']) {
            let tail = &rest[at..];
            let after = ["_ok \"", "_fail \"", "_expect \"", "_pass \""]
                .iter()
                .find(|m| tail.starts_with(**m))
                .map(|m| &tail[m.len()..])
                .or_else(|| {
                    tail.strip_prefix("'ok    %-34s")
                        .and_then(|t| t.find("' \"").map(|i| &t[i + 3..]))
                });
            if let Some(after) = after
                && let Some(end) = after.find('"')
            {
                let name = &after[..end];
                if !name.is_empty() && !name.contains('$') {
                    out.push(name.to_owned());
                }
            }
            rest = &rest[at + 1..];
        }
    }
    out
}

/// Whether a stream line is a check result named `name`: `ok    <name>`,
/// `FAIL  <name>` or `UNKNOWN <name>`, the name followed by a space or the
/// end of the line.
fn is_check_line(line: &str, names: &[String]) -> bool {
    let rest = line
        .strip_prefix("ok    ")
        .or_else(|| line.strip_prefix("FAIL  "))
        .or_else(|| line.strip_prefix("UNKNOWN "));
    let Some(rest) = rest else {
        return false;
    };
    names.iter().any(|n| {
        rest.strip_prefix(n.as_str())
            .is_some_and(|t| t.is_empty() || t.starts_with(' ') || t.starts_with('\n'))
    })
}

/// The `== … ==` headers a plant file echoes literally: `echo "== x =="`.
/// One with a shell expansion in it is skipped; its printed form is unknown.
fn echoed_headers(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("echo ")?;
            let quoted = rest.trim().strip_prefix(['"', '\''])?;
            let end = quoted.rfind(['"', '\''])?;
            let text = &quoted[..end];
            (text.starts_with("== ") && text.ends_with(" ==") && !text.contains('$'))
                .then(|| text.to_owned())
        })
        .collect()
}

/// A receipt as a bundle carries it: every field but `working_directory`.
///
/// That field is the absolute path of the checkout the gate ran in — a fact
/// about the performer's machine, not about the work. Carried, it made the
/// bundle (its bytes, its digest, its token estimate) depend on where the
/// repository sits: `war eval`'s scratch programs are named with a PID and a
/// nanosecond count whose digit counts vary, and the same fixture measured
/// 3043 and 3042 tokens (t-ade4). A bundle is a function of the tree and the
/// configuration only; the receipt file in the tree still holds the field,
/// and `receipt_digest` still covers it.
fn carried_receipt(r: &openwarrant_core::gate_run::GateReceipt) -> serde_json::Value {
    let mut v = serde_json::to_value(r).unwrap_or_default();
    if let Some(map) = v.as_object_mut() {
        map.remove("working_directory");
    }
    v
}

// ── one stream, one file, carried ───────────────────────────────────────────

/// How much of a thing to carry: its head under a cap (warrant scope), or a
/// byte allowance filled with excerpts (obligation scope).
#[derive(Clone, Copy)]
enum Carry {
    Head(usize),
    Excerpt(usize),
}

/// One captured stream of a gate run. An absent file is `captured: false`
/// with no `text` — never an empty string, which would read as "printed
/// nothing". `text` is the tail (the whole stream when it fits); an
/// excerpted stream also carries `excerpts`: the sections whose `== … ==`
/// header names the Warrant or obligation, the lines naming a term, and the
/// FAIL/ERROR lines. `mismatch` only when the receipt records a digest for
/// the stream and the file's differs.
fn captured(
    s: &Stream,
    carry: Carry,
    heads: &[String],
    checks: &[String],
    terms: &[String],
) -> serde_json::Value {
    let Some(bytes) = &s.bytes else {
        return serde_json::json!({ "captured": false });
    };
    let sha = sha256_hex(bytes);
    let (tail_cap, allowance) = match carry {
        Carry::Head(cap) => (cap, None),
        Carry::Excerpt(a) if bytes.len() <= a => (a, None),
        Carry::Excerpt(a) => {
            let tail = TAIL_BYTES.min(a);
            (tail, Some(a - tail))
        }
    };
    let truncated = bytes.len() > tail_cap;
    let tail = if truncated {
        &bytes[bytes.len() - tail_cap..]
    } else {
        &bytes[..]
    };
    let mut v = serde_json::json!({
        "captured": true,
        "sha256": sha,
        "bytes": bytes.len(),
        "truncated": truncated,
        "text": String::from_utf8_lossy(tail),
    });
    if let Some(allowance) = allowance {
        let whole = String::from_utf8_lossy(bytes);
        let mut p = Picker::new(&whole, allowance);
        let n = p.lines.len();
        let mut sections = Vec::new();
        let mut open: Option<usize> = None;
        for i in 0..=n {
            let header = i < n && p.lines[i].starts_with("== ");
            if header || i == n {
                if let Some(start) = open.take() {
                    sections.push((start, i));
                }
                if header && heads.iter().any(|h| p.lines[i].contains(h.as_str())) {
                    open = Some(i);
                }
            }
        }
        // This Warrant's own check lines first, wherever they sit: a section
        // is offered whole or until the allowance runs out, and a long one
        // could crowd them out.
        let own: Vec<usize> = (0..n)
            .filter(|&i| is_check_line(p.lines[i], checks))
            .collect();
        for i in own {
            p.offer(i, i + 1);
        }
        for (a, b) in sections {
            p.offer(a, b);
        }
        p.offer_terms(terms, 0, 0);
        let bad: Vec<usize> = (0..n)
            .filter(|&i| p.lines[i].starts_with("FAIL") || p.lines[i].starts_with("ERROR"))
            .collect();
        for i in bad {
            p.offer(i, i + 1);
        }
        v["excerpts"] = serde_json::to_value(p.excerpts()).unwrap_or_default();
    }
    if let Some(want) = &s.recorded
        && want.trim_start_matches("sha256:") != sha
    {
        v["mismatch"] = serde_json::Value::Bool(true);
    }
    v
}

fn carry_file(
    f: &File,
    carry: Carry,
    terms: &[String],
    carried_because: Vec<String>,
) -> BundledDeliverable {
    let d = &f.record;
    let bytes = match &f.read {
        Ok(b) => b,
        Err(e) => {
            return BundledDeliverable {
                id: d.id.clone(),
                title: d.title.clone(),
                target_ref: d.target_ref.clone(),
                present: false,
                error: Some(e.clone()),
                sha256: String::new(),
                bytes: 0,
                lines: 0,
                truncated: false,
                text: None,
                excerpts: vec![],
                carried_because,
                test_names: vec![],
            };
        }
    };
    let whole = String::from_utf8_lossy(bytes);
    let lines = whole.split_inclusive('\n').count();
    let (truncated, text, excerpts) = match carry {
        Carry::Head(cap) if bytes.len() > cap => (
            true,
            Some(String::from_utf8_lossy(&bytes[..cap]).into_owned()),
            vec![],
        ),
        Carry::Excerpt(a) if bytes.len() > a => {
            let mut p = Picker::new(&whole, a);
            p.offer_head(HEAD_LINES, a / 4);
            p.offer_terms(terms, BEFORE, AFTER);
            let mut excerpts = p.excerpts();
            if excerpts.is_empty() {
                excerpts = byte_windows(&whole, a, terms);
            }
            (true, None, excerpts)
        }
        _ => (false, Some(whole.clone().into_owned()), vec![]),
    };
    BundledDeliverable {
        id: d.id.clone(),
        title: d.title.clone(),
        target_ref: d.target_ref.clone(),
        present: true,
        error: None,
        sha256: sha256_hex(bytes),
        bytes: bytes.len() as u64,
        lines,
        truncated,
        text,
        excerpts,
        carried_because,
        test_names: if d.target_ref.ends_with(".rs") {
            test_names(&whole)
        } else {
            vec![]
        },
    }
}

fn carried_text(d: &BundledDeliverable) -> String {
    let mut s = d.text.clone().unwrap_or_default();
    for e in &d.excerpts {
        s.push_str(&e.text);
    }
    s
}

fn stream_text(v: &serde_json::Value) -> String {
    let mut s = v["text"].as_str().unwrap_or_default().to_owned();
    if let Some(ex) = v["excerpts"].as_array() {
        for e in ex {
            s.push_str(e["text"].as_str().unwrap_or_default());
        }
    }
    s
}

/// Where each term an obligation names is shown, and where the budget cut
/// it. `carried` pairs each carried deliverable with its index in the
/// sources.
fn term_places(
    src: &Sources,
    terms: &[String],
    carried: &[(usize, &BundledDeliverable)],
    runs: &[serde_json::Value],
) -> Vec<NamedTerm> {
    terms
        .iter()
        .map(|t| {
            let t = t.as_str();
            let mut shown_in = Vec::new();
            let mut cut_from = Vec::new();
            for (i, d) in carried {
                if carried_text(d).contains(t) {
                    shown_in.push(d.id.clone());
                } else if src.files[*i]
                    .read
                    .as_ref()
                    .is_ok_and(|b| String::from_utf8_lossy(b).contains(t))
                {
                    cut_from.push(d.id.clone());
                }
            }
            for (r, v) in src.runs.iter().zip(runs) {
                for (name, s) in [("stdout", &r.stdout), ("stderr", &r.stderr)] {
                    let label = format!("{} {name}", r.gate);
                    if stream_text(&v[name]).contains(t) {
                        shown_in.push(label);
                    } else if s
                        .bytes
                        .as_ref()
                        .is_some_and(|b| String::from_utf8_lossy(b).contains(t))
                    {
                        cut_from.push(label);
                    }
                }
            }
            if src.plants.iter().any(|p| p.text.contains(t)) {
                shown_in.push("plants".to_owned());
            }
            if src
                .atoms
                .iter()
                .any(|a| a.role != "assurance" && a.text.contains(t))
            {
                shown_in.push("atoms".to_owned());
            }
            NamedTerm {
                term: t.to_owned(),
                shown_in,
                cut_from,
            }
        })
        .collect()
}

fn named_paths(src: &Sources, paths: &[String], carried_ids: &[String]) -> Vec<NamedPath> {
    paths
        .iter()
        .map(|term| {
            let carried: Vec<String> = src
                .files
                .iter()
                .filter(|f| names_deliverable(term, &f.record))
                .map(|f| f.record.id.clone())
                .filter(|id| carried_ids.contains(id))
                .collect();
            let absent = carried.is_empty().then(|| {
                let local = src.warrant_dir.join(term.trim_end_matches('/'));
                if local.exists() {
                    format!(
                        "a record of {} ({}), not a deliverable: not carried",
                        src.alias,
                        local.strip_prefix(&src.root).unwrap_or(&local)
                    )
                } else if src.root.join(term.trim_end_matches('/')).exists() {
                    format!(
                        "a repository path that is not a deliverable of {}: not carried",
                        src.alias
                    )
                } else {
                    format!(
                        "no deliverable of {} and no file at this path: not carried",
                        src.alias
                    )
                }
            });
            NamedPath {
                term: term.clone(),
                carried,
                absent,
            }
        })
        .collect()
}

fn run_value(r: &Run, stdout: serde_json::Value, stderr: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "gate": r.gate,
        "run_id": r.run_id,
        "run": r.run,
        "receipt": r.receipt,
        "stdout": stdout,
        "stderr": stderr,
    })
}

/// The estimate over the whole canonical JSON, the field itself 0.
fn finish(mut bundle: Bundle) -> Result<Bundle, RepoError> {
    bundle.estimated_tokens = 0;
    let text = serde_jcs::to_string(&bundle)
        .map_err(|e| RepoError::Message(format!("could not canonicalise the bundle: {e}")))?;
    bundle.estimated_tokens = openwarrant_core::tokens::estimate(text.len() as u64);
    bundle.over_budget = bundle.estimated_tokens > bundle.budget_tokens;
    Ok(bundle)
}

/// The whole Warrant in one bundle: every deliverable, each whole under
/// `max_excerpt_bytes` else its head; every stream's tail under the same cap.
fn warrant_bundle(src: &Sources) -> Result<Bundle, RepoError> {
    let deliverables: Vec<BundledDeliverable> = src
        .files
        .iter()
        .map(|f| carry_file(f, Carry::Head(src.cap), &[], vec![]))
        .collect();
    // A stream over the cap is excerpted, not cut to its tail: the battery
    // prints this Warrant's section wherever its plant sits, usually far from
    // the end, and a tail-only stream left every obligation resting on it
    // unshown (OW-WAR-0118, 0120, 0144 on 2026-09-27; t-7aca).
    let mut stream_keys: Vec<String> = src
        .request
        .obligations
        .iter()
        .flat_map(|o| named(o).1)
        .collect();
    stream_keys.push(src.alias.clone());
    stream_keys.sort();
    stream_keys.dedup();
    let gate_runs: Vec<serde_json::Value> = src
        .runs
        .iter()
        .map(|r| {
            run_value(
                r,
                captured(
                    &r.stdout,
                    Carry::Excerpt(src.cap),
                    &src.heads,
                    &src.checks,
                    &stream_keys,
                ),
                captured(
                    &r.stderr,
                    Carry::Excerpt(src.cap),
                    &src.heads,
                    &src.checks,
                    &stream_keys,
                ),
            )
        })
        .collect();
    let carried: Vec<(usize, &BundledDeliverable)> = deliverables.iter().enumerate().collect();
    let ids: Vec<String> = deliverables.iter().map(|d| d.id.clone()).collect();
    let obligation_evidence = src
        .request
        .obligations
        .iter()
        .map(|o| {
            let (paths, terms) = named(o);
            ObligationEvidence {
                obligation: o.id.clone(),
                selection: "warrant: every deliverable of the Warrant".to_owned(),
                named_paths: named_paths(src, &paths, &ids),
                terms: term_places(src, &terms, &carried, &gate_runs),
            }
        })
        .collect();
    finish(Bundle {
        schema: SCHEMA.to_owned(),
        warrant: src.alias.clone(),
        scope: "warrant".to_owned(),
        authorized_contract_digest: src.authorized_contract_digest.clone(),
        request: src.request.clone(),
        obligation_evidence,
        atoms: src.atoms.clone(),
        required_sources: src.required_sources.clone(),
        deliverables,
        deliverables_not_carried: vec![],
        plants: src.plants.clone(),
        gate_runs,
        prior_verifications: src.prior.clone(),
        budget_tokens: src.budget,
        over_budget: false,
        estimated_tokens: 0,
        token_method: openwarrant_core::tokens::METHOD.to_owned(),
    })
}

/// One obligation's bundle, its carried bytes within `allowance`.
fn obligation_bundle_at(
    src: &Sources,
    o: &RequestedObligation,
    allowance: usize,
) -> Result<Bundle, RepoError> {
    let (paths, terms) = named(o);
    // Which files: those declaring the obligation, and those it names.
    let mut because: Vec<Vec<String>> = vec![vec![]; src.files.len()];
    for (i, f) in src.files.iter().enumerate() {
        if f.record.obligation_refs.contains(&o.id) {
            because[i].push("obligation_refs".to_owned());
        }
        for p in &paths {
            if names_deliverable(p, &f.record) {
                because[i].push(format!("named `{p}`"));
            }
        }
    }
    let mut selection = "named: the deliverables whose obligation_refs list it, and those its \
                         statement, scope or evidence names by path"
        .to_owned();
    if because.iter().all(Vec::is_empty) {
        selection = "all: the obligation names no deliverable and none lists it".to_owned();
        for b in &mut because {
            b.push("all".to_owned());
        }
    }
    let chosen: Vec<usize> = (0..src.files.len())
        .filter(|&i| !because[i].is_empty())
        .collect();
    // The allowance, water-filled over the chosen files and every stream.
    let mut sizes: Vec<usize> = chosen
        .iter()
        .map(|&i| src.files[i].read.as_ref().map_or(0, Vec::len))
        .collect();
    for r in &src.runs {
        for s in [&r.stdout, &r.stderr] {
            sizes.push(s.bytes.as_ref().map_or(0, Vec::len));
        }
    }
    let shares = water_fill(&sizes, allowance);
    let mut stream_keys = terms.clone();
    stream_keys.push(src.alias.clone());
    let mut keys = stream_keys.clone();
    keys.push(o.id.clone());
    let deliverables: Vec<BundledDeliverable> = chosen
        .iter()
        .zip(&shares)
        .map(|(&i, &share)| {
            carry_file(
                &src.files[i],
                Carry::Excerpt(share),
                &keys,
                because[i].clone(),
            )
        })
        .collect();
    // Obligation ids repeat across Warrants (every one has an OBL-001), so a
    // gate's corpus-wide output is searched for the alias and the terms only.
    let heads = &src.heads;
    let gate_runs: Vec<serde_json::Value> = src
        .runs
        .iter()
        .enumerate()
        .map(|(k, r)| {
            let out = shares[chosen.len() + 2 * k];
            let err = shares[chosen.len() + 2 * k + 1];
            run_value(
                r,
                captured(
                    &r.stdout,
                    Carry::Excerpt(out),
                    heads,
                    &src.checks,
                    &stream_keys,
                ),
                captured(
                    &r.stderr,
                    Carry::Excerpt(err),
                    heads,
                    &src.checks,
                    &stream_keys,
                ),
            )
        })
        .collect();
    let deliverables_not_carried = src
        .files
        .iter()
        .enumerate()
        .filter(|(i, _)| !chosen.contains(i))
        .map(|(_, f)| NotCarried {
            id: f.record.id.clone(),
            target_ref: f.record.target_ref.clone(),
            sha256: f.read.as_ref().map(|b| sha256_hex(b)).unwrap_or_default(),
            bytes: f.read.as_ref().map_or(0, |b| b.len() as u64),
        })
        .collect();
    let carried: Vec<(usize, &BundledDeliverable)> =
        chosen.iter().copied().zip(deliverables.iter()).collect();
    let ids: Vec<String> = deliverables.iter().map(|d| d.id.clone()).collect();
    let evidence = ObligationEvidence {
        obligation: o.id.clone(),
        selection,
        named_paths: named_paths(src, &paths, &ids),
        terms: term_places(src, &terms, &carried, &gate_runs),
    };
    let mut request = src.request.clone();
    request.obligations.retain(|x| x.id == o.id);
    finish(Bundle {
        schema: SCHEMA.to_owned(),
        warrant: src.alias.clone(),
        scope: "obligation".to_owned(),
        authorized_contract_digest: src.authorized_contract_digest.clone(),
        request,
        obligation_evidence: vec![evidence],
        atoms: src.atoms.clone(),
        required_sources: src.required_sources.clone(),
        deliverables,
        deliverables_not_carried,
        plants: src.plants.clone(),
        gate_runs,
        prior_verifications: src
            .prior
            .iter()
            .filter(|v| v.obligation == o.id)
            .cloned()
            .collect(),
        budget_tokens: src.budget,
        over_budget: false,
        estimated_tokens: 0,
        token_method: openwarrant_core::tokens::METHOD.to_owned(),
    })
}

/// One obligation's bundle under the budget: the fixed parts measured with
/// nothing carried, the rest shared, re-measured and shrunk until the whole
/// canonical JSON fits (escaping makes carried bytes cost more than their
/// length). Deterministic: every step is a function of the sources.
fn obligation_bundle(src: &Sources, o: &RequestedObligation) -> Result<Bundle, RepoError> {
    let budget = usize::try_from(src.budget.saturating_mul(4)).unwrap_or(usize::MAX);
    let fixed = obligation_bundle_at(src, o, 0)?;
    let fixed_bytes = usize::try_from(fixed.estimated_tokens.saturating_mul(4)).unwrap_or(0);
    if fixed_bytes >= budget {
        return Ok(fixed);
    }
    let mut allowance = budget - fixed_bytes;
    for _ in 0..16 {
        let b = obligation_bundle_at(src, o, allowance)?;
        if !b.over_budget || allowance == 0 {
            return Ok(b);
        }
        let size = usize::try_from(b.estimated_tokens.saturating_mul(4)).unwrap_or(usize::MAX);
        let over = size.saturating_sub(budget);
        allowance = allowance.saturating_sub(over + over / 4 + 1024);
    }
    obligation_bundle_at(src, o, 0)
}

/// The bundles for a Warrant: one whole when it fits the budget, else one per
/// obligation (a Warrant with no obligations keeps its one bundle).
pub fn build_all(
    repo: &Repository,
    alias: &str,
    performer: &str,
) -> Result<Vec<Bundle>, RepoError> {
    let src = load(repo, alias, performer)?;
    let whole = warrant_bundle(&src)?;
    if !whole.over_budget || src.request.obligations.is_empty() {
        return Ok(vec![whole]);
    }
    src.request
        .obligations
        .iter()
        .map(|o| obligation_bundle(&src, o))
        .collect()
}

/// Canonical bytes and the domain-prefixed digest.
pub fn canonical(bundle: &Bundle) -> Result<(String, String), RepoError> {
    let text = serde_jcs::to_string(bundle)
        .map_err(|e| RepoError::Message(format!("could not canonicalise the bundle: {e}")))?;
    let digest = sha256_digest(DigestDomain::VerificationBundle, bundle)
        .map_err(|e| RepoError::Message(e.to_string()))?;
    Ok((text, digest))
}

fn short(digest: &str) -> String {
    digest
        .trim_start_matches("sha256:")
        .chars()
        .take(16)
        .collect()
}

/// Build and write each `verifications/bundle-<digest16>.json`.
pub fn write(
    repo: &Repository,
    alias: &str,
    performer: &str,
) -> Result<Vec<(Utf8PathBuf, Bundle, String)>, RepoError> {
    // The directory first: an obligation may name `verifications/`, and
    // whether it exists is part of what the bundle says about it — a first
    // run must not describe a tree its own writing then changes.
    let dir = repo.warrant_dir(alias)?.join("verifications");
    std::fs::create_dir_all(&dir).map_err(|source| RepoError::Io {
        context: format!("could not create {dir}"),
        source,
    })?;
    let bundles = build_all(repo, alias, performer)?;
    let mut out = Vec::new();
    for bundle in bundles {
        let (text, digest) = canonical(&bundle)?;
        let path = dir.join(format!("bundle-{}.json", short(&digest)));
        std::fs::write(&path, text + "\n").map_err(|source| RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
        out.push((path, bundle, digest));
    }
    Ok(out)
}

fn which(b: &Bundle) -> String {
    b.request
        .obligations
        .iter()
        .map(|o| o.id.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe(b: &Bundle) -> String {
    let paths_absent: usize = b
        .obligation_evidence
        .iter()
        .map(|e| e.named_paths.iter().filter(|p| p.absent.is_some()).count())
        .sum();
    let terms_unshown: usize = b
        .obligation_evidence
        .iter()
        .map(|e| e.terms.iter().filter(|t| t.shown_in.is_empty()).count())
        .sum();
    format!(
        "{} scope, {} obligation(s), {} atom(s), {} deliverable(s) carried ({} truncated, {} \
         not carried), {} plant line(s), {} gate run(s), {} prior verification(s); {} named \
         path(s) not carried, {} named term(s) shown nowhere; ~{} of {} tokens ({})",
        b.scope,
        b.request.obligations.len(),
        b.atoms.len(),
        b.deliverables.len(),
        b.deliverables.iter().filter(|d| d.truncated).count(),
        b.deliverables_not_carried.len(),
        b.plants.len(),
        b.gate_runs.len(),
        b.prior_verifications.len(),
        paths_absent,
        terms_unshown,
        b.estimated_tokens,
        b.budget_tokens,
        b.token_method
    )
}

fn over_budget(report: &mut Report, repo: &Repository, path: &Utf8PathBuf, b: &Bundle) {
    if b.over_budget {
        report.push(Diagnostic::warn(
            "verify.bundle-over-budget",
            repo.relative(path),
            format!(
                "~{} tokens against `[verify] max_bundle_tokens` = {}: the atoms, request and \
                 records alone exceed it, so no more could be cut. It is written and sent as it \
                 is; expect the verifier to be slow",
                b.estimated_tokens, b.budget_tokens
            ),
        ));
    }
}

/// `war verify <alias> --bundle`: write them and say what each holds.
pub fn emit(repo: &Repository, alias: &str, performer: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    for (path, bundle, digest) in write(repo, alias, performer)? {
        report.push(Diagnostic::pass(
            "verify.bundle",
            format!(
                "{alias} [{}]: bundle {digest} written to {} — {}",
                which(&bundle),
                repo.relative(&path),
                describe(&bundle)
            ),
        ));
        over_budget(&mut report, repo, &path, &bundle);
    }
    Ok(report)
}

/// The obligations a response answers that its bundle did not carry. A
/// response that does not parse answers none here; the ingest refuses it.
fn outside(stdout: &str, bundle: &Bundle) -> Vec<String> {
    let Ok(response) = toml::from_str::<crate::verify::VerificationResponse>(stdout) else {
        return vec![];
    };
    response
        .verifications
        .iter()
        .map(|v| v.obligation.clone())
        .filter(|id| !bundle.request.obligations.iter().any(|o| &o.id == id))
        .collect()
}

enum Outcome {
    Answered(String),
    Failed(String),
    TimedOut,
}

/// One verifier call on one bundle, under the deadline.
fn call(
    argv: &[String],
    bundle_path: &Utf8PathBuf,
    root: &Utf8PathBuf,
    timeout: u64,
) -> Result<Outcome, RepoError> {
    let mut child = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .arg(bundle_path.as_str())
        .current_dir(root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RepoError::Message(format!(
                "verify.verifier-failed: could not run {:?}: {e}",
                argv[0]
            ))
        })?;
    // Drained while the child runs: a verifier printing more than a pipe
    // holds must not block on a reader that waits for it to exit.
    let drain = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut s = String::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_string(&mut s);
            }
            s
        })
    };
    let out = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let err = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn std::io::Read + Send>),
    );
    let started = std::time::Instant::now();
    let deadline = std::time::Duration::from_secs(timeout);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if started.elapsed() > deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(e) => return Err(RepoError::Message(format!("verify.verifier-failed: {e}"))),
        }
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    Ok(match status {
        None => Outcome::TimedOut,
        Some(s) if !s.success() => Outcome::Failed(format!(
            "exit {s}: {}",
            stderr.trim().chars().take(400).collect::<String>()
        )),
        Some(_) => Outcome::Answered(stdout),
    })
}

/// `war verify <alias> --run`: the configured verifier on each bundle, then
/// the unchanged ingest of each answer. A bundle whose call fails or times
/// out records nothing for its obligations and is reported by name; the
/// others are still ingested — each obligation is judged on its own.
pub fn run(repo: &Repository, alias: &str, performer: &str) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let argv = &repo.config.verify.verifier_argv;
    if argv.is_empty() {
        report.push(Diagnostic::error(
            "verify.no-verifier",
            "openwarrant.toml".to_owned(),
            "no verifier is configured: set `[verify] verifier_argv` to a command that reads a \
             bundle path and prints an oh.war/verification-response/v1 document on stdout. A seam \
             with nothing on the other side says so (§75.2)"
                .to_owned(),
        ));
        return Ok(report);
    }
    let bundles = write(repo, alias, performer)?;
    let timeout = repo.config.verify.timeout_secs();
    // Under `verifications/responses/`, not beside the records: every
    // `verifications/*.toml` is read as one obligation's verification, and a
    // whole response is not one — kept beside them, it made every reader of
    // the records (`war inbox`, `war resolve`) refuse the Warrant.
    let responses = repo
        .warrant_dir(alias)?
        .join("verifications")
        .join(RESPONSES_DIR);
    for (bundle_path, bundle, digest) in &bundles {
        let which = which(bundle);
        report.push(Diagnostic::pass(
            "verify.bundle",
            format!(
                "{alias} [{which}]: bundle {digest} ({} scope, ~{} tokens) written to {}",
                bundle.scope,
                bundle.estimated_tokens,
                repo.relative(bundle_path)
            ),
        ));
        over_budget(&mut report, repo, bundle_path, bundle);
        let stdout = match call(argv, bundle_path, &repo.root, timeout)? {
            Outcome::TimedOut => {
                report.push(Diagnostic::error(
                    "verify.verifier-timeout",
                    argv.join(" "),
                    format!(
                        "{alias} [{which}]: the verifier was killed after {timeout}s; nothing \
                         was ingested for it"
                    ),
                ));
                continue;
            }
            Outcome::Failed(why) => {
                report.push(Diagnostic::error(
                    "verify.verifier-failed",
                    argv.join(" "),
                    format!("{alias} [{which}]: {why}"),
                ));
                continue;
            }
            Outcome::Answered(stdout) => stdout,
        };
        let extra = outside(&stdout, bundle);
        if !extra.is_empty() {
            report.push(Diagnostic::error(
                "verify.outside-bundle",
                repo.relative(bundle_path),
                format!(
                    "{alias}: the response to the bundle for [{which}] also answers {}, which \
                     that bundle did not carry; a verdict on evidence the verifier was not \
                     shown is not a verification. Nothing from this response was recorded",
                    extra.join(", ")
                ),
            ));
            continue;
        }
        std::fs::create_dir_all(&responses).map_err(|source| RepoError::Io {
            context: format!("could not create {responses}"),
            source,
        })?;
        let response_path = responses.join(format!("response-{}.toml", short(digest)));
        std::fs::write(&response_path, &stdout).map_err(|source| RepoError::Io {
            context: format!("could not write {response_path}"),
            source,
        })?;
        let ingested = crate::verify::ingest(repo, alias, &response_path)?;
        // The response file was the seam's input; the verifications it
        // produced are the records. Kept only when the ingest accepted it.
        if !ingested.is_ready() {
            let _ = std::fs::remove_file(&response_path);
        }
        for d in ingested.diagnostics {
            report.push(d);
        }
        for n in ingested.notes {
            report.note(n);
        }
    }
    Ok(report)
}

/// Where `war verify --run` keeps each verifier's whole response, under the
/// Warrant's `verifications/`: a subdirectory, so no reader of the
/// per-obligation records parses it as one.
pub const RESPONSES_DIR: &str = "responses";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plant_s_literal_headers_are_read_and_an_expanded_one_is_not() {
        let src = "echo \"== friction measurement (OW-WAR-0118) ==\"\n  echo '== second =='\necho \"== $X ==\"\necho \"not a header\"\nprintf '== no ==\\n'\n";
        assert_eq!(
            echoed_headers(src),
            vec!["== friction measurement (OW-WAR-0118) ==", "== second =="]
        );
    }

    #[test]
    fn an_excerpted_stream_keeps_a_section_far_from_its_tail() {
        let mut text = String::new();
        text.push_str("== ours ==\nok    the claim this Warrant rests on\n");
        for i in 0..5000 {
            text.push_str(&format!("ok    filler line {i}\n"));
        }
        text.push_str("1 passed, 0 failed\n");
        let s = Stream {
            bytes: Some(text.into_bytes()),
            recorded: None,
        };
        let heads = vec!["== ours ==".to_owned()];
        let tail_only = captured(&s, Carry::Head(8192), &heads, &[], &[]);
        assert!(
            !tail_only["text"]
                .as_str()
                .unwrap()
                .contains("the claim this Warrant")
        );
        let v = captured(&s, Carry::Excerpt(8192), &heads, &[], &[]);
        assert_eq!(v["truncated"], true);
        assert!(stream_text(&v).contains("the claim this Warrant rests on"));
        assert!(v["text"].as_str().unwrap().contains("1 passed, 0 failed"));
        // A section whose header matches no head is not offered.
        let none = captured(
            &s,
            Carry::Excerpt(8192),
            &["== theirs ==".to_owned()],
            &[],
            &[],
        );
        assert!(!stream_text(&none).contains("the claim this Warrant rests on"));
    }

    #[test]
    fn a_file_of_one_long_line_is_carried_in_windows() {
        let mut line = String::from("{\"revision\":\"1.1.0\",\"body\":\"");
        line.push_str(&"x".repeat(60_000));
        line.push_str("\",\"source_sha256\":\"abc123\"}");
        let terms = vec!["war compile".to_owned(), "source_sha256".to_owned()];
        let w = byte_windows(&line, 4096, &terms);
        let all: String = w.iter().map(|e| e.text.as_str()).collect();
        assert!(all.contains("\"revision\":\"1.1.0\""), "the head");
        assert!(
            all.contains("\"source_sha256\":\"abc123\"}"),
            "the tail and the term"
        );
        assert!(all.len() <= 4096);
        assert!(w.iter().all(|e| e.start_line == 1 && e.end_line == 1));
        // Multi-byte text is cut on char boundaries, never inside one.
        let wide = "é".repeat(10_000);
        for e in byte_windows(&wide, 1001, &[]) {
            assert!(e.text.chars().all(|c| c == 'é'));
        }
    }

    #[test]
    fn a_plant_s_check_lines_are_carried_under_any_section() {
        let src = "er_ok \"a clean receipt names its source\" \"$X\"\n\
                   wu_expect \"an act by PUT\" \"$(x)\" 405\n\
                   printf 'ok    %-34s %s\\n' \"the web UI holds no authority\" \"x\"\n\
                   er_fail \"$dynamic\" \"x\"\n";
        let names = check_names(src);
        assert_eq!(
            names,
            vec![
                "a clean receipt names its source",
                "an act by PUT",
                "the web UI holds no authority"
            ]
        );
        let mut text = String::from("== someone else's plant ==\n");
        for i in 0..3000 {
            text.push_str(&format!("ok    filler {i}\n"));
        }
        text.push_str("ok    a clean receipt names its source   contract, tree; seal recomputes\n");
        text.push_str("ok    a clean receipt names its source, again  not this one\n");
        for i in 0..3000 {
            text.push_str(&format!("ok    more filler {i}\n"));
        }
        let s = Stream {
            bytes: Some(text.into_bytes()),
            recorded: None,
        };
        let v = captured(&s, Carry::Excerpt(4096), &[], &names, &[]);
        let shown = stream_text(&v);
        assert!(shown.contains("a clean receipt names its source   contract"));
        assert!(
            !shown.contains("names its source, again"),
            "a longer name is another check"
        );
    }

    #[test]
    fn test_names_are_read_from_test_attributes_only() {
        let src = "fn helper() {}\n#[test]\nfn one_works() {}\n#[test]\n#[ignore]\nfn two_is_ignored() {}\nfn three() {}\n";
        assert_eq!(test_names(src), vec!["one_works", "two_is_ignored"]);
    }

    fn obligation(statement: &str, scope: &str, evidence: &str) -> RequestedObligation {
        RequestedObligation {
            id: "OBL-001".to_owned(),
            statement: statement.to_owned(),
            scope: scope.to_owned(),
            evidence: evidence.to_owned(),
        }
    }

    #[test]
    fn a_carried_receipt_does_not_depend_on_where_the_checkout_sits() {
        let at = |dir: &str| openwarrant_core::gate_run::GateReceipt {
            run_id: "GR-x".into(),
            gate_definition_digest: "sha256:aa".into(),
            gate_binding_digest: "unbound:x".into(),
            subject_digests: vec!["tree:abc".into()],
            fixture_digests: vec![],
            runner: "test".into(),
            runtime_environment: "test".into(),
            arguments: vec![],
            working_directory: dir.into(),
            started_at: "2026-09-02T00:00:00Z".into(),
            completed_at: "2026-09-02T00:00:01Z".into(),
            exit_result: "pass".into(),
            selected_test_count: 0,
            selected_test_manifest: vec![],
            raw_evidence_refs: vec![],
            stdout_ref: "o".into(),
            stderr_ref: "e".into(),
            resource_usage: "wall-clock only".into(),
            verdict: openwarrant_core::gate_run::Verdict::Pass,
            receipt_digest: "sha256:bb".into(),
        };
        // t-ade4: two scratch programs whose names differ by a digit.
        let a = carried_receipt(&at("/tmp/war-eval-1849790-0-638569984-code-01"));
        let b = carried_receipt(&at("/tmp/war-eval-999999-0-38569984-code-01"));
        assert_eq!(a, b);
        assert!(a.get("working_directory").is_none());
        // Everything else is carried.
        assert_eq!(a["receipt_digest"], "sha256:bb");
        assert_eq!(a["subject_digests"][0], "tree:abc");
    }

    #[test]
    fn an_obligation_names_paths_and_terms() {
        let o = obligation(
            "the parser refuses a duplicate",
            "`check.rs`'s drift decision, `crates/x/src/tui/`, conformance/plants.d/98-a.sh.",
            "`war check --json` reports `deliverable.superseded-by`; `gate://a.b@1.0.0`; `D-004`",
        );
        let (paths, terms) = named(&o);
        assert_eq!(
            paths,
            vec![
                "check.rs",
                "crates/x/src/tui/",
                "conformance/plants.d/98-a.sh",
                "D-004"
            ]
        );
        assert_eq!(
            terms,
            vec![
                "war check --json",
                "deliverable.superseded-by",
                "gate://a.b@1.0.0"
            ]
        );
    }

    #[test]
    fn water_filling_gives_small_items_whole_and_shares_the_rest() {
        assert_eq!(water_fill(&[10, 1000, 1000], 610), vec![10, 300, 300]);
        assert_eq!(water_fill(&[10, 20], 1000), vec![10, 20]);
        assert_eq!(water_fill(&[], 5), Vec::<usize>::new());
    }

    #[test]
    fn an_excerpt_keeps_the_head_and_the_lines_naming_a_term() {
        let text: String = (1..=200).map(|i| format!("line {i}\n")).collect();
        let mut p = Picker::new(&text, 400);
        p.offer(0, 3);
        p.offer_terms(&["line 150".to_owned()], 1, 1);
        let ex = p.excerpts();
        assert_eq!(ex[0].start_line, 1);
        assert_eq!(ex[0].end_line, 3);
        assert_eq!(ex[1].start_line, 149);
        assert_eq!(ex[1].end_line, 151);
        assert_eq!(ex[1].text, "line 149\nline 150\nline 151\n");
    }

    #[test]
    fn an_allowance_is_never_exceeded() {
        let text = "aaaa\nbbbb\ncccc\n";
        let mut p = Picker::new(text, 11);
        p.offer(0, 3);
        let used: usize = p.excerpts().iter().map(|e| e.text.len()).sum();
        assert_eq!(used, 10);
    }
}
