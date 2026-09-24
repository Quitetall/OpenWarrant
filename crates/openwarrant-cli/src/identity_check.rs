// SPDX-License-Identifier: Apache-2.0
//! Identity across the corpus — SAS §12.1, §12.2, §12.7; RQ-001, RQ-002.
//!
//! `openwarrant_core::identity` holds ONE record to UUIDv7, and the contract
//! digest makes an authorized Warrant's UUID immovable. Neither asks the
//! questions that need more than one record in hand, and OW-WAR-0119's probes
//! found every one of them unanswered: two Warrants could share a UUID, an atom
//! could name another Warrant, an alias passed for an identity in a `war://`
//! reference, and no ADR identity was checked at all. Each rule here answers
//! one of those, by name:
//!
//! | rule | claim | severity |
//! |---|---|---|
//! | `identity.duplicate-uuid` | one UUID names one Warrant, and one ADR | error |
//! | `identity.atom-mismatch` | an atom's `warrant_uuid` is its manifest's | error |
//! | `identity.changed` | a committed manifest UUID does not change (§12.2) | error |
//! | `identity.alias-ref` | a `war://` body is a UUID, never an alias (§12.7) | error; warning in ADR `governs` |
//! | `identity.adr-not-v7` | an ADR identity is a UUIDv7 (§12.1) | warning |
//!
//! The two warnings are warnings because this corpus holds records that break
//! them and §12.2 forbids the obvious repair: re-minting an identity to satisfy
//! §12.1 is the change §12.2 says never happens. They are listed one by one so
//! the owner's decision (OW-WAR-0119 U-001, U-002) is about a known set.

use std::collections::BTreeMap;
use std::str::FromStr;

use openwarrant_core::{AdrRecord, IdentityError, WarUuid};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{Loaded, Repository};

/// The canonical text of a UUID of ANY version, or `None` when `raw` is not a
/// UUID at all.
///
/// Comparing raw strings would let `01A0…` and `01a0…` be two identities.
/// Only v7 is an admissible WAR identity, but a v4 ADR UUID is still an
/// identity for the purpose of asking whether two records share it.
fn canonical(raw: &str) -> Option<String> {
    match WarUuid::from_str(raw.trim()) {
        Ok(uuid) => Some(uuid.to_string()),
        Err(IdentityError::UuidNotV7 { value, .. }) => Some(value.to_string()),
        Err(_) => None,
    }
}

/// Run every identity rule once for a `war check` run.
///
/// `corpus` is every Warrant, because a duplicate or an alias's UUID is only
/// visible with all of them in hand; `checked` is the Warrants this run was
/// asked about, and only their own findings are reported.
pub fn check(
    repo: &Repository,
    corpus: &[Loaded],
    checked: &[Loaded],
    adrs: &[AdrRecord],
    report: &mut Report,
) {
    let asked: Vec<String> = checked.iter().map(Loaded::alias).collect();
    duplicate_warrants(repo, corpus, &asked, report);
    duplicate_adrs(adrs, report);
    atom_mismatch(repo, checked, report);
    changed(repo, checked, report);
    alias_refs(repo, corpus, checked, adrs, report);
    adr_not_v7(adrs, report);
}

