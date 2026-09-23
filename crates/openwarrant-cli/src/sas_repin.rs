//! `war sas repin` — the sixty-five warnings become one act (OW-WAR-0112 M5).
//!
//! A Warrant authorized against SAS 1.0.0 keeps that Basis after 1.1.0 is
//! accepted (OW-ADR-0016): the contract is what was signed. `war check`
//! says so, once per Warrant, as `sas.pin-superseded`, and until now the
//! only way to clear one was to write an amendment by hand. This command
//! writes it — `amendments/AM-<next>.yaml` carrying `sas_revision` and
//! `predecessor_sas_revision`, in the exact shape `88-sas-repin.sh` already
//! exercises — for one Warrant or for every authorized, unresolved one whose
//! pin is behind the latest revision. The re-pin moves the contract digest,
//! so each Warrant then shows `authorize rev N+1 [AM-nnn]` in the queue and
//! a human re-authorizes it. That signature is the act; this is paperwork.
//!
//! Refused by name: a Warrant whose manifest implements a §106 row the
//! latest revision lacks (`sas.repin-unknown-requirement`), and — when
//! named — a resolved Warrant (`sas.repin-resolved`: the resolution binds
//! the contract as it was, §56.3). Under `--all`, resolved Warrants are
//! skipped, and after OW-WAR-0112 phase B they no longer warn at all.
//! All-or-nothing: one refusal in the set and nothing is written.
//!
//! Stated in the output rather than discovered later: a receipt bound to
//! the old contract digest reads `evidence.stale-binding` after the re-pin;
//! `war evidence record <alias>` mints new ones.

use camino::Utf8PathBuf;

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub alias: Option<String>,
    pub all: bool,
    pub reason: Option<String>,
    pub dry_run: bool,
}

/// One re-pin the command would write.
#[derive(Debug, Clone)]
struct Plan {
    alias: String,
    dir: Utf8PathBuf,
    id: String,
    from: String,
    to: String,
    authorizer: String,
}

