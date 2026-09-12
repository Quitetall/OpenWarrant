// SPDX-License-Identifier: Apache-2.0

//! `war commit` (OW-WAR-0069): the commit message, written from the records.
//!
//! A commit that lands a signed act is describable without prose: the records
//! that appeared say which act, which Warrant, which actor, and over which
//! digest. Asking the signer to also write that sentence is asking them to
//! restate the tree. So this reads `git status`, classifies the records it
//! finds, and prints a Conventional Commits message naming exactly them.
//!
//! It is a draft, not a claim: `--write` stages nothing and commits nothing
//! unless asked, and a human may always pass their own `-m`. What the tool
//! will not do is invent a subject for changes it cannot classify, because a
//! commit message that describes the wrong change is worse than none.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// What kind of change a path represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Authorization,
    Resolution,
    Correction,
    SasRevision,
    Response,
    Question,
    Projection,
    Journal,
    Warrant,
    Code,
    Docs,
    Conformance,
    Other,
}

impl Kind {
    const fn word(self) -> &'static str {
        match self {
            Self::Authorization => "authorization",
            Self::Resolution => "resolution",
            Self::Correction => "correction",
            Self::SasRevision => "SAS revision",
            Self::Response => "signed response",
            Self::Question => "question",
            Self::Projection => "projection",
            Self::Journal => "journal",
            Self::Warrant => "Warrant record",
            Self::Code => "code",
            Self::Docs => "documentation",
            Self::Conformance => "plant",
            Self::Other => "file",
        }
    }

    /// The Conventional Commits type a change of this kind earns.
    const fn conventional(self) -> &'static str {
        match self {
            Self::Authorization | Self::Resolution | Self::Correction | Self::Response => "warrant",
            Self::SasRevision => "sas",
            Self::Question => "questions",
            Self::Projection | Self::Journal | Self::Warrant => "records",
            Self::Code => "feat",
            Self::Docs => "docs",
            Self::Conformance => "conformance",
            Self::Other => "chore",
        }
    }
}

fn classify(path: &str) -> Kind {
    let p = path;
    if p.ends_with("authorization.toml") {
        return Kind::Authorization;
    }
    if p.ends_with("resolution.toml") {
        return Kind::Resolution;
    }
    if p.contains("/corrections/") {
        return Kind::Correction;
    }
    if p.starts_with("docs/sas/revisions/") {
        return Kind::SasRevision;
    }
    if p.starts_with("docs/authority/responses/") {
        return Kind::Response;
    }
    if p.contains("/questions/") {
        return Kind::Question;
    }
    if p.contains("/generated/") || p.ends_with("WAR.json") {
        return Kind::Projection;
    }
    if p.ends_with("journal.jsonl") {
        return Kind::Journal;
    }
    if p.starts_with("docs/warrants/") {
        return Kind::Warrant;
    }
    if p.starts_with("conformance/") {
        return Kind::Conformance;
    }
    if p.ends_with(".rs") || p.ends_with("Cargo.toml") || p.ends_with(".sh") {
        return Kind::Code;
    }
    if p.starts_with("docs/") || p.ends_with(".md") {
        return Kind::Docs;
    }
    Kind::Other
}

fn warrant_of(path: &str) -> Option<String> {
    let rest = path.strip_prefix("docs/warrants/")?;
    let alias = rest.split('/').next()?;
    alias.contains("-WAR-").then(|| alias.to_owned())
}