/// `identity.duplicate-uuid` over Warrant manifests.
///
/// A copied Warrant directory under a new alias is the realistic way in, and
/// every other rule passes it: each copy is well-formed on its own. Two records
/// with one identity make every `war://` reference, every pin and every journal
/// event that names it ambiguous.
fn duplicate_warrants(repo: &Repository, corpus: &[Loaded], asked: &[String], report: &mut Report) {
    let mut by_uuid: BTreeMap<String, Vec<&Loaded>> = BTreeMap::new();
    for one in corpus {
        // An invalid manifest is `manifest.invalid`'s finding, not this one's.
        let Some(validated) = &one.validated else {
            continue;
        };
        by_uuid
            .entry(validated.uuid.to_string())
            .or_default()
            .push(one);
    }
    let mut clean = true;
    for (uuid, holders) in &by_uuid {
        if holders.len() < 2 || !holders.iter().any(|h| asked.contains(&h.alias())) {
            continue;
        }
        clean = false;
        let named: Vec<String> = holders
            .iter()
            .map(|h| {
                format!(
                    "{} ({})",
                    h.alias(),
                    repo.relative(&h.dir.join("manifest.toml"))
                )
            })
            .collect();
        report.push(Diagnostic::error(
            "identity.duplicate-uuid",
            repo.relative(&holders[1].dir.join("manifest.toml")),
            format!(
                "UUID {uuid} names {} Warrants: {}. §12.1: the UUIDv7 IS the record's \
                 identity, so one UUID names one Warrant. A copied Warrant keeps its \
                 alias's convenience and none of its identity — the copy needs a fresh \
                 UUID from `war new`, not the original's",
                holders.len(),
                named.join(", ")
            ),
        ));
    }
    if clean {
        report.push(Diagnostic::pass(
            "identity.duplicate-uuid",
            format!("{} Warrant UUID(s), each naming one Warrant", by_uuid.len()),
        ));
    }
}

/// `identity.duplicate-uuid` over ADR atoms.
fn duplicate_adrs(adrs: &[AdrRecord], report: &mut Report) {
    if adrs.is_empty() {
        return;
    }
    let mut by_uuid: BTreeMap<String, Vec<&AdrRecord>> = BTreeMap::new();
    for adr in adrs {
        // Not a UUID at all is `identity.adr-not-v7`'s finding; the raw text
        // still groups, so two ADRs sharing the same malformed value collide.
        let key = canonical(&adr.uuid).unwrap_or_else(|| adr.uuid.trim().to_owned());
        by_uuid.entry(key).or_default().push(adr);
    }
    let mut clean = true;
    for (uuid, holders) in &by_uuid {
        if holders.len() < 2 {
            continue;
        }
        clean = false;
        let named: Vec<String> = holders
            .iter()
            .map(|a| format!("{} ({})", a.local_alias, a.source))
            .collect();
        report.push(Diagnostic::error(
            "identity.duplicate-uuid",
            holders[1].source.clone(),
            format!(
                "adr_uuid {uuid} names {} ADRs: {}. §12.1: every ADR has its own \
                 identity, so one UUID names one decision",
                holders.len(),
                named.join(", ")
            ),
        ));
    }
    if clean {
        report.push(Diagnostic::pass(
            "identity.duplicate-uuid",
            format!("{} ADR UUID(s), each naming one ADR", by_uuid.len()),
        ));
    }
}

/// `identity.atom-mismatch` — every Markdown atom names its own Warrant.
///
/// Until this rule only the legacy importer compared them. An atom copied in
/// from another Warrant carries that Warrant's UUID in its frontmatter, and
/// nothing said so: the atom compiled into this contract while claiming to
/// belong to another one.
fn atom_mismatch(repo: &Repository, checked: &[Loaded], report: &mut Report) {
    for one in checked {
        let (Some(validated), Some(basis)) = (&one.validated, &one.basis) else {
            continue;
        };
        let own = validated.uuid.to_string();
        let mut stray = Vec::new();
        let mut seen = 0usize;
        for atom in basis.atoms.iter().filter(|a| a.source.ends_with(".md")) {
            let text = String::from_utf8_lossy(&atom.bytes);
            // An atom that does not parse, or declares no `warrant_uuid`, is
            // another rule's finding; this one compares what IS declared.
            let Ok(fm) = openwarrant_core::frontmatter::parse(&text) else {
                continue;
            };
            let Some(declared) = fm.scalar("warrant_uuid") else {
                continue;
            };
            seen += 1;
            if canonical(declared).as_deref() != Some(own.as_str()) {
                stray.push((atom.source.clone(), declared.to_owned()));
            }
        }
        let alias = one.alias();
        if stray.is_empty() {
            report.push(Diagnostic::pass(
                "identity.atom-mismatch",
                format!("{alias}: {seen} atom(s) name this Warrant's UUID"),
            ));
        }
        for (source, declared) in stray {
            report.push(Diagnostic::error(
                "identity.atom-mismatch",
                repo.relative(&one.dir.join(&source)),
                format!(
                    "{alias}: atom {source} declares warrant_uuid {declared} but this \
                     Warrant is {own}. An atom belongs to the Warrant whose manifest lists \
                     it; one naming another Warrant is either copied in or misfiled"
                ),
            ));
        }
    }
}

