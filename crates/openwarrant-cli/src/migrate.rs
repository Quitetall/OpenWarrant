// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war migrate` — import a legacy ADR corpus (SAS §96), discharging OW-WAR-0043.
//!
//! # What this command may not do
//!
//! §96.3 is the sentence the whole subcommand is arranged around: a textual gate
//! command becomes `legacy_declared_unqualified`, and a legacy `Complete` line
//! with no admissible evidence stays a historical claim. So the import has no
//! path that produces a resolution. Not a discouraged path — none. The count of
//! promoted resolutions is reported anyway, because OW-WAR-0043 OBL-003 is
//! written as a number so that bending it requires changing something checkable.
//!
//! `--attempt-promotion` exists ONLY so that refusal is observable from outside
//! the binary. It is a negative control, in the same sense the gate
//! qualification machinery already demands negative controls: a rule nobody can
//! watch failing is a rule nobody can show works.
//!
//! # Bytes first
//!
//! §96.1 keeps every body. The digest is recomputed here and compared by
//! [`MigratedAdr::validate`], so "0 failures" is a statement about bytes rather
//! than about non-emptiness. Reading the corpus out of a working tree is the
//! caller's choice; OBL-001 is discharged by naming the commit and by the
//! artifact being reproducible, which `--verify-against` checks.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::sha256_hex;
use openwarrant_core::migration::{
    HistoricalClaim, LegacyDeclaredUnqualified, LegacyMapping, MigratedAdr, map_legacy_heading,
};
use serde::{Deserialize, Serialize};

/// Why an import stopped.
///
/// Hand-rolled `Display` rather than a `thiserror` derive: this crate does not
/// depend on it, and `repo::RepoError` beside it is written the same way.
#[derive(Debug)]
pub enum MigrateError {
    UnpinnedCommit {
        found: String,
    },
    CorpusNotADirectory {
        path: Utf8PathBuf,
    },
    CorpusEmpty {
        path: Utf8PathBuf,
    },
    Read {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    Write {
        path: Utf8PathBuf,
        source: std::io::Error,
    },
    Serialise(serde_json::Error),
    Migration(openwarrant_core::migration::MigrationError),
    NotReproducible {
        commit: String,
        path: Utf8PathBuf,
    },
}

impl fmt::Display for MigrateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnpinnedCommit { found } => write!(
                f,
                "commit {found:?} is not a full 40-character lowercase Git object id. \
                 OBL-001 requires ONE NAMED, FROZEN commit: a branch or an \
                 abbreviation names a moving target, and an import of a moving target \
                 is an import of nothing in particular"
            ),
            Self::CorpusNotADirectory { path } => {
                write!(f, "corpus {path:?} is not a directory")
            }
            Self::CorpusEmpty { path } => write!(
                f,
                "corpus {path:?} contains no ADR files matching NNNN-*.md. An empty \
                 import is reported as an error rather than as a clean run of zero \
                 ADRs, which would satisfy every count while importing nothing"
            ),
            Self::Read { path, source } => write!(f, "reading {path:?}: {source}"),
            Self::Write { path, source } => write!(f, "writing {path:?}: {source}"),
            Self::Serialise(source) => write!(f, "serialising the import artifact: {source}"),
            Self::Migration(source) => write!(f, "{source}"),
            Self::NotReproducible { commit, path } => write!(
                f,
                "the import artifact is not reproducible: re-running at {commit} \
                 produced different bytes from {path:?}. OBL-001's evidence is \
                 \"a re-run at that SHA producing byte-identical output\", so a \
                 differing re-run is the obligation failing, not a formatting detail"
            ),
        }
    }
}

impl std::error::Error for MigrateError {}

impl From<serde_json::Error> for MigrateError {
    fn from(source: serde_json::Error) -> Self {
        Self::Serialise(source)
    }
}

impl From<openwarrant_core::migration::MigrationError> for MigrateError {
    fn from(source: openwarrant_core::migration::MigrationError) -> Self {
        Self::Migration(source)
    }
}