/// The changed paths, staged and unstaged, as git reports them.
fn changed(repo: &Repository) -> Result<Vec<String>, RepoError> {
    let out = std::process::Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=all"])
        .current_dir(&repo.root)
        .output()
        .map_err(|source| RepoError::Io {
            context: "could not run git status".to_owned(),
            source,
        })?;
    if !out.status.success() {
        return Err(RepoError::Message(
            "git status failed; is this a repository?".to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.get(3..).map(str::trim))
        .map(|p| p.rsplit(" -> ").next().unwrap_or(p).to_owned())
        .filter(|p| !p.is_empty())
        .collect())
}

/// The message, drafted from what changed.
pub fn message(repo: &Repository) -> Result<String, RepoError> {
    let paths = changed(repo)?;
    if paths.is_empty() {
        return Err(RepoError::Message(
            "nothing has changed, so there is nothing to describe".to_owned(),
        ));
    }
    let mut by_kind: BTreeMap<Kind, Vec<String>> = BTreeMap::new();
    let mut warrants: BTreeSet<String> = BTreeSet::new();
    for p in &paths {
        by_kind.entry(classify(p)).or_default().push(p.clone());
        if let Some(a) = warrant_of(p) {
            warrants.insert(a);
        }
    }
    // The subject takes the most significant kind present: a signature beats a
    // projection, because the projection is a consequence of it.
    let lead = *by_kind.keys().next().expect("non-empty");
    let scope = match warrants.len() {
        0 => String::new(),
        1 => format!("({})", warrants.iter().next().expect("one")),
        n if n <= 3 => format!(
            "({})",
            warrants.iter().cloned().collect::<Vec<_>>().join(", ")
        ),
        n => format!("({n} Warrants)"),
    };
    let mut s = String::new();
    let _ = writeln!(
        s,
        "{}{scope}: {}",
        lead.conventional(),
        subject(lead, &by_kind, &warrants)
    );
    let _ = writeln!(s);
    for (kind, files) in &by_kind {
        let shown: Vec<&str> = files.iter().take(4).map(String::as_str).collect();
        let _ = writeln!(
            s,
            "- {} ({}): {}{}",
            kind.word(),
            files.len(),
            shown.join(", "),
            if files.len() > shown.len() {
                format!(", and {} more", files.len() - shown.len())
            } else {
                String::new()
            }
        );
    }
    if by_kind.contains_key(&Kind::Response) || by_kind.contains_key(&Kind::Authorization) {
        let _ = writeln!(
            s,
            "\nEvery signature here is the signer's own ssh confirmation; the tool drafted \
             the records and confirmed nothing."
        );
    }
    Ok(s)
}

fn subject(
    lead: Kind,
    by_kind: &BTreeMap<Kind, Vec<String>>,
    warrants: &BTreeSet<String>,
) -> String {
    let n = |k: Kind| by_kind.get(&k).map_or(0, Vec::len);
    match lead {
        Kind::Authorization => format!("{} authorization(s) recorded", n(Kind::Authorization)),
        Kind::Resolution => format!("{} resolution(s) recorded", n(Kind::Resolution)),
        Kind::Correction => format!(
            "{} correction(s) on delivered artifacts",
            n(Kind::Correction)
        ),
        Kind::SasRevision => "a SAS revision moved".to_owned(),
        Kind::Response => format!("{} signed response(s) ingested", n(Kind::Response)),
        Kind::Question => format!("{} question record(s)", n(Kind::Question)),
        Kind::Projection => "recompiled projections".to_owned(),
        Kind::Journal => "journal events appended".to_owned(),
        Kind::Warrant => {
            if warrants.len() == 1 {
                "Warrant records edited".to_owned()
            } else {
                format!("records edited across {} Warrants", warrants.len())
            }
        }
        Kind::Code => format!("{} source file(s) changed", n(Kind::Code)),
        Kind::Docs => format!("{} document(s) changed", n(Kind::Docs)),
        Kind::Conformance => format!("{} conformance file(s) changed", n(Kind::Conformance)),
        Kind::Other => "files changed".to_owned(),
    }
}

/// `war commit`: print the drafted message, or commit with it.
pub fn run(repo: &Repository, write: bool) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let text = message(repo)?;
    if !write {
        println!("{text}");
        report.push(Diagnostic::pass(
            "commit.drafted",
            "drafted from the changed records; nothing staged, nothing committed",
        ));
        report.note(
            "`war commit --write` stages everything and commits with this message.".to_owned(),
        );
        return Ok(report);
    }
    let add = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&repo.root)
        .status()
        .map_err(|source| RepoError::Io {
            context: "could not run git add".to_owned(),
            source,
        })?;
    if !add.success() {
        return Err(RepoError::Message("git add failed".to_owned()));
    }
    let mut child = std::process::Command::new("git")
        .args(["commit", "-q", "-F", "-"])
        .current_dir(&repo.root)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|source| RepoError::Io {
            context: "could not run git commit".to_owned(),
            source,
        })?;
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().expect("piped");
        stdin
            .write_all(text.as_bytes())
            .map_err(|source| RepoError::Io {
                context: "could not write the message".to_owned(),
                source,
            })?;
    }
    let status = child.wait().map_err(|source| RepoError::Io {
        context: "could not wait for git commit".to_owned(),
        source,
    })?;
    if status.success() {
        report.push(Diagnostic::pass(
            "commit.written",
            text.lines().next().unwrap_or("committed").to_owned(),
        ));
    } else {
        report.push(Diagnostic::error(
            "commit.failed",
            String::new(),
            format!("git commit exited {status}"),
        ));
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_classify_by_what_they_record() {
        assert_eq!(
            classify("docs/warrants/OW-WAR-0064/authorization.toml"),
            Kind::Authorization
        );
        assert_eq!(
            classify("docs/warrants/OW-WAR-0005/corrections/D-001-1.toml"),
            Kind::Correction
        );
        assert_eq!(classify("docs/sas/revisions/1.0.0.toml"), Kind::SasRevision);
        assert_eq!(
            classify("docs/authority/responses/OW-WAR-0064.response.toml"),
            Kind::Response
        );
        assert_eq!(
            classify("docs/warrants/generated/CORPUS_STATUS.json"),
            Kind::Projection
        );
        assert_eq!(classify("crates/openwarrant-cli/src/sign.rs"), Kind::Code);
        // A plant is a plant before it is a shell script: the battery is the
        // thing a reader of the commit cares about.
        assert_eq!(
            classify("conformance/plants.d/91-questions.sh"),
            Kind::Conformance
        );
        assert_eq!(classify("docs/SKILLS.md"), Kind::Docs);
    }

    #[test]
    fn a_signature_leads_the_subject_over_its_consequences() {
        // A commit carrying an authorization AND the projections it moved is
        // about the authorization; the projections are downstream.
        let mut by: BTreeMap<Kind, Vec<String>> = BTreeMap::new();
        by.insert(
            Kind::Authorization,
            vec!["docs/warrants/OW-WAR-0064/authorization.toml".to_owned()],
        );
        by.insert(
            Kind::Projection,
            vec!["docs/warrants/generated/CORPUS_STATUS.json".to_owned()],
        );
        let lead = *by.keys().next().unwrap();
        assert_eq!(lead, Kind::Authorization);
        assert_eq!(lead.conventional(), "warrant");
        assert!(subject(lead, &by, &BTreeSet::new()).contains("authorization"));
    }

    #[test]
    fn a_warrant_alias_becomes_the_scope() {
        assert_eq!(
            warrant_of("docs/warrants/OW-WAR-0042/manifest.toml").as_deref(),
            Some("OW-WAR-0042")
        );
        assert_eq!(warrant_of("docs/warrants/generated/CORPUS_STATUS.md"), None);
        assert_eq!(warrant_of("crates/openwarrant-cli/src/main.rs"), None);
    }
}
