// SPDX-License-Identifier: Apache-2.0
//! `war admin deliver <alias> [<D-id>...]` — declare a deliverable delivered (t-39dc).
//!
//! A Warrant's `deliverables.toml` names what the work produces (§37.1). It
//! says the work was DELIVERED when each entry is content addressed and
//! carries §37.2's provenance: the sha256 of the bytes as they are, how they
//! were made, and by what. Nothing wrote that but a hand or a throwaway
//! script, so on 2026-09-26 a wave of Warrants went to the blind verifier
//! with no delivery recorded and nothing it could establish from.
//!
//! This command records it, from the file on disk, and refuses:
//!
//! - a RESOLVED Warrant (`deliver.resolved`). Its §56.2 record binds
//!   `sha256(deliverables.toml)`; moving a byte of that file changes what
//!   was accepted, and `war sign correct` is the act for a delivered file that
//!   moved (AGENTS.md rule 4, OW-WAR-0064);
//! - a deliverable whose file is missing (`deliver.missing`): a digest of
//!   nothing is not a delivery;
//! - a target that is not a file in this repository (`deliver.not-a-file`).
//!
//! A path a LATER authorized Warrant governs (OW-ADR-0021) is not refused
//! and not recorded: its bytes are that Warrant's to record now, and this
//! Warrant's pin of it is historical — it verifies at this Warrant's own
//! resolution, not against today's file. It is named (`deliver.governed`,
//! with who governs it) and the Warrant's other deliverables are recorded.
//! Refusing the whole Warrant for it left every Warrant that shares a file
//! with a later one (`lib.rs`, `sign.rs`, most of the corpus) undeliverable.
//!
//! A compiled projection (`deliver.projection`) is named and not recorded:
//! `generated.drift` verifies it, and a digest of it cannot hold still.
//!
//! Any refusal refuses the whole command and nothing is written: a delivery
//! recorded for three of four files reads, later, as a Warrant that
//! delivered three.
//!
//! The file is edited as text, key by key, never round-tripped through the
//! parser: it is hand-written and hand-commented, and the comments are part
//! of the record (the same reason `war admin pins --refresh` edits the digest in
//! place). The result is parsed back and compared with what was meant before
//! it is written.
//!
//! Delivering does not stale an authorization: the set the authorizer signed
//! is `(id, target_ref)` pairs (`ownership::set_digest`), and neither moves.
//! It DOES move the tree every tree-bound receipt names — `deliverables.toml`
//! is source to the reuse rule — which is why `war evidence prepare` delivers first
//! and records evidence after.

use camino::Utf8Path;
use openwarrant_core::deliverable::{ArtifactProvenance, Deliverable, DeliverableKind};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// What the caller asks for beyond the alias.
#[derive(Debug, Default, Clone)]
pub struct Options<'a> {
    /// Deliverable ids; empty means every declared deliverable.
    pub ids: &'a [String],
    /// §37.2 `producer`. Defaults to the recorded one, then the repository's
    /// performer.
    pub producer: Option<&'a str>,
    /// §37.2 `creation_method`. Defaults to the recorded one, then `authored`.
    pub method: Option<&'a str>,
    /// Report what would be recorded and write nothing.
    pub dry_run: bool,
}

/// What one deliverable came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Its provenance is recorded now (or would be, under `dry_run`).
    Recorded,
    /// Already content addressed at the bytes on disk, with provenance.
    Current,
}

/// The per-deliverable outcomes of one run, for `war evidence prepare`.
#[derive(Debug, Default, Clone)]
pub struct Delivered {
    pub outcomes: Vec<(String, Outcome)>,
    /// Whether `deliverables.toml` was written.
    pub wrote: bool,
}

/// `war admin deliver`.
pub fn run(repo: &Repository, alias: &str, opts: &Options<'_>) -> Result<Report, RepoError> {
    run_with(repo, alias, opts).map(|(r, _)| r)
}