/// One imported ADR plus the counts that make the obligations checkable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportArtifact {
    /// OBL-001 — the one named, frozen commit this import ran against.
    pub commit_sha: String,
    pub corpus_relative_root: String,
    pub adr_count: usize,
    /// OBL-003 — expected ZERO, and the reason this is a number.
    pub promoted_resolutions: usize,
    pub historical_claims: usize,
    pub legacy_declared_unqualified_gates: usize,
    /// OBL-002 — ADRs whose recorded digest is not the digest of their body.
    pub preservation_failures: Vec<String>,
    /// Deliverable 5 — element -> how many ADRs used it, sorted.
    pub unmapped_elements: BTreeMap<String, usize>,
    pub adrs: Vec<MigratedAdr>,
}

/// The digest §96.1 is checked with: a plain content hash of the body bytes.
///
/// Not domain-separated. Domain separation distinguishes structured payloads
/// that could otherwise collide; this is a hash of one file's bytes, and the
/// value's whole job is to be recomputable by anyone holding the same bytes.
#[must_use]
pub fn body_digest(body: &str) -> String {
    format!("sha256:{}", sha256_hex(body.as_bytes()))
}

fn is_adr_filename(name: &str) -> bool {
    let bytes = name.as_bytes();
    name.ends_with(".md")
        && bytes.len() > 5
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
}

/// Split YAML frontmatter from the body, returning `(frontmatter, body)`.
///
/// The body is everything after the closing fence, VERBATIM. §96.1 is a claim
/// about those bytes, so nothing here trims, re-wraps, or normalises them.
fn split_frontmatter(text: &str) -> (&str, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("", text);
    };
    match rest.find("\n---\n") {
        Some(at) => (&rest[..at], &rest[at + 5..]),
        None => ("", text),
    }
}

fn headings(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|line| line.strip_prefix("## "))
        .map(|h| h.trim().to_owned())
        .collect()
}

/// Tokens that make a string a COMMAND rather than a sentence about one.
///
/// LamQuant's `adr_model._RUNNABLE_GATE_RE`, verbatim. Kept as data so the two
/// implementations can be diffed by eye.
const RUNNABLE_LEADERS: [&str; 13] = [
    "python3", "python", "cargo", "bash", "sh", "pytest", "make", "just", "blut", "graphify",
    "npm", "pnpm", "uv",
];

/// Placeholders `adr_model._append_if_concrete` rejects outright.
const PLACEHOLDERS: [&str; 6] = [
    "command",
    "none",
    "n/a",
    "pending",
    "see implementation plan",
    "unmeasured",
];