pub fn run(repo: &Repository, opts: &Options) -> Result<Report, RepoError> {
    let mut report = Report::default();
    let Some(latest) = repo.latest_sas_revision()? else {
        report.push(Diagnostic::error(
            "sas.unrecorded",
            repo.relative(&repo.root.join(&repo.config.paths.sas)),
            "no SAS revision is recorded; `war sas propose <version>` first",
        ));
        return Ok(report);
    };
    let dirs: Vec<Utf8PathBuf> = match (&opts.alias, opts.all) {
        (Some(alias), _) => vec![repo.warrant_dir(alias)?],
        (None, true) => repo.warrant_dirs()?,
        (None, false) => {
            report.push(Diagnostic::error(
                "sas.repin-target",
                "-".to_owned(),
                "name a Warrant (`war sas repin <alias>`) or pass `--all`",
            ));
            return Ok(report);
        }
    };
    let named = opts.alias.is_some();

    let mut plans = Vec::new();
    let mut refused = 0usize;
    for dir in dirs {
        let alias = dir.file_name().unwrap_or("?").to_owned();
        let Some(auth) = repo.load_authorization(&dir)? else {
            if named {
                report.push(Diagnostic::error(
                    "sas.repin-unauthorized",
                    repo.relative(&dir.join("manifest.toml")),
                    format!("{alias}: not authorized; there is no pin to move — a draft compiles against the latest revision on its own"),
                ));
                refused += 1;
            }
            continue;
        };
        if repo.load_resolution(&dir)?.is_some() {
            if named {
                report.push(Diagnostic::error(
                    "sas.repin-resolved",
                    repo.relative(&dir.join("resolution.toml")),
                    format!("{alias}: resolved; the resolution binds the contract as it was and a re-pin would only read stale (§56.3). Nothing to re-pin"),
                ));
                refused += 1;
            }
            continue;
        }
        let amended = crate::repo::amendment_sas_revision(&dir).map(|(v, _)| v);
        let Some(pin) = amended.or_else(|| auth.sas_revision.clone()) else {
            if named {
                report.push(Diagnostic::warn(
                    "sas.repin-unpinned",
                    repo.relative(&dir.join("authorization.toml")),
                    format!("{alias}: authorized with no SAS pin at all (before OW-ADR-0016); nothing to move from"),
                ));
            }
            continue;
        };
        if pin == latest.version {
            if named {
                report.push(Diagnostic::pass(
                    "sas.repin-current",
                    format!("{alias}: already pinned to SAS {}", latest.version),
                ));
            }
            continue;
        }
        // Every row the manifest implements must exist in the target
        // revision; a requirement that vanished under it is a broken trace.
        let one = repo.load_warrant(&dir)?;
        let mut missing = Vec::new();
        if let Some(basis) = &one.basis {
            for i in &basis.manifest.implements {
                if let Ok(rq) = openwarrant_core::traceability::RequirementRef::parse(&i.r#ref)
                    && !latest.requirements.contains_key(&rq.canonical())
                {
                    missing.push(i.r#ref.clone());
                }
            }
        }
        if !missing.is_empty() {
            report.push(Diagnostic::error(
                "sas.repin-unknown-requirement",
                repo.relative(&dir.join("manifest.toml")),
                format!(
                    "{alias}: implements {} and SAS {}'s §106 has no such row — re-pinning would \
                     leave the Warrant implementing nothing; amend the manifest first",
                    missing.join(", "),
                    latest.version
                ),
            ));
            refused += 1;
            continue;
        }
        let authorizer = auth
            .revision
            .authorization
            .as_ref()
            .map(|a| a.authorizer.clone())
            .unwrap_or_default();
        plans.push(Plan {
            id: next_amendment_id(&dir),
            alias,
            dir,
            from: pin,
            to: latest.version.clone(),
            authorizer,
        });
    }

    if plans.is_empty() && refused == 0 {
        report.push(Diagnostic::pass(
            "sas.repin-nothing",
            format!(
                "every authorized, unresolved Warrant is pinned to SAS {} already",
                latest.version
            ),
        ));
        return Ok(report);
    }
    if refused > 0 {
        report.push(Diagnostic::error(
            "sas.repin-refused",
            "-".to_owned(),
            format!("{refused} refusal(s) above; nothing written (all-or-nothing)"),
        ));
        return Ok(report);
    }

    let today = crate::gate_cmd::receipt::rfc3339_from_secs(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs()),
    );
    let today = today.get(..10).unwrap_or(&today).to_owned();
    for p in &plans {
        let path = p.dir.join("amendments").join(format!("{}.yaml", p.id));
        let rel = repo.relative(&path);
        if opts.dry_run {
            report.push(Diagnostic::pass(
                "sas.repin-would-write",
                format!(
                    "{}: {rel} would re-pin SAS {} → {}; `war sign {} --ssh-sign` would then be revision N+1",
                    p.alias, p.from, p.to, p.alias
                ),
            ));
            continue;
        }
        std::fs::create_dir_all(p.dir.join("amendments")).map_err(|source| RepoError::Io {
            context: format!("could not create {}/amendments", p.dir),
            source,
        })?;
        std::fs::write(&path, render(p, opts.reason.as_deref(), &today)).map_err(|source| {
            RepoError::Io {
                context: format!("could not write {path}"),
                source,
            }
        })?;
        report.push(Diagnostic::pass(
            "sas.repin-written",
            format!(
                "{}: {rel} re-pins SAS {} → {}; a human re-authorizes: `war sign {} --ssh-sign` (revision N+1, [{}])",
                p.alias, p.from, p.to, p.alias, p.id
            ),
        ));
    }
    report.note(format!(
        "A re-pin moves the contract digest. Receipts bound to the old digest read \
         `evidence.stale-binding` until `war evidence record <alias>` mints new ones; the \
         amendment is a draft until the signature — `war sign --all --dry-run` shows what \
         each would meet. {} Warrant(s){}",
        plans.len(),
        if opts.dry_run {
            ", nothing written"
        } else {
            ""
        }
    ));
    Ok(report)
}

/// `AM-<n+1>` over the numbers already under `amendments/`.
fn next_amendment_id(dir: &camino::Utf8Path) -> String {
    let max = std::fs::read_dir(dir.join("amendments"))
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    let stem = name.strip_suffix(".yaml")?;
                    stem.strip_prefix("AM-")?.parse::<u64>().ok()
                })
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    format!("AM-{:03}", max + 1)
}