/// What git can say about the committed baseline.
enum Baseline {
    /// Not a git repository, or git would not run: nothing is known.
    Unaskable(String),
    /// A repository with no commit yet — every Warrant is new.
    Empty,
    /// `HEAD` exists; the prefix places this repository's root inside the
    /// work tree, so `HEAD:<prefix><rel>` names a committed file.
    Head { prefix: String },
}

fn git(repo: &Repository, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    std::process::Command::new("git")
        .args(args)
        .current_dir(&repo.root)
        .output()
}

fn baseline(repo: &Repository) -> Baseline {
    let inside = match git(repo, &["rev-parse", "--show-prefix"]) {
        Err(e) => return Baseline::Unaskable(format!("git could not be run ({e})")),
        Ok(out) if !out.status.success() => {
            return Baseline::Unaskable(format!("{} is not inside a git repository", repo.root));
        }
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_owned(),
    };
    match git(repo, &["rev-parse", "--verify", "--quiet", "HEAD"]) {
        Ok(out) if out.status.success() => Baseline::Head { prefix: inside },
        Ok(_) => Baseline::Empty,
        Err(e) => Baseline::Unaskable(format!("git could not be run ({e})")),
    }
}

/// `identity.changed` — §12.2 held directly: a UUID committed at `HEAD` for a
/// manifest path is the UUID that path still holds.
///
/// `journal.wrong-warrant` and the contract digest already catch this for a
/// Warrant with a journal or an authorization. A draft with neither had no
/// check at all, and the journal is itself a file the same edit can rewrite.
/// The baseline is `HEAD`, as it is for `journal.rewritten`; a Warrant not in
/// `HEAD` is new and has nothing to compare, which is a pass. Where git
/// cannot answer the question is UNKNOWN (Law 15) — never a pass.
fn changed(repo: &Repository, checked: &[Loaded], report: &mut Report) {
    let prefix = match baseline(repo) {
        Baseline::Unaskable(why) => {
            report.push(Diagnostic::unknown(
                "identity.changed",
                repo.config.paths.warrants.clone(),
                format!(
                    "{why}, so no manifest UUID can be compared with a committed one. \
                     §12.2's \"never changes\" is unasked here, not established"
                ),
            ));
            return;
        }
        Baseline::Empty => {
            report.push(Diagnostic::pass(
                "identity.changed",
                "nothing is committed yet, so every manifest UUID is new",
            ));
            return;
        }
        Baseline::Head { prefix } => prefix,
    };
    let mut compared = 0usize;
    let mut new = 0usize;
    let mut moved = 0usize;
    for one in checked {
        let Some(validated) = &one.validated else {
            continue;
        };
        let alias = one.alias();
        let rel = repo.relative(&one.dir.join("manifest.toml"));
        let committed = match git(repo, &["show", &format!("HEAD:{prefix}{rel}")]) {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).into_owned(),
            // HEAD exists and does not hold this path: a new Warrant.
            Ok(_) => {
                new += 1;
                continue;
            }
            Err(e) => {
                report.push(Diagnostic::unknown(
                    "identity.changed",
                    rel,
                    format!("{alias}: git could not be run ({e}); the committed UUID is unasked"),
                ));
                continue;
            }
        };
        let before = toml::from_str::<toml::Table>(&committed)
            .ok()
            .and_then(|t| t.get("uuid").and_then(|v| v.as_str()).map(str::to_owned));
        let Some(before) = before else {
            report.push(Diagnostic::unknown(
                "identity.changed",
                rel,
                format!(
                    "{alias}: the manifest committed at HEAD does not parse or names no \
                     uuid, so there is no committed identity to compare with"
                ),
            ));
            continue;
        };
        let now = validated.uuid.to_string();
        compared += 1;
        if canonical(&before).as_deref() != Some(now.as_str()) {
            moved += 1;
            report.push(Diagnostic::error(
                "identity.changed",
                rel,
                format!(
                    "{alias}: the manifest UUID was {before} at HEAD and is now {now}. \
                     §12.2: the UUIDv7 is created at draft creation and never changes. \
                     A different identity is a different Warrant — `war new` makes one; \
                     editing this manifest does not"
                ),
            ));
        }
    }
    if moved == 0 {
        report.push(Diagnostic::pass(
            "identity.changed",
            format!("{compared} committed manifest UUID(s) unchanged since HEAD, {new} new"),
        ));
    }
}