/// Does this look like something that could be executed?
fn is_runnable(value: &str) -> bool {
    let is_boundary = |c: char| c.is_whitespace() || ";&|()".contains(c);
    for (at, _) in value.char_indices() {
        let before_ok = at == 0 || value[..at].chars().next_back().is_some_and(is_boundary);
        if !before_ok {
            continue;
        }
        let rest = &value[at..];
        // `./script` — a relative path is its own leader.
        if rest.starts_with("./") && rest.len() > 2 {
            return true;
        }
        for leader in RUNNABLE_LEADERS {
            // `get`, not a slice: an index into this corpus lands inside an
            // em-dash often enough that `rest[..n]` panicked on the first real
            // ADR. Byte length is not character position, and a gate that
            // crashes on the input it exists to read is worse than one that
            // reports nothing.
            let Some(head) = rest.get(..leader.len()) else {
                continue;
            };
            if head.eq_ignore_ascii_case(leader)
                && rest[leader.len()..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
            {
                return true;
            }
        }
    }
    false
}

/// Push `value` unless it is a template placeholder or plain prose.
fn append_if_concrete(value: &str, out: &mut Vec<String>) {
    let normalised: String = value
        .chars()
        .filter(|c| !"`*_".contains(*c))
        .collect::<String>()
        .trim()
        .to_lowercase();
    let placeholder = normalised.is_empty()
        || (value.contains('<') && value.contains('>'))
        || PLACEHOLDERS.contains(&normalised.as_str())
        || normalised.starts_with("pending ")
        || normalised.starts_with("todo")
        || normalised.starts_with("tbd")
        || !is_runnable(value);
    if !placeholder && !out.iter().any(|existing| existing == value) {
        out.push(value.to_owned());
    }
}

/// The value side of a `gate_cmd:` line, if the line declares one.
///
/// Accepts the shapes the house template grew: an optional list bullet, and
/// optional backtick/emphasis marks around the key.
fn gate_cmd_value(line: &str) -> Option<&str> {
    let mut rest = line.trim_start();
    for bullet in ["- ", "* "] {
        if let Some(stripped) = rest.strip_prefix(bullet) {
            rest = stripped.trim_start();
            break;
        }
    }
    rest = rest.trim_start_matches(['`', '*', '_']);
    // `get`, not a slice. "gate_cmd".len() is 8, and a bullet beginning with an
    // em-dash puts byte 8 inside that character — the same panic this file hit
    // twice, in two functions, on the first real ADR. Byte length is not
    // character position and this corpus is full of em-dashes.
    let head = rest.get(.."gate_cmd".len())?;
    if !head.eq_ignore_ascii_case("gate_cmd") {
        return None;
    }
    let after = rest["gate_cmd".len()..].trim_start_matches(['`', '*', '_']);
    let after = after.trim_start_matches([' ', '\t']);
    let value = after.strip_prefix(':')?;
    Some(value.trim_start_matches(['`', '*', '_']).trim())
}

/// Every concrete `gate_cmd` an ADR declares (§96.2's `gate_cmd` row).
///
/// This deliberately reproduces LamQuant's `adr_model._gate_commands` rather
/// than scanning for the string `gate_cmd:`. LamQuant ADR 0186 makes the point
/// and this function is the answer to it: a raw line scan reports 165 over that
/// corpus, `adr_model` reports 101 over 68 ADRs, and the difference is prose and
/// template mentions that are not commands. An importer using the raw number
/// would disagree with every figure the source repository has published, which
/// is indistinguishable from having imported different documents.
///
/// Three behaviours carry that difference, all of them load-bearing:
///
/// 1. **A wrapped backtick span is one command.** ADR 0054 wraps a `cargo test`
///    line mid-span; taking the line alone truncates it at `--test` with no
///    value, which then looks like a broken gate rather than a wrapped one.
/// 2. **Each backtick span is its OWN leg.** A line reading `` `a` + `b` + and
///    the default build green`` is prose containing two commands and one
///    requirement that is not a command. Joining them with `&&` would silently
///    drop the third.
/// 3. **Prose is not a command.** A value with no runnable leader is a sentence.
fn gate_commands(body: &str) -> Vec<String> {
    let lines: Vec<&str> = body.lines().collect();
    let mut commands = vec![];
    for (index, line) in lines.iter().enumerate() {
        let Some(value) = gate_cmd_value(line) else {
            continue;
        };
        let mut raw = value.to_owned();
        // An odd backtick count means the span wrapped; fold following lines in
        // until they balance, the way markdown renders it.
        if raw.matches('`').count() % 2 == 1 {
            for next in &lines[index + 1..] {
                raw.push(' ');
                raw.push_str(next.trim());
                if raw.matches('`').count() % 2 == 0 {
                    break;
                }
            }
            raw = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        let spans: Vec<&str> = raw
            .split('`')
            .skip(1)
            .step_by(2)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if spans.is_empty() {
            append_if_concrete(raw.trim_matches(['`', '*', '_', ' ']), &mut commands);
        } else {
            for span in spans {
                append_if_concrete(span, &mut commands);
            }
        }
    }
    commands
}

/// The Completion section's text, if the ADR has one.
///
/// Returned whole rather than parsed for a verdict. §96.3 makes this a CLAIM,
/// and a claim is preserved, not interpreted — deciding here whether it "looks
/// complete" is the laundering the section exists to prevent.
fn completion_text(body: &str) -> Option<String> {
    let mut lines = body.lines();
    let mut collected: Vec<&str> = vec![];
    let mut inside = false;
    for line in lines.by_ref() {
        if let Some(heading) = line.strip_prefix("## ") {
            if inside {
                break;
            }
            inside = heading.trim_start().starts_with("Completion");
            continue;
        }
        if inside {
            collected.push(line);
        }
    }
    let text = collected.join("\n").trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Frontmatter keys §96.2 maps directly, in the table's own spelling.
fn frontmatter_elements(frontmatter: &str) -> Vec<&'static str> {
    let mut found = vec![];
    for line in frontmatter.lines() {
        let key = line.split(':').next().unwrap_or("").trim();
        match key {
            "status" => found.push("status"),
            "supersedes" | "superseded_by" | "amends" | "amended_by" | "extends" => {
                found.push("supersedes/amends/extends");
            }
            _ => {}
        }
    }
    found.sort_unstable();
    found.dedup();
    found
}

/// Import the corpus. Reads only; writing the artifact is the caller's step.
pub fn import(
    corpus: &Utf8Path,
    commit_sha: &str,
    attempt_promotion: bool,
) -> Result<ImportArtifact, MigrateError> {
    if commit_sha.len() != 40 || !commit_sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(MigrateError::UnpinnedCommit {
            found: commit_sha.to_owned(),
        });
    }
    if commit_sha.chars().any(|c| c.is_ascii_uppercase()) {
        return Err(MigrateError::UnpinnedCommit {
            found: commit_sha.to_owned(),
        });
    }
    if !corpus.is_dir() {
        return Err(MigrateError::CorpusNotADirectory {
            path: corpus.to_owned(),
        });
    }

    // Sorted: the artifact must be byte-identical on a re-run (OBL-001), and
    // directory order is not.
    let mut files: Vec<Utf8PathBuf> = vec![];
    let entries = fs::read_dir(corpus).map_err(|source| MigrateError::Read {
        path: corpus.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| MigrateError::Read {
            path: corpus.to_owned(),
            source,
        })?;
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.file_name().is_some_and(is_adr_filename) {
            files.push(path);
        }
    }
    files.sort();
    if files.is_empty() {
        return Err(MigrateError::CorpusEmpty {
            path: corpus.to_owned(),
        });
    }

    let mut artifact = ImportArtifact {
        commit_sha: commit_sha.to_owned(),
        corpus_relative_root: corpus.file_name().unwrap_or("decisions").to_owned(),
        adr_count: files.len(),
        promoted_resolutions: 0,
        historical_claims: 0,
        legacy_declared_unqualified_gates: 0,
        preservation_failures: vec![],
        unmapped_elements: BTreeMap::new(),
        adrs: Vec::with_capacity(files.len()),
    };

    for path in &files {
        let text = fs::read_to_string(path).map_err(|source| MigrateError::Read {
            path: path.clone(),
            source,
        })?;
        let source = path.file_name().unwrap_or("?").to_owned();
        let (frontmatter, body) = split_frontmatter(&text);

        let mut mapped: BTreeMap<String, LegacyMapping> = BTreeMap::new();
        let mut unmapped: Vec<String> = vec![];
        for element in frontmatter_elements(frontmatter) {
            if let Some(m) = map_legacy_heading(element) {
                mapped.insert(element.to_owned(), m);
            }
        }
        for heading in headings(body) {
            match map_legacy_heading(&heading) {
                Some(m) => {
                    mapped.insert(heading, m);
                }
                None => unmapped.push(heading),
            }
        }
        unmapped.sort();
        unmapped.dedup();
        for element in &unmapped {
            *artifact
                .unmapped_elements
                .entry(element.clone())
                .or_insert(0) += 1;
        }

        let gates: Vec<LegacyDeclaredUnqualified> = gate_commands(body)
            .iter()
            .map(|c| LegacyDeclaredUnqualified::from_command(c))
            .collect();
        artifact.legacy_declared_unqualified_gates += gates.len();

        let mut claims = vec![];
        if let Some(text) = completion_text(body) {
            let claim = HistoricalClaim::from_completion_line(&source, &text);
            // The negative control. Promotion is refused because there is no
            // admissible evidence, which is the whole of §96.3 in one call.
            if attempt_promotion {
                claim.require_evidence_to_promote()?;
                artifact.promoted_resolutions += 1;
            }
            claims.push(claim);
        }
        artifact.historical_claims += claims.len();

        let migrated = MigratedAdr {
            source: source.clone(),
            preserved_body: body.to_owned(),
            preserved_body_digest: body_digest(body),
            mapped_elements: mapped,
            unmapped_elements: unmapped,
            legacy_gates: gates,
            historical_claims: claims,
        };
        // OBL-002, per ADR, over the whole bounded corpus rather than a sample.
        if migrated.validate(body_digest).is_err() {
            artifact.preservation_failures.push(source);
        }
        artifact.adrs.push(migrated);
    }

    Ok(artifact)
}

/// Render the artifact deterministically.
pub fn render(artifact: &ImportArtifact) -> Result<String, MigrateError> {
    let mut json = serde_json::to_string_pretty(artifact)?;
    json.push('\n');
    Ok(json)
}

/// Write the artifact, or compare against an existing one (OBL-001's re-run).
pub fn write_or_verify(
    artifact: &ImportArtifact,
    out: &Utf8Path,
    verify_only: bool,
) -> Result<(), MigrateError> {
    let rendered = render(artifact)?;
    if verify_only {
        let existing = fs::read_to_string(out).map_err(|source| MigrateError::Read {
            path: out.to_owned(),
            source,
        })?;
        if existing != rendered {
            return Err(MigrateError::NotReproducible {
                commit: artifact.commit_sha.clone(),
                path: out.to_owned(),
            });
        }
        return Ok(());
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|source| MigrateError::Write {
            path: out.to_owned(),
            source,
        })?;
    }
    fs::write(out, rendered).map_err(|source| MigrateError::Write {
        path: out.to_owned(),
        source,
    })
}

