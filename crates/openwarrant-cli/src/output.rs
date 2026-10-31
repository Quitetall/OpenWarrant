// SPDX-License-Identifier: Apache-2.0
//! Machine output for every command (SAS §76.4): one envelope, one schema.
//!
//! §76.4 says every command SHOULD support `--json` with a stable versioned
//! result schema. Before this module, one of twenty-three did, and `Report`
//! could not even be serialised. An agent driving `war` had to parse the human
//! rendering — which is the "reimplement WAR semantics in the consumer" failure
//! §77.3 forbids, one layer down.
//!
//! # The envelope
//!
//! ```json
//! {
//!   "schema": "oh.war/report/v1",
//!   "command": "check",
//!   "diagnostics": [{"severity": "error", "rule": "…", "file": "…", "message": "…",
//!                    "remedy": {"kind": "auto" | "human" | "info", "argv": ["war", "…"], "purpose": "…"}}],
//!   "notes": ["…"],
//!   "counts": {"pass": 0, "warn": 0, "unknown": 0, "error": 0, "worst": "pass"},
//!   "verdict": "well_formed" | "not_ready",
//!   "verdict_line": "…",
//!   "exit_code": 0,
//!   "result": { … }
//! }
//! ```
//!
//! `result` is optional and command-specific; every payload carries its own
//! `schema` where one exists (request documents already do). Field names are
//! frozen at 1.0; additions are additive.
//!
//! # Why the serialisation lives here and not on `Diagnostic`
//!
//! `diagnostic.rs` is a pinned deliverable of a resolved Warrant (OW-WAR-0005).
//! Deriving `Serialize` there would be a correction for a derive. A mirror
//! struct here costs nothing and keeps the wire shape explicit — the JSON
//! field names are the contract, not the Rust ones.

use std::io::Write;

use serde::Serialize;

use crate::diagnostic::{Report, Severity};

pub const SCHEMA: &str = "oh.war/report/v1";

/// Human (the §71.7 rendering) or JSON (the envelope). Threaded from the
/// global `--json` flag into every command arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Human,
    Json,
}

impl Mode {
    #[must_use]
    pub const fn from_flag(json: bool) -> Self {
        if json { Self::Json } else { Self::Human }
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Serialize)]
pub(crate) struct WireDiagnostic<'a> {
    severity: &'static str,
    rule: &'a str,
    file: Option<&'a str>,
    message: &'a str,
    /// §76.2, OW-WAR-0112: the command that answers this finding and whose
    /// act it is. Additive — absent when nothing answers, never null.
    #[serde(skip_serializing_if = "Option::is_none")]
    remedy: Option<crate::remedy::Remedy>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Serialize)]
pub(crate) struct Counts {
    pass: usize,
    warn: usize,
    unknown: usize,
    error: usize,
    worst: &'static str,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Serialize)]
pub(crate) struct Envelope<'a> {
    schema: &'static str,
    command: &'a str,
    diagnostics: Vec<WireDiagnostic<'a>>,
    notes: &'a [String],
    counts: Counts,
    verdict: &'static str,
    verdict_line: String,
    exit_code: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
}

const fn severity_name(s: Severity) -> &'static str {
    match s {
        Severity::Pass => "pass",
        Severity::Warn => "warn",
        Severity::Unknown => "unknown",
        Severity::Error => "error",
    }
}

static VERBOSE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