/// [`run`], with what happened to each deliverable.
pub fn run_with(
    repo: &Repository,
    alias: &str,
    opts: &Options<'_>,
) -> Result<(Report, Delivered), RepoError> {
    let mut report = Report::default();
    let mut delivered = Delivered::default();
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let manifest = dir.join("deliverables.toml");
    let file = repo.relative(&manifest);

    if crate::check::resolution_binds(repo, &one) {
        report.push(Diagnostic::error(
            "deliver.resolved",
            file,
            format!(
                "{alias} is resolved: its §56.2 record binds sha256(deliverables.toml), so no \
                 byte of it moves. A delivered file that changed since is `war sign correct {alias} \
                 <D-id>`; new work on it is a new Warrant"
            ),
        ));
        return Ok((report, delivered));
    }
    let set = repo.load_deliverables(&dir)?;
    for (path, why) in &set.failures {
        report.push(Diagnostic::error(
            "deliver.malformed",
            path.clone(),
            format!("{why} — nothing is recorded into a file that does not parse"),
        ));
    }
    if !set.failures.is_empty() {
        return Ok((report, delivered));
    }
    if set.records.is_empty() {
        report.push(Diagnostic::error(
            "deliver.none-declared",
            file,
            format!(
                "{alias} declares no deliverable: declare it in deliverables.toml (§37.1) first. \
                 Delivery is recorded against a declaration, never instead of one"
            ),
        ));
        return Ok((report, delivered));
    }
    for id in opts.ids {
        if !set.records.iter().any(|d| &d.id == id) {
            report.push(Diagnostic::error(
                "deliver.unknown-id",
                file.clone(),
                format!(
                    "{alias} declares no {id}; it declares {}",
                    set.records
                        .iter()
                        .map(|d| d.id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    if !report.is_ready() {
        return Ok((report, delivered));
    }

    let authorization = repo.load_authorization(&dir)?;
    let authorized = authorization.as_ref().filter(|a| {
        a.revision.state == openwarrant_core::RevisionState::Authorized
            && a.revision.authorization.is_some()
    });
    let authorized_at = authorized
        .and_then(|a| a.revision.authorization.as_ref())
        .map(|a| a.effective_time.clone());
    // §37.2's `contract_digest`: the revision the bytes were delivered under.
    // Before a human signs one there is none, and "unrecorded" says so; the
    // digest today's draft compiles to would claim an authority nobody gave.
    let contract_digest = authorized.map_or_else(
        || "unrecorded".to_owned(),
        |a| a.revision.contract_digest.clone(),
    );
    let ownership = crate::ownership::Ownership::index(repo)?;
    let projections = crate::gate_cmd::source::Exclusions::of(repo);
    let warrants = repo.config.paths.warrants.to_string();
    let tool = crate::build_identity::version_line();

    let mut edits: Vec<(String, ArtifactProvenance)> = Vec::new();
    for d in set
        .records
        .iter()
        .filter(|d| opts.ids.is_empty() || opts.ids.contains(&d.id))
    {
        let Some(path) = target_path(d) else {
            report.push(Diagnostic::error(
                "deliver.not-a-file",
                file.clone(),
                format!(
                    "{alias}: {} → {:?} is not a file in this repository (kind {}); `war admin deliver` \
                     digests files, and a {} is delivered by whatever records it",
                    d.id, d.target_ref, d.kind, d.kind
                ),
            ));
            continue;
        };
        let full = repo.root.join(path);
        let bytes = match std::fs::read(&full) {
            Ok(b) if full.is_file() => b,
            Ok(_) | Err(_) => {
                report.push(Diagnostic::error(
                    "deliver.missing",
                    file.clone(),
                    format!(
                        "{alias}: {} → {} is not there to deliver. A digest of nothing is not a \
                         delivery: write the file, or take it out of the declaration",
                        d.id, d.target_ref
                    ),
                ));
                continue;
            }
        };
        // A compiled projection (CURRENT.md, a Warrant's generated/ views) is
        // whatever its records compile to, and `generated.drift` is its check.
        // A pinned digest of it cannot hold still: the projections render each
        // deliverable's digest status, so compiling one moves its own bytes
        // and the next compile reads it drifted (t-88d2).
        if projections.excludes(path)
            && !crate::gate_cmd::source::is_evidence_record(path, &warrants)
        {
            report.push(Diagnostic::pass(
                "deliver.projection",
                format!(
                    "{alias}: {} → {} is a compiled projection; not recorded — its bytes are \
                     what the records compile to, and `war check --generated` is what verifies \
                     them",
                    d.id, d.target_ref
                ),
            ));
            continue;
        }
        // OW-ADR-0021: a path a Warrant authorized after this one governs is
        // that Warrant's now. Only an AUTHORIZED Warrant can be earlier than
        // another; a draft that declares a governed path becomes its owner
        // when it is signed, so its delivery is recorded as any other.
        if let Some(at) = authorized_at.as_deref()
            && let Some(owner) = ownership.newer_than(&d.target_ref, alias, Some(at))
        {
            report.push(Diagnostic::pass(
                "deliver.governed",
                format!(
                    "{alias}: {} → {} is governed by {}/{} (authorized {}), later than {alias} \
                     (authorized {at}); not recorded here — its bytes are that Warrant's to \
                     record, and this pin is historical (OW-ADR-0021). `war admin pins` shows who \
                     governs what",
                    d.id, d.target_ref, owner.alias, owner.deliverable_id, owner.authorized_at
                ),
            ));
            continue;
        }
        let digest = format!("sha256:{}", openwarrant_compiler::sha256_hex(&bytes));
        let recorded = d.provenance.clone().unwrap_or_default();
        if d.content_addressed
            && recorded.content_digest == digest
            && recorded.validate(&d.id).is_ok()
        {
            report.push(Diagnostic::pass(
                "deliver.current",
                format!(
                    "{alias}: {} → {} is delivered at {} already; nothing to record",
                    d.id,
                    d.target_ref,
                    short(&digest)
                ),
            ));
            delivered.outcomes.push((d.id.clone(), Outcome::Current));
            continue;
        }
        let keep = |recorded: &str, default: String| {
            if recorded.trim().is_empty() {
                default
            } else {
                recorded.to_owned()
            }
        };
        let provenance = ArtifactProvenance {
            producer: opts
                .producer
                .map(str::to_owned)
                .unwrap_or_else(|| keep(&recorded.producer, repo.performer())),
            producing_attempt: keep(&recorded.producing_attempt, "unrecorded".to_owned()),
            contract_digest: contract_digest.clone(),
            input_digests: recorded.input_digests.clone(),
            tool_or_runtime_identity: tool.clone(),
            creation_method: opts
                .method
                .map(str::to_owned)
                .unwrap_or_else(|| keep(&recorded.creation_method, "authored".to_owned())),
            content_digest: digest.clone(),
            media_type: keep(&recorded.media_type, media_type(path).to_owned()),
            classification: keep(&recorded.classification, "internal".to_owned()),
            retention: keep(&recorded.retention, "repository-lifetime".to_owned()),
            source_holder: keep(&recorded.source_holder, "git".to_owned()),
        };
        report.push(Diagnostic::pass(
            "deliver.recorded",
            format!(
                "{alias}: {} → {} delivered at {}{}",
                d.id,
                d.target_ref,
                short(&digest),
                if opts.dry_run { " (dry run)" } else { "" }
            ),
        ));
        delivered.outcomes.push((d.id.clone(), Outcome::Recorded));
        edits.push((d.id.clone(), provenance));
    }
    if !report.is_ready() {
        report.note(format!(
            "{alias}: nothing was written — a refusal refuses the whole delivery, so the file \
             never records part of one"
        ));
        delivered.outcomes.clear();
        return Ok((report, delivered));
    }
    if edits.is_empty() || opts.dry_run {
        return Ok((report, delivered));
    }

    let original = std::fs::read_to_string(&manifest).map_err(|source| RepoError::Io {
        context: format!("could not read {manifest}"),
        source,
    })?;
    let text = match apply(&original, &edits) {
        Ok(t) => t,
        Err(why) => {
            report.push(Diagnostic::error("deliver.unsupported-shape", file, why));
            delivered.outcomes.clear();
            return Ok((report, delivered));
        }
    };
    if let Err(why) = confirm(&text, &set.records, &edits) {
        report.push(Diagnostic::error(
            "deliver.rewrite-mismatch",
            file,
            format!("{why}; nothing was written"),
        ));
        delivered.outcomes.clear();
        return Ok((report, delivered));
    }
    // Only over the bytes this read: a declaration edited meanwhile is not
    // overwritten with a delivery recorded against the one before it.
    if let Err(refused) = crate::compile::atomic::write_if(
        &manifest,
        &text,
        &crate::compile::atomic::Prestate::of(original.as_bytes()),
    ) {
        report.push(refused.diagnostic());
        delivered.outcomes.clear();
        return Ok((report, delivered));
    }
    delivered.wrote = true;
    report.note(
        "Delivery records what the bytes ARE, not that anyone approved them. The independent \
         verifier reads them; a human resolves."
            .to_owned(),
    );
    Ok((report, delivered))
}

/// The repository-relative file a deliverable names, or `None` when it names
/// something else: a URI, an absolute path, or a path climbing out.
pub(crate) fn target_path(d: &Deliverable) -> Option<&str> {
    if d.kind == DeliverableKind::GitCommit {
        return None;
    }
    let t = d.target_ref.trim();
    if t.is_empty() || t.contains("://") || t.starts_with('/') {
        return None;
    }
    if Utf8Path::new(t)
        .components()
        .any(|c| !matches!(c, camino::Utf8Component::Normal(_)))
    {
        return None;
    }
    Some(t)
}

/// §37.2 `media_type` by extension, for a deliverable that records none.
fn media_type(path: &str) -> &'static str {
    match Utf8Path::new(path).extension().unwrap_or("") {
        "rs" => "text/x-rust",
        "md" => "text/markdown",
        "sh" => "text/x-shellscript",
        "toml" => "application/toml",
        "json" => "application/json",
        "yaml" | "yml" => "application/yaml",
        "txt" => "text/plain",
        "html" => "text/html",
        "js" => "text/javascript",
        "ts" => "text/x-typescript",
        "py" => "text/x-python",
        _ => "application/octet-stream",
    }
}

fn short(digest: &str) -> &str {
    let end = "sha256:".len() + 12;
    digest.get(..end).unwrap_or(digest)
}

/// A TOML basic string.
fn quoted(s: &str) -> String {
    toml::Value::String(s.to_owned()).to_string()
}

/// The provenance keys this command writes, in §37.2's order, rendered.
/// `input_digests` is never written: it is kept as recorded.
fn provenance_lines(p: &ArtifactProvenance) -> [(&'static str, String); 10] {
    [
        ("producer", quoted(&p.producer)),
        ("producing_attempt", quoted(&p.producing_attempt)),
        ("contract_digest", quoted(&p.contract_digest)),
        (
            "tool_or_runtime_identity",
            quoted(&p.tool_or_runtime_identity),
        ),
        ("creation_method", quoted(&p.creation_method)),
        ("content_digest", quoted(&p.content_digest)),
        ("media_type", quoted(&p.media_type)),
        ("classification", quoted(&p.classification)),
        ("retention", quoted(&p.retention)),
        ("source_holder", quoted(&p.source_holder)),
    ]
}

/// The key a `key = value` line sets, when it is one.
fn key_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with('#') || t.starts_with('[') {
        return None;
    }
    let (k, _) = t.split_once('=')?;
    let k = k.trim();
    (!k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.'))
    .then_some(k)
}

/// A line that says something: not blank, not a comment.
fn substantive(line: &str) -> bool {
    let t = line.trim();
    !t.is_empty() && !t.starts_with('#')
}

/// The trailing comment of `line`, with the space before it, if the value is
/// a plain string or bare word with no `#` inside it. Conservative: a value
/// that might contain `#` keeps no comment rather than a wrong one.
fn trailing_comment(line: &str) -> &str {
    let Some((_, value)) = line.split_once('=') else {
        return "";
    };
    let v = value.trim_start();
    let rest = if let Some(inner) = v.strip_prefix('"') {
        match inner.find('"') {
            Some(i) if !inner[..i].contains('\\') => &inner[i + 1..],
            _ => return "",
        }
    } else {
        v
    };
    match rest.find('#') {
        Some(i) => {
            let start = line.len() - rest.len() + i;
            let with_space = line[..start].trim_end().len();
            &line[with_space..]
        }
        None => "",
    }
}

/// `key = value`, keeping the indentation and trailing comment of `old`.
fn replaced(old: &str, key: &str, value: &str) -> String {
    let indent = &old[..old.len() - old.trim_start().len()];
    format!("{indent}{key} = {value}{}", trailing_comment(old))
}

/// Write `content_addressed = true` and each provenance into `text`, one
/// `[[deliverable]]` block at a time, key by key. Every line not rewritten is
/// kept byte for byte.
///
/// Refused, with the reason: a block this cannot find, and provenance in a
/// shape this does not edit (an inline table, dotted keys) — rewriting those
/// by guess is how a comment or a value is lost.
pub fn apply(text: &str, edits: &[(String, ArtifactProvenance)]) -> Result<String, String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let trailing_newline = text.ends_with('\n');
    for (id, provenance) in edits {
        let starts: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.trim() == "[[deliverable]]")
            .map(|(i, _)| i)
            .collect();
        let block = starts.iter().enumerate().find_map(|(n, &start)| {
            let end = starts.get(n + 1).copied().unwrap_or(lines.len());
            let main_end = (start + 1..end)
                .find(|&i| lines[i].trim_start().starts_with('['))
                .unwrap_or(end);
            let names_id = (start + 1..main_end).any(|i| {
                key_of(&lines[i]) == Some("id")
                    && lines[i]
                        .split_once('=')
                        .is_some_and(|(_, v)| v.trim().trim_matches('"') == id)
            });
            names_id.then_some((start, main_end, end))
        });
        let Some((start, main_end, end)) = block else {
            return Err(format!(
                "no [[deliverable]] block with id = {id:?} could be found as text"
            ));
        };
        if (start + 1..main_end).any(|i| {
            key_of(&lines[i]).is_some_and(|k| k == "provenance" || k.starts_with("provenance."))
        }) {
            return Err(format!(
                "{id} writes its provenance inline or as dotted keys; `war admin deliver` edits a \
                 [deliverable.provenance] table only"
            ));
        }
        let header = (main_end..end).find(|&i| lines[i].trim() == "[deliverable.provenance]");
        if (main_end..end).any(|i| {
            let t = lines[i].trim();
            t.starts_with("[deliverable.provenance.") || t == "[[deliverable.provenance]]"
        }) {
            return Err(format!(
                "{id}'s provenance has a nested table; `war admin deliver` edits a flat one only"
            ));
        }
        let rendered = provenance_lines(provenance);

        // The provenance table first (it is below the main table, so the
        // indices above it stay put).
        match header {
            Some(h) => {
                let t_end = (h + 1..end)
                    .find(|&i| lines[i].trim_start().starts_with('['))
                    .unwrap_or(end);
                let last = (h + 1..t_end)
                    .rev()
                    .find(|&i| substantive(&lines[i]))
                    .unwrap_or(h);
                // Replace in place first, then insert what is missing in one
                // splice, so no index moves under a lookup.
                let mut missing = Vec::new();
                for (key, value) in &rendered {
                    if let Some(i) = (h + 1..t_end).find(|&i| key_of(&lines[i]) == Some(key)) {
                        lines[i] = replaced(&lines[i], key, value);
                    } else {
                        missing.push(format!("{key} = {value}"));
                    }
                }
                lines.splice(last + 1..last + 1, missing);
            }
            None => {
                let last = (start + 1..main_end)
                    .rev()
                    .find(|&i| substantive(&lines[i]))
                    .unwrap_or(start);
                let mut table = vec![String::new(), "[deliverable.provenance]".to_owned()];
                table.extend(rendered.iter().map(|(k, v)| format!("{k} = {v}")));
                lines.splice(last + 1..last + 1, table);
            }
        }
        // Then `content_addressed` in the main table.
        match (start + 1..main_end).find(|&i| key_of(&lines[i]) == Some("content_addressed")) {
            Some(i) => lines[i] = replaced(&lines[i], "content_addressed", "true"),
            None => {
                let last = (start + 1..main_end)
                    .rev()
                    .find(|&i| substantive(&lines[i]))
                    .unwrap_or(start);
                lines.insert(last + 1, "content_addressed = true".to_owned());
            }
        }
    }
    let mut out = lines.join("\n");
    if trailing_newline {
        out.push('\n');
    }
    Ok(out)
}

/// Parse the rewritten text back and hold it to what was meant: the same
/// deliverables, in the same order, each unchanged except that the edited
/// ones are content addressed at the new provenance.
fn confirm(
    text: &str,
    before: &[Deliverable],
    edits: &[(String, ArtifactProvenance)],
) -> Result<(), String> {
    #[derive(serde::Deserialize)]
    struct File {
        #[serde(default)]
        deliverable: Vec<Deliverable>,
    }
    let after = toml::from_str::<File>(text)
        .map_err(|e| format!("the rewritten file does not parse: {e}"))?
        .deliverable;
    if after.len() != before.len() {
        return Err(format!(
            "the rewritten file declares {} deliverable(s), not {}",
            after.len(),
            before.len()
        ));
    }
    for (was, now) in before.iter().zip(&after) {
        let mut want = was.clone();
        if let Some((_, p)) = edits.iter().find(|(id, _)| id == &was.id) {
            want.content_addressed = true;
            let mut p = p.clone();
            p.input_digests = was
                .provenance
                .as_ref()
                .map(|r| r.input_digests.clone())
                .unwrap_or_default();
            want.provenance = Some(p);
        }
        if &want != now {
            return Err(format!(
                "{} did not read back as written (a shape this edit does not understand)",
                was.id
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prov(digest: &str) -> ArtifactProvenance {
        ArtifactProvenance {
            producer: "claude".into(),
            producing_attempt: "unrecorded".into(),
            contract_digest: "unrecorded".into(),
            input_digests: vec![],
            tool_or_runtime_identity: "war test".into(),
            creation_method: "authored".into(),
            content_digest: digest.into(),
            media_type: "text/plain".into(),
            classification: "internal".into(),
            retention: "repository-lifetime".into(),
            source_holder: "git".into(),
        }
    }

    const TWO: &str = r#"schema = "oh.war/deliverables/v1"

# The set, declared by the performer.

[[deliverable]]
id = "D-001"
title = "one"
kind = "file"
target_ref = "a.txt"
required = true
content_addressed = false   # not yet delivered
provenance_required = false
obligation_refs = ["OBL-001"]

# a comment that belongs to D-002
[[deliverable]]
id = "D-002"
title = "two"
kind = "document"
target_ref = "b.md"
required = true
content_addressed = true
provenance_required = true
obligation_refs = ["OBL-001"]

[deliverable.provenance]
producer = "codex"   # who wrote it
producing_attempt = "X/attempt-1"
contract_digest = "unrecorded"
tool_or_runtime_identity = "rustc 1.97.1"
creation_method = "generated"
content_digest = "sha256:old"
media_type = "text/markdown"
classification = "internal"
retention = "repository"
source_holder = "git"
"#;

    fn parse(text: &str) -> Vec<Deliverable> {
        #[derive(serde::Deserialize)]
        struct File {
            deliverable: Vec<Deliverable>,
        }
        toml::from_str::<File>(text).unwrap().deliverable
    }

    #[test]
    fn a_block_without_provenance_gains_the_table_and_keeps_every_comment() {
        let out = apply(TWO, &[("D-001".into(), prov("sha256:new"))]).unwrap();
        let d = parse(&out);
        assert!(d[0].content_addressed);
        assert_eq!(
            d[0].provenance.as_ref().unwrap().content_digest,
            "sha256:new"
        );
        assert_eq!(d[1], parse(TWO)[1], "D-002 is untouched");
        for comment in [
            "# The set, declared by the performer.",
            "# not yet delivered",
            "# a comment that belongs to D-002",
            "# who wrote it",
        ] {
            assert!(out.contains(comment), "{comment:?} survived");
        }
        // The new table sits inside D-001, before D-002's comment.
        let table = out.find("[deliverable.provenance]").unwrap();
        assert!(table < out.find("# a comment that belongs to D-002").unwrap());
        assert!(confirm(&out, &parse(TWO), &[("D-001".into(), prov("sha256:new"))]).is_ok());
    }

    #[test]
    fn an_existing_table_is_edited_key_by_key() {
        let mut p = prov("sha256:new");
        p.producer = "codex".into();
        let out = apply(TWO, &[("D-002".into(), p.clone())]).unwrap();
        let d = parse(&out);
        assert_eq!(d[1].provenance.as_ref().unwrap(), &p);
        assert!(out.contains("producer = \"codex\"   # who wrote it"));
        assert_eq!(out.matches("[deliverable.provenance]").count(), 1);
        assert_eq!(d[0], parse(TWO)[0], "D-001 is untouched");
    }

    #[test]
    fn an_inline_provenance_is_refused_not_guessed() {
        let text = "[[deliverable]]\nid = \"D-001\"\ntitle = \"t\"\nkind = \"file\"\n\
                    target_ref = \"a\"\nprovenance = { producer = \"x\" }\n";
        let err = apply(text, &[("D-001".into(), prov("sha256:x"))]).unwrap_err();
        assert!(err.contains("inline"), "{err}");
    }

    #[test]
    fn an_unknown_block_is_refused() {
        assert!(apply(TWO, &[("D-009".into(), prov("sha256:x"))]).is_err());
    }

    #[test]
    fn a_target_outside_the_repository_is_not_a_file() {
        let mut d = parse(TWO)[0].clone();
        for bad in ["../x", "/etc/passwd", "git://abc", "a/../../b", ""] {
            d.target_ref = bad.into();
            assert_eq!(target_path(&d), None, "{bad}");
        }
        d.target_ref = "docs/a.md".into();
        assert_eq!(target_path(&d), Some("docs/a.md"));
    }
}