/// Print the report OW-WAR-0043's deliverables are read from.
pub fn print(artifact: &ImportArtifact) {
    println!("commit                        {}", artifact.commit_sha);
    println!("ADRs imported                 {}", artifact.adr_count);
    println!(
        "historical claims             {}",
        artifact.historical_claims
    );
    println!(
        "promoted resolutions          {}   (OBL-003 expects 0)",
        artifact.promoted_resolutions
    );
    println!(
        "legacy_declared_unqualified   {}",
        artifact.legacy_declared_unqualified_gates
    );
    println!(
        "preservation failures         {}   (OBL-002 expects 0)",
        artifact.preservation_failures.len()
    );
    for source in &artifact.preservation_failures {
        println!("  ✗ PreservedBodyDigestMismatch {source}");
    }

    let distinct = artifact.unmapped_elements.len();
    let occurrences: usize = artifact.unmapped_elements.values().sum();
    println!("unmapped elements             {distinct} distinct / {occurrences} occurrence(s)");
    // Deliverable 5, batched per AM-001: recurring first, and the singleton
    // count stated rather than 212 lines nobody reads.
    let mut recurring: Vec<(&String, &usize)> = artifact
        .unmapped_elements
        .iter()
        .filter(|(_, n)| **n >= 3)
        .collect();
    recurring.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (element, n) in &recurring {
        println!("  batch  {n:>4}  {element}");
    }
    let singletons = artifact
        .unmapped_elements
        .values()
        .filter(|n| **n == 1)
        .count();
    println!("  {singletons} singleton(s) need one disposition each (T2)");
}