static QUIET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set once from the global `--quiet` flag.
pub fn set_quiet(on: bool) {
    QUIET.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the brief rendering keeps only what blocks (`--quiet`).
#[must_use]
pub fn quiet() -> bool {
    QUIET.load(std::sync::atomic::Ordering::Relaxed)
}

/// Set once from the global `--verbose` flag.
pub fn set_verbose(on: bool) {
    VERBOSE.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Whether the human rendering lists every finding (`--verbose`, or
/// `OPENWARRANT_VERBOSE=1` for a harness such as the conformance battery that
/// asserts on PASS lines) instead of the brief one.
#[must_use]
pub fn verbose() -> bool {
    VERBOSE.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var_os("OPENWARRANT_VERBOSE").is_some_and(|v| !v.is_empty() && v != "0")
}

/// Passes the brief rendering prints: they say what the run covered.
const HEADLINE_RULES: &[&str] = &["check.changed"];

/// `OW-WAR-0001`, or the alias part of `OW-WAR-0001/D-002`.
fn is_alias(token: &str) -> bool {
    let head = token.split('/').next().unwrap_or(token);
    let mut parts = head.splitn(3, '-');
    let (Some(ns), Some(war), Some(n)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    ns.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && ns
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && war == "WAR"
        && n.len() == 4
        && n.chars().all(|c| c.is_ascii_digit())
}

/// One remedy line for a command that answers findings in several Warrants:
/// aliases become `<alias>`, and a pin refresh named for several Warrants is
/// the refresh of all of them, which is what `pins --refresh` without
/// `--alias` does (it never moves a resolved Warrant's pins).
fn folded(argv: &[String]) -> String {
    if argv.starts_with(&["war", "admin", "pins", "--refresh"].map(String::from)) {
        return "war admin pins --refresh".to_owned();
    }
    argv.iter()
        .map(|t| match (is_alias(t), t.contains('/')) {
            (false, _) => t.clone(),
            (true, false) => "<alias>".to_owned(),
            (true, true) => "<alias>/<D-id>".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// `text` with each run of whitespace one space, cut at a word boundary to
/// at most `max` characters and marked when cut, so a cut never ends inside
/// a word.
#[must_use]
pub fn clipped(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let cut: String = flat.chars().take(max).collect();
    let at = cut.rfind(' ').filter(|&i| i > max / 2).unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end_matches([',', ';', ':', ' ']))
}

/// What a finding is about: the id its message starts with (`OW-WAR-0001:`,
/// `OW-ADR-0005:`, `SAS-0.1.0-draft.1:`), else the Warrant its file is in.
fn subject(d: &crate::diagnostic::Diagnostic) -> Option<String> {
    if let Some((head, _)) = d.message.split_once(':')
        && !head.is_empty()
        && head.len() <= 40
        && !head.contains(char::is_whitespace)
        && head.contains('-')
    {
        return Some(head.to_owned());
    }
    d.file
        .as_deref()?
        .split('/')
        .find(|seg| is_alias(seg))
        .map(str::to_owned)
}

/// Up to `max` names, then how many more.
fn listed<'a>(names: impl IntoIterator<Item = &'a str>, max: usize) -> String {
    let all: Vec<&str> = names.into_iter().collect();
    let shown = all[..all.len().min(max)].join(", ");
    match all.len().saturating_sub(max) {
        0 => shown,
        more => format!("{shown} and {more} more"),
    }
}

/// The brief human rendering, the default. A clean `war check` on a
/// 147-Warrant corpus printed 481 KB, 1,891 lines of them PASS, and an agent
/// paid for every byte. Errors and unknowns print in full, exactly as
/// `--verbose` prints them. A warning rule that fires once prints in full;
/// one that fires many times is one line with its count and first message.
/// Passes are a count. The same remedy for many Warrants is one line. What
/// the run did not check is still named on every run, by label.
#[must_use]
pub fn brief(report: &Report) -> String {
    brief_with(report, quiet())
}

/// [`brief`], or under `quiet` only what blocks: errors, unknowns, their
/// remedies, the counts and the verdict.
#[must_use]
pub fn brief_with(report: &Report, quiet: bool) -> String {
    use crate::diagnostic::Diagnostic;
    use std::fmt::Write as _;
    let mut out = String::new();
    // A pass that says what the run covered is the one a reader needs.
    let mut headlines = 0;
    for d in &report.diagnostics {
        if d.severity == Severity::Pass && HEADLINE_RULES.contains(&d.rule.as_str()) {
            headlines += 1;
            let _ = writeln!(out, "{d}");
        }
    }
    for d in &report.diagnostics {
        if matches!(d.severity, Severity::Error | Severity::Unknown) {
            let _ = writeln!(out, "{d}");
        }
    }
    let mut warn_rules: Vec<(&str, Vec<&Diagnostic>)> = Vec::new();
    for d in report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warn)
    {
        match warn_rules.iter_mut().find(|(r, _)| *r == d.rule) {
            Some((_, all)) => all.push(d),
            None => warn_rules.push((&d.rule, vec![d])),
        }
    }
    if quiet {
        warn_rules.clear();
    }
    let mut folded_warns = 0;
    for (rule, all) in &warn_rules {
        if let [only] = all.as_slice() {
            let _ = writeln!(out, "{only}");
        } else {
            folded_warns += all.len();
            let _ = writeln!(
                out,
                "{:<7} {:<34} ×{}  {}",
                Severity::Warn,
                rule,
                all.len(),
                clipped(&all[0].message, 200)
            );
            let mut names: Vec<String> = Vec::new();
            for s in all.iter().filter_map(|d| subject(d)) {
                if !names.contains(&s) {
                    names.push(s);
                }
            }
            if !names.is_empty() {
                let _ = writeln!(
                    out,
                    "{:<7} {:<34}   → {}",
                    "",
                    "",
                    listed(names.iter().map(String::as_str), 8)
                );
            }
        }
    }
    // (kind, argv with aliases blanked) → (distinct commands, findings).
    type Key = (&'static str, Vec<String>);
    let mut remedies: Vec<(Key, std::collections::BTreeSet<Vec<String>>, usize)> = Vec::new();
    for d in report.diagnostics.iter().filter(|d| match d.severity {
        Severity::Pass => false,
        Severity::Warn => !quiet,
        Severity::Error | Severity::Unknown => true,
    }) {
        let Some(r) = crate::remedy::remedy_for(d) else {
            continue;
        };
        let template: Vec<String> = r
            .argv
            .iter()
            .map(|t| {
                if is_alias(t) {
                    "<alias>".to_owned()
                } else {
                    t.clone()
                }
            })
            .collect();
        let key = (r.kind.label(), template);
        match remedies.iter_mut().find(|(k, _, _)| *k == key) {
            Some((_, commands, n)) => {
                commands.insert(r.argv.clone());
                *n += 1;
            }
            None => remedies.push((key, [r.argv.clone()].into(), 1)),
        }
    }
    if !remedies.is_empty() {
        let _ = writeln!(out, "\nREMEDIES:");
        for ((kind, _), commands, n) in &remedies {
            let (line, scope) = match commands.iter().next() {
                Some(only) if commands.len() == 1 => (only.join(" "), String::new()),
                Some(first) => {
                    let mut names: Vec<&str> = Vec::new();
                    for t in commands.iter().flatten().filter(|t| is_alias(t)) {
                        let a = t.split('/').next().unwrap_or(t);
                        if !names.contains(&a) {
                            names.push(a);
                        }
                    }
                    let names = match names.as_slice() {
                        [] => String::new(),
                        _ => format!(": {}", listed(names, 5)),
                    };
                    (
                        folded(first),
                        format!(", {} Warrants{names}", commands.len()),
                    )
                }
                None => continue,
            };
            let _ = writeln!(out, "  {kind:<5} {line}   (×{n}{scope})");
        }
    }
    let _ = writeln!(
        out,
        "\n{} pass · {} warn · {} unknown · {} error   (worst: {})",
        report.count(Severity::Pass),
        report.count(Severity::Warn),
        report.count(Severity::Unknown),
        report.count(Severity::Error),
        report.worst(),
    );
    if !report.notes.is_empty() && !quiet {
        let labels: Vec<String> = report
            .notes
            .iter()
            .map(|n| clipped(n.split(" — ").next().unwrap_or(n), 100))
            .collect();
        let _ = writeln!(out, "NOT CHECKED: {}", labels.join(" · "));
    }
    let _ = writeln!(out, "\n{}", report.verdict_line());
    let folded = report.count(Severity::Pass) - headlines + folded_warns;
    if !quiet && (folded > 0 || !report.notes.is_empty()) {
        let _ = writeln!(
            out,
            "(`--verbose` lists all {} findings and what each unchecked aspect means)",
            report.diagnostics.len()
        );
    }
    out
}

/// The exit code a report earns: 0 when well-formed, 2 otherwise. One place,
/// so the fifteen command arms that used to spell this ladder cannot drift.
#[must_use]
pub fn exit_code(report: &Report) -> u8 {
    if report.is_ready() {
        crate::EXIT_OK
    } else {
        crate::EXIT_NOT_READY
    }
}

/// Render the envelope for a report.
#[must_use]
pub fn envelope(command: &str, report: &Report, result: Option<serde_json::Value>) -> String {
    envelope_with_status(command, report, result, exit_code(report))
}

fn envelope_with_status(
    command: &str,
    report: &Report,
    result: Option<serde_json::Value>,
    status: u8,
) -> String {
    let e = Envelope {
        schema: SCHEMA,
        command,
        diagnostics: report
            .diagnostics
            .iter()
            .map(|d| WireDiagnostic {
                severity: severity_name(d.severity),
                rule: &d.rule,
                file: d.file.as_deref(),
                message: &d.message,
                remedy: crate::remedy::remedy_for(d),
            })
            .collect(),
        notes: &report.notes,
        counts: Counts {
            pass: report.count(Severity::Pass),
            warn: report.count(Severity::Warn),
            unknown: report.count(Severity::Unknown),
            error: report.count(Severity::Error),
            worst: severity_name(report.worst()),
        },
        verdict: if report.is_ready() {
            "well_formed"
        } else {
            "not_ready"
        },
        verdict_line: report.verdict_line(),
        exit_code: status,
        result,
    };
    // Pretty, and only ever the envelope on stdout: rmcp and any pipe reader
    // depend on stdout being one JSON value.
    serde_json::to_string_pretty(&e).unwrap_or_else(|err| {
        format!(
            "{{\"schema\":\"{SCHEMA}\",\"command\":{command:?},\"diagnostics\":[{{\"severity\":\"error\",\"rule\":\"cli.serialize\",\"file\":null,\"message\":{:?}}}],\"notes\":[],\"counts\":{{\"pass\":0,\"warn\":0,\"unknown\":0,\"error\":1,\"worst\":\"error\"}},\"verdict\":\"not_ready\",\"verdict_line\":\"NOT READY\",\"exit_code\":{status}}}",
            err.to_string()
        )
    })
}

/// Print a report in the chosen mode and return the exit code.
pub fn finish(mode: Mode, command: &str, report: &Report, result: Option<serde_json::Value>) -> u8 {
    match mode {
        Mode::Human => crate::check::print(report),
        Mode::Json => {
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", envelope(command, report, result));
        }
    }
    exit_code(report)
}

/// Emit a value that is the whole answer (a request document, a projection):
/// TOML or the human rendering in `Human` mode, the envelope with `result` in
/// `Json` mode. `human` is what the command printed before `--json` existed —
/// unchanged, so no existing reader moves.
pub fn emit(mode: Mode, command: &str, human: &str, result: serde_json::Value) {
    match mode {
        Mode::Human => println!("{human}"),
        Mode::Json => {
            let report = Report::default();
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", envelope(command, &report, Some(result)));
        }
    }
}

/// The bare-error path (`run` returned `Err`): under `--json` the error is
/// still an envelope on stdout, so a caller never has to parse stderr.
pub fn error(mode: Mode, message: &str) {
    error_with_status(mode, message, crate::EXIT_DIAGNOSTIC);
}

/// Render an invocation error with the actual process status chosen by the caller.
pub fn error_with_status(mode: Mode, message: &str, status: u8) {
    match mode {
        Mode::Human => eprintln!("error: {message}"),
        Mode::Json => {
            let mut report = Report::default();
            report.push(crate::diagnostic::Diagnostic::new(
                Severity::Error,
                "cli.error",
                None,
                message,
            ));
            let mut out = std::io::stdout().lock();
            let _ = writeln!(
                out,
                "{}",
                envelope_with_status("error", &report, None, status)
            );
        }
    }
}

/// A failed observation is UNKNOWN, rather than an invalid-input diagnostic.
pub fn unavailable(mode: Mode, rule: &str, message: &str) {
    match mode {
        Mode::Human => eprintln!("UNKNOWN {rule}: {message}"),
        Mode::Json => {
            let mut report = Report::default();
            report.push(crate::diagnostic::Diagnostic::new(
                Severity::Unknown,
                rule,
                None,
                message,
            ));
            let mut out = std::io::stdout().lock();
            let _ = writeln!(out, "{}", envelope("error", &report, None));
        }
    }
}

/// A value's canonical JSON as a `serde_json::Value`, for `result`.
pub fn value<T: Serialize>(v: &T) -> serde_json::Value {
    serde_json::to_value(v).unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Diagnostic;

    #[test]
    fn the_envelope_has_exactly_the_frozen_keys_and_the_exit_code_matches_readiness() {
        let mut r = Report::default();
        r.push(Diagnostic::pass("manifest.valid", "ok"));
        r.push(Diagnostic::error(
            "generated.drift",
            "x".to_owned(),
            "moved",
        ));
        r.note("Preflight not run");
        let text = envelope("check", &r, None);
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "command",
                "counts",
                "diagnostics",
                "exit_code",
                "notes",
                "schema",
                "verdict",
                "verdict_line"
            ]
        );
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(v["command"], "check");
        assert_eq!(v["exit_code"], 2);
        assert_eq!(v["verdict"], "not_ready");
        assert_eq!(v["counts"]["error"], 1);
        assert_eq!(v["counts"]["worst"], "error");
        assert_eq!(v["diagnostics"][1]["severity"], "error");
        assert_eq!(v["diagnostics"][1]["file"], "x");
        assert_eq!(v["diagnostics"][0]["file"], serde_json::Value::Null);
        assert_eq!(v["notes"][0], "Preflight not run");
        assert_eq!(exit_code(&r), crate::EXIT_NOT_READY);
    }

    #[test]
    fn a_clean_report_is_well_formed_with_exit_zero_and_a_result_when_given() {
        let r = Report::default();
        let text = envelope("status", &r, Some(serde_json::json!({"schema": "x"})));
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["verdict"], "well_formed");
        assert_eq!(v["exit_code"], 0);
        assert_eq!(v["result"]["schema"], "x");
        assert!(text.starts_with('{'), "stdout is one JSON value");
    }

    #[test]
    fn an_unknown_blocks_readiness_in_the_envelope_too() {
        let mut r = Report::default();
        r.push(Diagnostic::unknown(
            "gate-run.unaskable",
            "g".to_owned(),
            "could not run",
        ));
        let v: serde_json::Value = serde_json::from_str(&envelope("gate", &r, None)).unwrap();
        assert_eq!(v["verdict"], "not_ready");
        assert_eq!(v["counts"]["worst"], "unknown");
    }

    fn noisy() -> Report {
        let mut r = Report::default();
        for i in 0..40 {
            r.push(Diagnostic::pass(
                "manifest.valid",
                format!("OW-WAR-{i:04}: ok"),
            ));
        }
        for i in 1..=5 {
            r.push(Diagnostic::warn(
                "deliverable.pin-stale",
                format!("docs/warrants/OW-WAR-{i:04}/deliverables.toml"),
                format!(
                    "OW-WAR-{i:04}: D-001 records sha256:aa for x.rs and the file is now sha256:bb"
                ),
            ));
        }
        r.push(Diagnostic::warn(
            "authority.unprotected",
            "openwarrant.toml".to_owned(),
            "no store",
        ));
        r.push(Diagnostic::error(
            "atom.header",
            "docs/warrants/OW-WAR-0062/atoms/10-intent.md".to_owned(),
            "the header's `order` is \"11\"",
        ));
        r.note("gate execution — `war check` does not run gates");
        r
    }

    #[test]
    fn the_brief_rendering_keeps_errors_whole_and_folds_what_repeats() {
        let text = brief_with(&noisy(), false);
        // The error, its file and its line are all there.
        assert!(text.contains("ERROR atom.header"), "{text}");
        assert!(
            text.contains("→ docs/warrants/OW-WAR-0062/atoms/10-intent.md"),
            "{text}"
        );
        // No PASS line, but every pass counted.
        assert!(!text.contains("PASS "), "{text}");
        assert!(
            text.contains("40 pass · 6 warn · 0 unknown · 1 error"),
            "{text}"
        );
        // A warning rule that fires once is whole; five of one rule are one line.
        assert!(text.contains("WARN authority.unprotected"), "{text}");
        assert_eq!(text.matches("deliverable.pin-stale").count(), 1, "{text}");
        assert!(text.contains("×5  OW-WAR-0001: D-001"), "{text}");
        // The folded line still names every Warrant it stands for.
        assert!(
            text.contains("→ OW-WAR-0001, OW-WAR-0002, OW-WAR-0003, OW-WAR-0004, OW-WAR-0005"),
            "{text}"
        );
        // Five refreshes are the one refresh of all of them, by name.
        assert!(
            text.contains(
                "war admin pins --refresh   (×5, 5 Warrants: OW-WAR-0001, OW-WAR-0002, \
                 OW-WAR-0003, OW-WAR-0004, OW-WAR-0005)"
            ),
            "{text}"
        );
        assert!(!text.contains("--alias OW-WAR-0002"), "{text}");
        // What was not checked is still named on every run, by label.
        assert!(text.contains("NOT CHECKED: gate execution"), "{text}");
        assert!(text.contains("`--verbose` lists all 47 findings"), "{text}");
    }

    #[test]
    fn a_quiet_brief_rendering_is_only_what_blocks() {
        let text = brief_with(&noisy(), true);
        assert!(text.contains("ERROR atom.header"), "{text}");
        assert!(!text.contains("WARN"), "{text}");
        assert!(!text.contains("pins --refresh"), "{text}");
        assert!(!text.contains("NOT CHECKED"), "{text}");
        assert!(text.contains("1 error"), "{text}");
    }

    #[test]
    fn an_alias_is_a_namespace_war_and_four_digits() {
        assert!(is_alias("OW-WAR-0001"));
        assert!(is_alias("LQ2-WAR-0130/D-002"));
        assert!(!is_alias("OW-SAS-0001"));
        assert!(!is_alias("ow-WAR-0001"));
        assert!(!is_alias("OW-WAR-01"));
        assert!(!is_alias("--alias"));
    }

    #[test]
    fn a_cut_ends_at_a_word_and_runs_of_space_are_one() {
        assert_eq!(clipped("a   b\n c", 80), "a b c");
        assert_eq!(clipped("alpha beta gamma delta", 14), "alpha beta…");
        assert!(!clipped(&"word ".repeat(60), 100).ends_with(" …"));
    }
}