/// The `war://` body when it is not a UUID, or `None` for a UUID reference or
/// another scheme.
fn alias_body(reference: &str) -> Option<&str> {
    let body = reference.trim().strip_prefix("war://")?;
    canonical(body).is_none().then_some(body)
}

/// `identity.alias-ref` — §12.7: machine references use the identity.
///
/// An alias is repository-scoped (RQ-002): `war://OW-WAR-0007` means one
/// Warrant here and another, or nothing, anywhere else. When the alias does
/// resolve here the finding names the UUID, so the repair is a copy.
fn alias_refs(
    repo: &Repository,
    corpus: &[Loaded],
    checked: &[Loaded],
    adrs: &[AdrRecord],
    report: &mut Report,
) {
    let uuid_of: BTreeMap<String, String> = corpus
        .iter()
        .filter_map(|one| {
            let v = one.validated.as_ref()?;
            Some((v.alias.to_string(), v.uuid.to_string()))
        })
        .collect();
    let remedy = |alias: &str| match uuid_of.get(alias) {
        Some(uuid) => format!("write war://{uuid} — the UUID {alias} has in this repository"),
        None => format!(
            "{alias} is not a Warrant in this repository, so no UUID can be offered; \
             name the Warrant by its UUID"
        ),
    };

    let mut refs = 0usize;
    let mut offending = 0usize;
    for one in checked {
        let Some(validated) = &one.validated else {
            continue;
        };
        let alias = one.alias();
        let file = repo.relative(&one.dir.join("manifest.toml"));
        let relations = validated
            .raw
            .parents
            .iter()
            .map(|p| ("parents", p.r#ref.as_str()))
            .chain(
                validated
                    .raw
                    .supersedes
                    .iter()
                    .map(|s| ("supersedes", s.r#ref.as_str())),
            );
        for (table, reference) in relations {
            refs += 1;
            let Some(body) = alias_body(reference) else {
                continue;
            };
            offending += 1;
            report.push(Diagnostic::error(
                "identity.alias-ref",
                file.clone(),
                format!(
                    "{alias}: [[{table}]] ref = \"{reference}\" names an alias where an \
                     identity is required (§12.7, RQ-002): {}",
                    remedy(body)
                ),
            ));
        }
    }
    if offending == 0 {
        report.push(Diagnostic::pass(
            "identity.alias-ref",
            format!("{refs} parent and supersedes reference(s), each by UUID"),
        ));
    }

    // A warning, not an error: six ADRs in this corpus already govern by
    // alias, and correcting them is an ADR amendment (OW-WAR-0119 U-002).
    for adr in adrs {
        for reference in &adr.governs {
            let Some(body) = alias_body(reference) else {
                continue;
            };
            report.push(Diagnostic::warn(
                "identity.alias-ref",
                adr.source.clone(),
                format!(
                    "{}: governs \"{reference}\" names an alias where an identity is \
                     expected (§12.7): {}",
                    adr.local_alias,
                    remedy(body)
                ),
            ));
        }
    }
}

/// `identity.adr-not-v7` — §12.1 asks a UUIDv7 of every ADR as of every WAR.
///
/// Reported, never repaired: §12.2 forbids changing an identity, so an ADR
/// minted v4 keeps it until the owner decides otherwise (OW-WAR-0119 U-001).
fn adr_not_v7(adrs: &[AdrRecord], report: &mut Report) {
    let mut v7 = 0usize;
    for adr in adrs {
        let why = match WarUuid::from_str(adr.uuid.trim()) {
            Ok(_) => {
                v7 += 1;
                continue;
            }
            Err(IdentityError::UuidNotV7 { found, .. }) => format!("is a version-{found} UUID"),
            Err(_) => "is not a UUID".to_owned(),
        };
        report.push(Diagnostic::warn(
            "identity.adr-not-v7",
            adr.source.clone(),
            format!(
                "{}: adr_uuid {} {why}; §12.1 requires a UUIDv7 for every ADR. Reported, \
                 not re-minted — §12.2 says an identity never changes",
                adr.local_alias, adr.uuid
            ),
        ));
    }
    if !adrs.is_empty() && v7 == adrs.len() {
        report.push(Diagnostic::pass(
            "identity.adr-not-v7",
            format!("all {v7} ADR UUID(s) are UUIDv7"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use camino::{Utf8Path, Utf8PathBuf};

    use super::*;
    use crate::diagnostic::Severity;

    /// A scaffolded program in its own directory, with or without git.
    struct Scratch(Utf8PathBuf);

    impl Scratch {
        fn new(tag: &str, with_git: bool) -> Self {
            let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
                .unwrap()
                .join(format!("war-identity-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            crate::init::run_program("Identity Plant", "ID", Some(root.clone()))
                .expect("scaffolds");
            let scratch = Self(root);
            // `war init` makes the directory a git repository (OW-WAR-0124,
            // OBL-007); the no-git case removes it, so it is really without.
            if !with_git {
                let _ = std::fs::remove_dir_all(scratch.0.join(".git"));
            }
            if with_git {
                scratch.git(&["init", "-q", "."]);
                scratch.git(&["add", "-A"]);
                scratch.git(&[
                    "-c",
                    "user.email=test@invalid",
                    "-c",
                    "user.name=test",
                    "commit",
                    "-qm",
                    "baseline",
                ]);
            }
            scratch
        }

        fn git(&self, args: &[&str]) {
            let ok = Command::new("git")
                .args(args)
                .current_dir(&self.0)
                .output()
                .expect("git runs")
                .status
                .success();
            assert!(ok, "git {args:?} failed in {}", self.0);
        }

        fn warrant(&self) -> Utf8PathBuf {
            self.0.join("docs/warrants/ID-WAR-0001")
        }

        fn edit(&self, path: &Utf8Path, from: &str, to: &str) {
            let text = std::fs::read_to_string(path).unwrap();
            assert!(text.contains(from), "{from} is not in {path}");
            std::fs::write(path, text.replacen(from, to, 1)).unwrap();
        }

        fn uuid(&self) -> String {
            let manifest = std::fs::read_to_string(self.warrant().join("manifest.toml")).unwrap();
            let table: toml::Table = toml::from_str(&manifest).unwrap();
            table["uuid"].as_str().unwrap().to_owned()
        }

        fn findings(&self, rule: &str) -> Vec<Diagnostic> {
            let repo = Repository::discover(Some(self.0.clone())).unwrap();
            crate::check::run(&repo, None, false)
                .unwrap()
                .diagnostics
                .into_iter()
                .filter(|d| d.rule == rule)
                .collect()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn worst(findings: &[Diagnostic]) -> Severity {
        findings
            .iter()
            .map(|d| d.severity)
            .max()
            .unwrap_or(Severity::Pass)
    }

    fn adr(alias: &str, uuid: &str, governs: &str) -> AdrRecord {
        AdrRecord::parse(
            &format!("docs/adr/atoms/{alias}.md"),
            &format!(
                "---\nadr_uuid: {uuid}\nlocal_alias: {alias}\nstatus: accepted\n\
                 governs:\n  - \"{governs}\"\n---\n\n# {alias}\n"
            ),
        )
        .expect("the fixture ADR parses")
    }

    const V7: &str = "01a0d04c-5e99-7663-90dd-3063ec23be35";
    const V7_OTHER: &str = "01a018db-19fc-72ba-87b3-c1bd1aec86a8";
    const V4: &str = "7c1f7f7e-4b1a-4d2a-9d3e-2b8f0a6c5e11";

    #[test]
    fn a_copied_warrant_shares_its_uuid_and_is_refused_by_name() {
        let s = Scratch::new("dup", false);
        assert_eq!(
            worst(&s.findings("identity.duplicate-uuid")),
            Severity::Pass
        );

        let copy = s.0.join("docs/warrants/ID-WAR-0002");
        std::fs::create_dir_all(&copy).unwrap();
        for entry in walk(&s.warrant()) {
            let to = copy.join(entry.strip_prefix(s.warrant()).unwrap());
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(&entry, &to).unwrap();
        }
        s.edit(
            &copy.join("manifest.toml"),
            "local_alias = \"ID-WAR-0001\"",
            "local_alias = \"ID-WAR-0002\"",
        );
        let found = s.findings("identity.duplicate-uuid");
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .unwrap_or_else(|| panic!("no duplicate refused: {found:?}"));
        assert!(error.message.contains("ID-WAR-0001"), "{}", error.message);
        assert!(error.message.contains("ID-WAR-0002"), "{}", error.message);
    }

    fn walk(dir: &Utf8Path) -> Vec<Utf8PathBuf> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = Utf8PathBuf::from_path_buf(entry.unwrap().path()).unwrap();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
        out
    }

    #[test]
    fn two_adrs_with_one_uuid_are_refused_and_distinct_ones_pass() {
        let mut report = Report::default();
        duplicate_adrs(
            &[adr("A-1", V7, "war://x"), adr("A-2", V7_OTHER, "war://x")],
            &mut report,
        );
        assert_eq!(report.worst(), Severity::Pass, "{report:?}");

        let mut report = Report::default();
        // Case differs; the identity does not.
        let upper = V7.to_uppercase();
        duplicate_adrs(
            &[adr("A-1", V7, "war://x"), adr("A-2", &upper, "war://x")],
            &mut report,
        );
        let error = report
            .diagnostics
            .iter()
            .find(|d| d.severity == Severity::Error && d.rule == "identity.duplicate-uuid")
            .unwrap_or_else(|| panic!("no duplicate refused: {report:?}"));
        assert!(error.message.contains("A-1") && error.message.contains("A-2"));
    }

    #[test]
    fn an_atom_naming_another_warrant_is_refused_and_restored_passes() {
        let s = Scratch::new("atom", false);
        assert_eq!(worst(&s.findings("identity.atom-mismatch")), Severity::Pass);

        let own = s.uuid();
        let intent = s.warrant().join("atoms/10-intent.md");
        s.edit(
            &intent,
            &format!("warrant_uuid: {own}"),
            &format!("warrant_uuid: {V7_OTHER}"),
        );
        let found = s.findings("identity.atom-mismatch");
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .unwrap_or_else(|| panic!("no mismatch refused: {found:?}"));
        assert!(
            error.message.contains("atoms/10-intent.md"),
            "{}",
            error.message
        );
        assert!(error.message.contains(V7_OTHER), "{}", error.message);

        s.edit(
            &intent,
            &format!("warrant_uuid: {V7_OTHER}"),
            &format!("warrant_uuid: {own}"),
        );
        assert_eq!(worst(&s.findings("identity.atom-mismatch")), Severity::Pass);
    }

    #[test]
    fn a_committed_uuid_that_changes_is_refused_naming_both() {
        let s = Scratch::new("changed", true);
        assert_eq!(worst(&s.findings("identity.changed")), Severity::Pass);

        let before = s.uuid();
        let after = WarUuid::mint().to_string();
        s.edit(
            &s.warrant().join("manifest.toml"),
            &format!("uuid = \"{before}\""),
            &format!("uuid = \"{after}\""),
        );
        let found = s.findings("identity.changed");
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .unwrap_or_else(|| panic!("no change refused: {found:?}"));
        assert!(error.message.contains(&before) && error.message.contains(&after));
    }

    #[test]
    fn an_uncommitted_warrant_has_nothing_to_compare_and_passes() {
        let s = Scratch::new("new", true);
        // Move the baseline Warrant out of HEAD: it is new as far as git knows.
        s.git(&["rm", "-rq", "--cached", "docs/warrants/ID-WAR-0001"]);
        s.git(&[
            "-c",
            "user.email=test@invalid",
            "-c",
            "user.name=test",
            "commit",
            "-qm",
            "untrack",
        ]);
        let found = s.findings("identity.changed");
        assert_eq!(worst(&found), Severity::Pass, "{found:?}");
        assert!(
            found.iter().any(|d| d.message.contains("1 new")),
            "{found:?}"
        );
    }

    #[test]
    fn with_no_git_repository_the_change_is_unknown_not_passed() {
        let s = Scratch::new("nogit", false);
        let found = s.findings("identity.changed");
        assert_eq!(worst(&found), Severity::Unknown, "{found:?}");
        assert!(
            !found.iter().any(|d| d.severity == Severity::Pass),
            "{found:?}"
        );
    }

    #[test]
    fn an_alias_is_refused_where_an_identity_is_required() {
        assert_eq!(alias_body("war://ID-WAR-0002"), Some("ID-WAR-0002"));
        assert_eq!(alias_body(&format!("war://{V7}")), None);
        // A v4 UUID is still a UUID: its version is another rule's business.
        assert_eq!(alias_body(&format!("war://{V4}")), None);
        assert_eq!(alias_body("sas://WAR-SAS-RQ-001"), None);

        let s = Scratch::new("alias", false);
        assert_eq!(worst(&s.findings("identity.alias-ref")), Severity::Pass);
        let own = s.uuid();
        let manifest = s.warrant().join("manifest.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(
            &manifest,
            format!("{text}\n[[supersedes]]\nref = \"war://ID-WAR-0001\"\nreason = \"a plant\"\n"),
        )
        .unwrap();
        let found = s.findings("identity.alias-ref");
        let error = found
            .iter()
            .find(|d| d.severity == Severity::Error)
            .unwrap_or_else(|| panic!("no alias refused: {found:?}"));
        assert!(
            error.message.contains(&format!("war://{own}")),
            "{}",
            error.message
        );
    }

    #[test]
    fn an_adr_governing_by_alias_is_a_warning_naming_the_uuid() {
        let s = Scratch::new("governs", false);
        let repo = Repository::discover(Some(s.0.clone())).unwrap();
        let corpus = vec![repo.load_warrant(&s.warrant()).unwrap()];
        let own = s.uuid();

        let mut report = Report::default();
        alias_refs(
            &repo,
            &corpus,
            &corpus,
            &[adr("A-1", V7, &format!("war://{own}"))],
            &mut report,
        );
        assert_eq!(report.worst(), Severity::Pass, "{report:?}");

        let mut report = Report::default();
        alias_refs(
            &repo,
            &corpus,
            &corpus,
            &[adr("A-1", V7, "war://ID-WAR-0001")],
            &mut report,
        );
        let warning = report
            .diagnostics
            .iter()
            .find(|d| d.rule == "identity.alias-ref" && d.severity == Severity::Warn)
            .unwrap_or_else(|| panic!("no warning: {report:?}"));
        assert!(warning.message.contains(&own), "{}", warning.message);
        assert_eq!(report.worst(), Severity::Warn, "never an error: {report:?}");
    }

    #[test]
    fn a_v4_adr_uuid_is_reported_one_by_one_and_a_v7_one_is_not() {
        let mut report = Report::default();
        adr_not_v7(&[adr("A-1", V7, "war://x")], &mut report);
        assert_eq!(report.worst(), Severity::Pass, "{report:?}");

        let mut report = Report::default();
        adr_not_v7(
            &[
                adr("A-1", V7, "war://x"),
                adr("A-2", V4, "war://x"),
                adr("A-3", V4, "war://x"),
            ],
            &mut report,
        );
        let warned: Vec<&str> = report
            .diagnostics
            .iter()
            .filter(|d| d.rule == "identity.adr-not-v7" && d.severity == Severity::Warn)
            .map(|d| d.file.as_deref().unwrap_or_default())
            .collect();
        assert_eq!(warned, ["docs/adr/atoms/A-2.md", "docs/adr/atoms/A-3.md"]);
        assert_eq!(report.worst(), Severity::Warn, "never an error: {report:?}");
    }
}
