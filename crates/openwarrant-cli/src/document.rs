// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war document review [alias]` — the document work kind's gate (slice C4a).
//!
//! A research memo, a decision record or a written analysis is delivered as
//! Markdown, and "done" for it means four things a gate can check:
//!
//! 1. every Markdown deliverable exists at its recorded digest;
//! 2. every citation resolves — a relative path exists; a URL is recorded,
//!    never fetched;
//! 3. every obligation has an admissible `established` verification from a
//!    verifier who is not the performer;
//! 4. no placeholder is left in prose: `TODO`, `TBD`, `FIXME`, `lorem`,
//!    `<placeholder>` outside fenced code.
//!
//! Corpus-wide by default, so it runs as a registered gate with a static
//! argv; one alias narrows it. A Warrant with no Markdown deliverable is not
//! the document kind and is skipped, not failed.

use camino::Utf8Path;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

const PLACEHOLDERS: [&str; 5] = ["TODO", "TBD", "FIXME", "lorem ipsum", "<placeholder>"];

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `(line number, target)` of every Markdown link or image target.
///
/// The target ends at the first `)`: a target with nested parentheses
/// (`[x](url_(y))`) is read up to the inner one and reported as missing.
/// Wrap such a target in angle brackets `<…>` or avoid it; the simple
/// reader is kept so a citation is never silently accepted.
fn citations(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut rest = line;
        while let Some(k) = rest.find("](") {
            let after = &rest[k + 2..];
            let Some(end) = after.find(')') else { break };
            let target = after[..end]
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_owned();
            if !target.is_empty() {
                out.push((i + 1, target));
            }
            rest = &after[end + 1..];
        }
    }
    out
}

/// `(line number, word)` of every placeholder outside fenced code.
fn placeholders(text: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let lower = line.to_lowercase();
        for p in PLACEHOLDERS {
            let hit = if p.chars().all(|c| c.is_ascii_uppercase()) {
                // Uppercase markers are whole words: `TODO` yes, `mastodon` no.
                line.split(|c: char| !c.is_ascii_alphanumeric())
                    .any(|w| w == p)
            } else {
                lower.contains(&p.to_lowercase())
            };
            if hit {
                out.push((i + 1, p));
            }
        }
    }
    out
}

pub fn review(repo: &Repository, only: Option<&str>) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let dirs = match only {
        Some(alias) => vec![repo.warrant_dir(alias)?],
        None => repo.warrant_dirs()?,
    };
    let mut reviewed = 0usize;
    for dir in dirs {
        let Ok(loaded) = repo.load_warrant(&dir) else {
            continue;
        };
        let alias = loaded.alias();
        let Ok(set) = repo.load_deliverables(&dir) else {
            continue;
        };
        let docs: Vec<_> = set
            .records
            .iter()
            .filter(|d| d.target_ref.ends_with(".md"))
            .collect();
        if docs.is_empty() {
            continue;
        }
        // Corpus-wide, the gate reviews the Warrants that BIND to it (§43):
        // a code Warrant that happens to deliver a Markdown file cites
        // `war-check`, and this gate's four rules are not its contract. One
        // alias named on the command line is reviewed whatever it cites.
        if only.is_none() {
            let cites = loaded.basis.as_ref().is_some_and(|b| {
                b.atoms
                    .iter()
                    .filter(|a| a.role == "assurance")
                    .any(|a| String::from_utf8_lossy(&a.bytes).contains("gate://document.review@"))
            });
            if !cites {
                continue;
            }
        }
        reviewed += 1;
        let performer = repo.performer();
        for d in &docs {
            let path = repo.root.join(&d.target_ref);
            let rel = d.target_ref.clone();
            let Ok(bytes) = std::fs::read(&path) else {
                report.push(Diagnostic::error(
                    "document.undigested",
                    rel,
                    format!("{alias}: {} does not exist", d.id),
                ));
                continue;
            };
            match d
                .provenance
                .as_ref()
                .map(|p| p.content_digest.trim_start_matches("sha256:"))
            {
                Some(recorded) if recorded == sha256_hex(&bytes) => report.push(Diagnostic::pass(
                    "document.digested",
                    format!("{alias}: {} is at its recorded digest", d.id),
                )),
                Some(_) => report.push(Diagnostic::error(
                    "document.undigested",
                    rel.clone(),
                    format!(
                        "{alias}: {} has moved since its digest was recorded; re-record the \
                         deliverable, or for a resolved Warrant `war correct {alias} {}`",
                        d.id, d.id
                    ),
                )),
                None => report.push(Diagnostic::error(
                    "document.undigested",
                    rel.clone(),
                    format!("{alias}: {} records no content digest", d.id),
                )),
            }
            let text = String::from_utf8_lossy(&bytes);
            let mut urls = 0usize;
            for (line, target) in citations(&text) {
                if target.starts_with("http://") || target.starts_with("https://") {
                    urls += 1;
                    continue;
                }
                if target.starts_with('#') || target.starts_with("mailto:") {
                    continue;
                }
                let target_path = target.split('#').next().unwrap_or("");
                let base = Utf8Path::new(&rel).parent().unwrap_or(Utf8Path::new(""));
                let resolved = repo.root.join(base).join(target_path);
                if !resolved.exists() {
                    report.push(Diagnostic::error(
                        "document.citation-missing",
                        rel.clone(),
                        format!("{alias}: line {line} cites {target:?}, which does not exist"),
                    ));
                }
            }
            if urls > 0 {
                report.note(format!(
                    "{alias}: {} URL citation(s) in {} recorded, not fetched",
                    urls, d.target_ref
                ));
            }
            for (line, word) in placeholders(&text) {
                report.push(Diagnostic::error(
                    "document.placeholder",
                    rel.clone(),
                    format!("{alias}: line {line} still says {word:?}"),
                ));
            }
        }
        // Every obligation reviewed by someone else.
        let declared = crate::resolve::declared_obligations(&loaded);
        let verifications = repo
            .load_verifications(&dir)
            .map(|v| v.records)
            .unwrap_or_default();
        for id in &declared {
            let ok = verifications.iter().any(|v| {
                v.obligation == *id
                    && v.disposition.to_string() == "established"
                    && v.verifier.actor != performer
                    && v.verifier.actor != v.performer
            });
            if ok {
                report.push(Diagnostic::pass(
                    "document.reviewed",
                    format!("{alias}: {id} established by a verifier who is not the performer"),
                ));
            } else {
                report.push(Diagnostic::error(
                    "document.review-absent",
                    repo.relative(&dir.join("verifications")),
                    format!(
                        "{alias}: {id} has no `established` verification from someone other than \
                         the performer ({performer}); `war verify {alias} --bundle` and hand it off"
                    ),
                ));
            }
        }
    }
    report.push(Diagnostic::pass(
        "document.reviewed-warrants",
        format!("{reviewed} Warrant(s) with Markdown deliverables reviewed"),
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn citations_and_placeholders_are_read_outside_fences_only() {
        let text = "See [the SAS](../sas/x.md) and [web](https://example.org) and [a](#local).\n\n```\n[not](a-link) TODO\n```\n\nTBD here; mastodon is not a placeholder; Lorem Ipsum is.\n";
        let c = citations(text);
        assert_eq!(
            c,
            vec![
                (1, "../sas/x.md".to_owned()),
                (1, "https://example.org".to_owned()),
                (1, "#local".to_owned())
            ]
        );
        let p = placeholders(text);
        assert_eq!(p, vec![(7, "TBD"), (7, "lorem ipsum")]);
    }
}
