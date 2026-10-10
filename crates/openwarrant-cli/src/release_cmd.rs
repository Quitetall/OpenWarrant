// SPDX-License-Identifier: Apache-2.0
//! `war sign release <tag>` — one sitting, one signature, before a release
//! (OW-WAR-0148 M14, decision 7).
//!
//! Signing is asynchronous and follows the preset: `never` under vibe,
//! batched at release under team (and when no preset is set), at merge under
//! regulated. Whatever the timing, the acts are the ones `war sign --list`
//! shows, and the signature is `war sign --batch`'s (OW-WAR-0072): this
//! command chooses nothing new and signs nothing itself.
//!
//! - Without `--ssh-sign` it is the request. It lists every act awaiting a
//!   signature, drafts and judges the batch exactly as the signature would
//!   (`war sign --batch --dry-run`), names the acts that sign alone or in a
//!   second batch, prints the command a person runs, and runs `[notify]`.
//!   Nothing is written and no key is asked.
//! - `--dry-run` is that judgement alone, without the notification.
//! - `--ssh-sign` is the batch, each response's meaning naming the release.
//!
//! An agent never waits on any of it: the request returns at once, and the
//! work it lists goes on.

use crate::diagnostic::{Diagnostic, Report};
use crate::preset::{Policy, Signing};
use crate::repo::{RepoError, Repository};
use crate::sign;

/// A tag is one word a person typed: no whitespace, no control character,
/// nothing that could read as a flag.
fn tag_ok(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 128
        && !tag.starts_with('-')
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | '/'))
}

/// `war sign release <tag> [--as] [--ssh-sign | --dry-run]`: the report,
/// the envelope's `result`, and the lines a person reads above the report.
pub fn run(
    repo: &Repository,
    tag: &str,
    actor: Option<String>,
    ssh_sign: bool,
    dry_run: bool,
) -> Result<(Report, serde_json::Value, String), RepoError> {
    let mut report = Report::default();
    if !tag_ok(tag) {
        report.push(Diagnostic::error(
            "release.tag",
            "war sign release".to_owned(),
            format!(
                "{tag:?} is not a release tag: one word of letters, digits and `.-_+/`, as you \
                 tag it (`war sign release v1.2.0`)"
            ),
        ));
        return Ok((report, serde_json::Value::Null, String::new()));
    }
    let policy = match Policy::read(&repo.root) {
        Ok(p) => p,
        Err(e) => {
            report.push(Diagnostic::error(
                crate::preset::CONFIG_RULE,
                crate::init::CONFIG_FILE.to_owned(),
                e,
            ));
            return Ok((report, serde_json::Value::Null, String::new()));
        }
    };
    let opts = sign::Options {
        actor,
        meaning: Some(format!("Signed at release {tag}.")),
        dry_run: !ssh_sign,
        ssh_sign,
        ..sign::Options::default()
    };
    let pending = sign::pending_for(repo, &opts)?;
    let lines: Vec<String> = pending.iter().map(sign::line).collect();
    let mut result = serde_json::json!({
        "schema": "oh.war/release-request/v1",
        "tag": tag,
        "preset": policy.preset_name(),
        "signing": policy.signing.as_str(),
        "pending": lines,
    });
    if policy.signing == Signing::Never {
        report.push(Diagnostic::pass(
            "release.nothing-asked",
            format!(
                "the {} preset asks for no signature, so nothing is required before {tag}{}",
                policy.preset_name(),
                if lines.is_empty() {
                    String::new()
                } else {
                    format!(
                        "; {} act(s) wait anyway and `war sign --list` shows them",
                        lines.len()
                    )
                }
            ),
        ));
        return Ok((report, result, String::new()));
    }
    if pending.is_empty() {
        report.push(Diagnostic::pass(
            "release.nothing-pending",
            format!("nothing awaits a signature before {tag}"),
        ));
        return Ok((report, result, format!("release {tag}: nothing to sign")));
    }
    let mut human = format!(
        "release {tag} (preset {}, signing {}): {} act(s) await a signature:",
        policy.preset_name(),
        policy.signing.as_str(),
        lines.len()
    );
    for l in &lines {
        human.push_str(&format!("\n  {l}"));
    }
    if policy.signing == Signing::Merge {
        human.push_str(
            "\nUnder signing at merge these should have been signed as each merged; one batch \
             signs them now.",
        );
    }
    let batch = crate::batch_cmd::run(repo, &[], &opts)?;
    let would = batch
        .diagnostics
        .iter()
        .find(|d| d.rule == "batch.would-record")
        .map(|d| d.message.clone());
    let left_out: Vec<String> = batch
        .diagnostics
        .iter()
        .filter(|d| d.rule == "batch.left-out")
        .map(|d| d.message.clone())
        .collect();
    let signer = opts
        .actor
        .clone()
        .or_else(|| batch_signer(would.as_deref()));
    let command = format!(
        "war sign release {tag} --ssh-sign{}",
        signer
            .as_deref()
            .map(|s| format!(" --as {s:?}"))
            .unwrap_or_default()
    );
    result["batch"] = serde_json::json!(would);
    result["left_out"] = serde_json::json!(left_out);
    result["command"] = serde_json::json!(command);
    for d in batch.diagnostics {
        report.push(d);
    }
    if ssh_sign {
        return Ok((report, result, human));
    }
    if let Some(w) = &would {
        human.push_str(&format!("\nOne signature carries {w}."));
    }
    if !left_out.is_empty() {
        human.push_str(&format!(
            "\n{} act(s) sign alone or in a second batch (below).",
            left_out.len()
        ));
    }
    if report.is_ready() {
        human.push_str(&format!(
            "\nA person runs: {command}\nNothing was written and no key was asked; work goes on \
             meanwhile."
        ));
        if !dry_run
            && let Some(d) = crate::notify::human_waits(
                &repo.root,
                &crate::notify::Wait {
                    event: "release.requested",
                    subject: tag,
                    message: &format!(
                        "release {tag}: {} act(s) wait on one signature",
                        lines.len()
                    ),
                    command: &command,
                },
            )
        {
            report.push(d);
        }
    }
    Ok((report, result, human))
}

/// The signer a judged batch names ("2 act(s) as Ada (authorizer), one
/// signature: ...").
fn batch_signer(would: Option<&str>) -> Option<String> {
    let rest = would?.split_once(" as ")?.1;
    let name = rest.split_once(" (")?.0;
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_is_one_plain_word() {
        for ok in ["v1.2.0", "1.0.0-alpha.2", "release/2026-10", "v2+build"] {
            assert!(tag_ok(ok), "{ok}");
        }
        for bad in ["", "--ssh-sign", "v1 2", "v1\n", "v1;rm", "ü"] {
            assert!(!tag_ok(bad), "{bad:?}");
        }
    }

    #[test]
    fn the_signer_is_read_from_the_judged_batch() {
        assert_eq!(
            batch_signer(Some(
                "2 act(s) as Ada Lovelace (authorizer), one signature: authorize X"
            ))
            .as_deref(),
            Some("Ada Lovelace")
        );
        assert_eq!(batch_signer(None), None);
    }
}