/// Whether the import satisfies the obligations that are countable.
#[must_use]
pub fn obligations_met(artifact: &ImportArtifact) -> bool {
    artifact.promoted_resolutions == 0 && artifact.preservation_failures.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch corpus that removes itself.
    ///
    /// Hand-rolled rather than a `tempdir` dev-dependency: this workspace keeps
    /// its dependency surface deliberately small, and one Drop is cheaper than
    /// arguing about a crate.
    struct Scratch(Utf8PathBuf);

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn corpus(files: &[(&str, &str)]) -> (Scratch, Utf8PathBuf) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let unique = format!(
            "war-migrate-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir().join(unique))
            .expect("temp dir path is utf8");
        fs::create_dir_all(&root).expect("create scratch");
        for (name, text) in files {
            fs::write(root.join(name), text).expect("write");
        }
        (Scratch(root.clone()), root)
    }

    const SHA: &str = "ee950b8054756eb981e936b27c8d3e2a7c144296";

    fn adr(body: &str) -> String {
        format!("---\nstatus: accepted\n---\n{body}")
    }

    /// The panic this file hit on the FIRST real ADR, twice, in two functions.
    ///
    /// `"gate_cmd".len()` is 8 and `"graphify".len()` is 8, and a bullet opening
    /// with an em-dash puts byte 8 inside that character. Byte length is not
    /// character position. A gate that crashes on the input it exists to read is
    /// worse than one that reports nothing, so this stays.
    #[test]
    fn a_multibyte_line_does_not_panic_the_extractor() {
        for line in [
            "— a dash-led bullet",
            "- — nested dash",
            "…",
            "— gate_cmd: `cargo test`",
            "café",
        ] {
            let _ = gate_cmd_value(line);
            let _ = is_runnable(line);
        }
        // An EM-DASH is not a markdown bullet, and adr_model's `[-*]` does not
        // accept it either — verified against the Python directly, which returns
        // `[]` for this line and `['cargo test -p x']` for the hyphen form. The
        // first version of this test asserted the opposite and failed, which is
        // the test doing its job on its author.
        assert!(gate_commands("— gate_cmd: `cargo test -p x`\n").is_empty());
        assert_eq!(
            gate_commands("- gate_cmd: `cargo test -p x`\n"),
            vec!["cargo test -p x".to_owned()]
        );
    }

    /// Behaviour 1: a backtick span that wraps is ONE command, not a truncation.
    /// ADR 0054 wraps a `cargo test` line; the line alone stops at `--test`.
    #[test]
    fn a_wrapped_backtick_span_is_folded_into_one_command() {
        let body = "- gate_cmd: `cargo test -p lamquant-optimum --features encode --test\n  dual_format_battery`\n";
        assert_eq!(
            gate_commands(body),
            vec![
                "cargo test -p lamquant-optimum --features encode --test dual_format_battery"
                    .to_owned()
            ]
        );
    }

    /// Behaviour 2: each marked span is its own leg. Joining them with `&&`
    /// would silently drop the trailing requirement that is not a command.
    #[test]
    fn each_backtick_span_is_its_own_leg() {
        let body = "- gate_cmd: `cargo test -p blut` + `cargo clippy` + default build green.\n";
        assert_eq!(
            gate_commands(body),
            vec!["cargo test -p blut".to_owned(), "cargo clippy".to_owned()]
        );
    }

    /// Behaviour 3: prose is not a command, and neither is a placeholder.
    #[test]
    fn prose_and_placeholders_are_not_commands() {
        for body in [
            "- gate_cmd: see Implementation Plan\n",
            "- gate_cmd: pending\n",
            "- gate_cmd: TBD\n",
            "- gate_cmd: `<command>`\n",
            "- gate_cmd: the suite must stay green\n",
            "- gate_cmd: none\n",
            "- gate_cmd:\n",
        ] {
            assert!(
                gate_commands(body).is_empty(),
                "{body:?} produced a command"
            );
        }
    }

    /// A command repeated inside one ADR counts once, matching adr_model.
    #[test]
    fn a_repeated_command_is_deduplicated_within_one_adr() {
        let body = "- gate_cmd: `cargo test`\n- gate_cmd: `cargo test`\n";
        assert_eq!(gate_commands(body).len(), 1);
    }

    /// The house template's spellings of the key itself all parse.
    #[test]
    fn the_key_is_recognised_however_it_is_marked_up() {
        for line in [
            "- gate_cmd: `cargo test`",
            "- `gate_cmd:` `cargo test`",
            "- **gate_cmd:** `cargo test`",
            "* gate_cmd: `cargo test`",
            "gate_cmd: `cargo test`",
            "  - _gate_cmd_: `cargo test`",
        ] {
            assert_eq!(
                gate_commands(&format!("{line}\n")),
                vec!["cargo test".to_owned()],
                "{line:?} did not parse"
            );
        }
    }

    #[test]
    fn a_branch_name_is_not_a_frozen_commit() {
        let (_d, root) = corpus(&[("0001-x.md", &adr("## Decision\nx\n"))]);
        assert!(matches!(
            import(&root, "main", false),
            Err(MigrateError::UnpinnedCommit { .. })
        ));
    }

    #[test]
    fn an_abbreviated_sha_is_not_a_frozen_commit() {
        let (_d, root) = corpus(&[("0001-x.md", &adr("## Decision\nx\n"))]);
        assert!(matches!(
            import(&root, "ee950b8", false),
            Err(MigrateError::UnpinnedCommit { .. })
        ));
    }

    #[test]
    fn the_body_is_preserved_byte_for_byte_and_its_digest_recomputes() {
        let body = "## Decision\n\nKeep every byte,   including   this spacing.\n";
        let (_d, root) = corpus(&[("0001-x.md", &adr(body))]);
        let artifact = import(&root, SHA, false).expect("import");
        assert_eq!(artifact.adrs[0].preserved_body, body);
        assert_eq!(artifact.adrs[0].preserved_body_digest, body_digest(body));
        assert!(artifact.preservation_failures.is_empty());
        assert!(obligations_met(&artifact));
    }

    #[test]
    fn a_gate_command_imports_unqualified_and_cannot_be_promoted() {
        let (_d, root) = corpus(&[(
            "0001-x.md",
            &adr("## Implementation Plan\n- `gate_cmd:` `cargo test`\n"),
        )]);
        let artifact = import(&root, SHA, false).expect("import");
        assert_eq!(artifact.legacy_declared_unqualified_gates, 1);
        let gate = &artifact.adrs[0].legacy_gates[0];
        assert!(!gate.is_now_qualified());
        assert!(gate.promote().is_err(), "a migrated gate promoted");
    }

    /// OBL-003's negative control, at the library seam the binary calls.
    #[test]
    fn attempting_to_promote_a_completion_line_is_refused() {
        let (_d, root) = corpus(&[(
            "0001-x.md",
            &adr("## Completion / Resolution\n- **verdict:** `passed`\n"),
        )]);
        let clean = import(&root, SHA, false).expect("import");
        assert_eq!(clean.historical_claims, 1);
        assert_eq!(clean.promoted_resolutions, 0);

        let err = import(&root, SHA, true).expect_err("promotion must be refused");
        assert!(
            format!("{err}").contains("HISTORICAL"),
            "wrong refusal: {err}"
        );
    }

    #[test]
    fn am_001_normalisation_is_applied_to_real_heading_shapes() {
        let (_d, root) = corpus(&[(
            "0001-x.md",
            &adr("## Alternatives Considered\nx\n\n## Validation  *(ongoing)*\ny\n"),
        )]);
        let artifact = import(&root, SHA, false).expect("import");
        assert!(
            artifact.unmapped_elements.is_empty(),
            "unmapped: {:?}",
            artifact.unmapped_elements
        );
        assert_eq!(artifact.adrs[0].mapped_elements.len(), 3); // + status
    }

    #[test]
    fn an_invented_section_is_reported_unmapped_not_guessed() {
        let (_d, root) = corpus(&[("0001-x.md", &adr("## Reversal triggers\nx\n"))]);
        let artifact = import(&root, SHA, false).expect("import");
        assert_eq!(
            artifact.unmapped_elements.get("Reversal triggers"),
            Some(&1)
        );
    }

    /// OBL-001's evidence: a re-run at the same SHA is byte-identical.
    #[test]
    fn the_artifact_is_reproducible() {
        let (_d, root) = corpus(&[
            ("0002-b.md", &adr("## Decision\nb\n")),
            ("0001-a.md", &adr("## Decision\na\n")),
        ]);
        let first = render(&import(&root, SHA, false).expect("import")).expect("render");
        let second = render(&import(&root, SHA, false).expect("import")).expect("render");
        assert_eq!(first, second);
        // ...and the order is the sorted one, not the directory's.
        let a = first.find("0001-a.md").expect("0001 present");
        let b = first.find("0002-b.md").expect("0002 present");
        assert!(a < b, "ADRs are not in sorted order");
    }

    #[test]
    fn an_empty_corpus_is_refused_rather_than_reported_as_a_clean_import() {
        let (_d, root) = corpus(&[("README.md", "not an ADR\n")]);
        assert!(matches!(
            import(&root, SHA, false),
            Err(MigrateError::CorpusEmpty { .. })
        ));
    }
}
