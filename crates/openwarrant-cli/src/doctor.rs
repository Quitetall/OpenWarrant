// SPDX-License-Identifier: Apache-2.0
//! Read-only aggregation. This is not an admission decision or an authority check.
use crate::{
    diagnostic::{Diagnostic, Report, Severity},
    repo::{RepoError, Repository},
};
use serde_json::{Value, json};

fn unavailable(report: &mut Report, component: &str, error: RepoError) {
    let rule = format!("doctor.{component}");
    let diagnostic = match error {
        RepoError::Io { .. } | RepoError::NotFound { .. } | RepoError::NonUtf8Path => {
            Diagnostic::unknown(rule, component, error.to_string())
        }
        _ => Diagnostic::error(rule, component, error.to_string()),
    };
    // Component identifiers are rules, not navigable file paths.
    report.push(Diagnostic {
        file: None,
        ..diagnostic
    });
}

pub fn run(
    root: Option<camino::Utf8PathBuf>,
    alias: Option<&str>,
    generated: bool,
) -> (Report, Value) {
    let mut report = Report::default();
    // `execution_authorized: false` stays for the readers that check it
    // (tests/doctor_cli.rs): it says doctor grants no Warrant execution. It
    // never meant ordinary work needs a grant, and `ordinary_work_needs_
    // authorization: false` says that in the same breath (M9).
    let mut result = json!({
        "schema": "oh.war/doctor/v1", "read_only": true,
        "admission": "UNKNOWN", "secure_authority": "UNKNOWN",
        "execution_authorized": false, "ordinary_work_needs_authorization": false,
        "remedies": [],
        "scope": "legacy-record-diagnostics-and-configuration"
    });
    // Which binary is answering, before anything it says can be read. An
    // operator debugging `war` needs this line first: one name on PATH can
    // hide a wrapper, a stale debug build, or three frozen snapshots.
    let install = crate::install::observe();
    report.diagnostics.extend(install.report().diagnostics);
    result["install"] = install.json();
    report.note("Not an approval: ordinary work needs nothing from this report (no Warrant, ticket or signature is required to edit, build or test). Doctor reads records, configuration and the signing setup, and changes nothing.");
    let repo = match Repository::discover(root) {
        Ok(repo) => {
            // M9: text newer than this binary, said before anything it reads.
            report.diagnostics.extend(crate::skew::findings(&repo.root));
            repo
        }
        Err(error) => {
            unavailable(&mut report, "repository", error);
            report.note("Inspect openwarrant.toml. For a new repository, see `war init --help`; doctor never initializes or changes it.");
            result["remedies"] = json!([{"component":"repository","argv":["war","init","--help"],"purpose":"Inspect setup options; repair malformed existing configuration at its source."}]);
            return (report, result);
        }
    };
    match crate::check::run(&repo, alias, generated) {
        Ok(check) => {
            report.diagnostics.extend(check.diagnostics);
            report.notes.extend(check.notes);
        }
        Err(error) => unavailable(&mut report, "records", error),
    }
    report.note("Legacy check diagnostics are preserved verbatim; some inherited file-read checks classify unavailable reads as errors. Doctor does not claim complete I/O classification coverage.");
    // Run independently: one malformed record must not suppress configuration
    // and authority observations. Do not invoke configured commands here.
    let roles = repo.root.join("docs/authority/roles.toml");
    match std::fs::metadata(&roles) {
        Ok(metadata) if !metadata.is_file() => report.push(Diagnostic::error(
            "doctor.authority-not-file", "docs/authority/roles.toml",
            "Authority register path is not a regular file; inspect the path before attempting any governed act.")),
        Ok(_) => match repo.load_authority_register() {
            Ok(_) => report.push(Diagnostic::warn("doctor.legacy-authority", "docs/authority/roles.toml",
                "Legacy role declarations parse. They do not establish a protected trust root. Inspect `war authority --help`.")),
            Err(error) => unavailable(&mut report, "authority", error),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => report.push(Diagnostic::warn(
            "doctor.authority-absent", "docs/authority/roles.toml",
            "No authority register (docs/authority/roles.toml). Only sign-off needs one; ordinary work does not. See `war authority --help`.")),
        Err(source) => unavailable(&mut report, "authority", RepoError::Io {
            context: format!("could not inspect {roles}"), source,
        }),
    }
    for (configured, rule, message) in [
        (
            !repo.config.perform.performer_argv.is_empty(),
            "doctor.performer",
            "No CLI performer configured. Configure [perform].performer_argv for `war perform`; prompt-only harness work does not require it.",
        ),
        (
            !repo.config.verify.verifier_argv.is_empty(),
            "doctor.verifier",
            "No CLI independent verifier configured. Configure [verify].verifier_argv for `war verify --run`; configuration alone cannot prove independence.",
        ),
    ] {
        if !configured {
            report.push(Diagnostic::warn(rule, "openwarrant.toml", message));
        }
    }
    result["configuration"] = json!({
        "performer_configured": !repo.config.perform.performer_argv.is_empty(),
        "verifier_configured": !repo.config.verify.verifier_argv.is_empty(),
        "backend_probed": false, "signer_custody": "UNKNOWN"
    });
    // M9: the signing setup, probed without signing anything.
    let (signing, signing_json) = crate::signing_probe::probe(&repo);
    report.diagnostics.extend(signing);
    result["signing"] = signing_json;
    report.note("Signing probes signed nothing: they read PATH, SSH_AUTH_SOCK, `ssh-add -L` (public keys), roles.toml and allowed_signers. Whether a key was loaded with `ssh-add -c`, key custody and backend availability are not probed. A missing piece blocks only the sign-off, not your work; `war doctor --fix-signing` at a terminal offers to repair it.");
    match crate::frontier::run(&repo, alias) {
        Ok((findings, frontier)) => {
            report.diagnostics.extend(findings.diagnostics);
            report.notes.extend(findings.notes);
            if frontier.blocked > 0 {
                report.push(Diagnostic::new(Severity::Warn, "doctor.dependencies", None, format!(
                    "{} stage(s) have unmet milestone dependencies; inspect `war frontier`. This is not a complete admission result.", frontier.blocked)));
            }
            result["frontier"] = json!(frontier);
        }
        Err(error) => unavailable(&mut report, "frontier", error),
    }
    let mut check = vec!["war", "check"];
    let mut frontier = vec!["war", "frontier"];
    if let Some(alias) = alias {
        check.push(alias);
        frontier.push(alias);
    }
    if generated {
        check.push("--generated");
    }
    result["remedies"] = json!([
        {"component":"records", "argv":check, "purpose":"Inspect source diagnostics; preserve signed revisions and request amendments where needed."},
        {"component":"dependencies", "argv":frontier, "purpose":"Inspect blocked stage prerequisites; complete required work rather than clearing records."},
        {"component":"authority", "argv":["war","authority","--help"], "purpose":"Inspect protected authority setup; do not edit active grants to bypass checks."},
        {"component":"performer", "argv":["war","perform","--help"], "purpose":"Inspect performer configuration requirements."},
        {"component":"verifier", "argv":["war","verify","--help"], "purpose":"Inspect independent verifier configuration requirements."},
        {"component":"signing", "argv":["war","doctor","--fix-signing"], "purpose":"At a terminal: repair the signing setup interactively. Signs nothing; writes roles.toml or allowed_signers only when absent, after confirming the exact bytes."}
    ]);
    report.note("Next diagnostic commands: `war check`, `war frontier`, `war authority --help`, `war perform --help`, `war verify --help`. JSON remedies preserve exact arguments; nothing is executed automatically.");
    (report, result)
}