/// The amendment, in the shape `88-sas-repin.sh` plants and `check` reads
/// (`sas_revision:` line-wise). `basis_requirements` is the §28.5 element
/// that moved.
fn render(p: &Plan, reason: Option<&str>, today: &str) -> String {
    let reason = reason.map_or_else(
        || {
            format!(
                "Re-pin to SAS {} (from {}), written by `war sas repin` on {today}. The Basis \
                 names the revision in force; the contract's own text is unchanged. The \
                 authorizer adopts this record by signing revision N+1.",
                p.to, p.from
            )
        },
        str::to_owned,
    );
    format!(
        "schema: \"oh.war/amendment/v1\"\n\
         \n\
         id: {id:?}\n\
         band: \"manual_revision\"\n\
         reason: {reason:?}\n\
         governing_adr_or_policy: \"adr://OW-ADR-0016\"\n\
         artifact_admissibility: \"remain_admissible\"\n\
         restart_or_repair_instruction: \"Continue; the Basis names a newer SAS revision. Receipts bound to the old contract digest are re-minted with `war evidence record {alias}`.\"\n\
         re_preflight_required: \"false\"\n\
         authorizer: {authorizer:?}\n\
         effective_time: {today:?}\n\
         sas_revision: {to:?}\n\
         predecessor_sas_revision: {from:?}\n\
         \n\
         semantic_diff:\n  \
           - element: \"basis_requirements\"\n    \
             before: \"pinned to SAS {from}\"\n    \
             after: \"pinned to SAS {to}\"\n\
         \n\
         affected_stages: []\n\
         affected_milestones: []\n",
        id = p.id,
        alias = p.alias,
        authorizer = p.authorizer,
        to = p.to,
        from = p.from,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rendered_amendment_carries_the_pin_check_reads() {
        let p = Plan {
            alias: "OW-WAR-0047".into(),
            dir: Utf8PathBuf::from("x"),
            id: "AM-002".into(),
            from: "1.0.0".into(),
            to: "1.1.0".into(),
            authorizer: "Brian Lam".into(),
        };
        let text = render(&p, None, "2026-09-22");
        assert!(text.contains("sas_revision: \"1.1.0\"\n"));
        assert!(text.contains("predecessor_sas_revision: \"1.0.0\"\n"));
        assert!(text.contains("id: \"AM-002\"\n"));
        assert!(text.contains("authorizer: \"Brian Lam\"\n"));
        assert!(text.contains("element: \"basis_requirements\""));
        // The exact line-wise reader `check` uses finds the pin.
        let found = text
            .lines()
            .find_map(|l| l.strip_prefix("sas_revision:"))
            .map(|v| v.trim().trim_matches('"').to_owned());
        assert_eq!(found.as_deref(), Some("1.1.0"));
    }

    #[test]
    fn the_next_id_counts_past_the_highest_existing_one() {
        let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-repin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(next_amendment_id(&dir), "AM-001");
        std::fs::create_dir_all(dir.join("amendments")).unwrap();
        std::fs::write(dir.join("amendments/AM-003.yaml"), "x").unwrap();
        std::fs::write(dir.join("amendments/AM-001.yaml"), "x").unwrap();
        assert_eq!(next_amendment_id(&dir), "AM-004");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
