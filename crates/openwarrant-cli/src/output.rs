// SPDX-License-Identifier: AGPL-3.0-or-later
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
//!   "diagnostics": [{"severity": "error", "rule": "…", "file": "…", "message": "…"}],
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
        exit_code: exit_code(report),
        result,
    };
    // Pretty, and only ever the envelope on stdout: rmcp and any pipe reader
    // depend on stdout being one JSON value.
    serde_json::to_string_pretty(&e).unwrap_or_else(|err| {
        format!(
            "{{\"schema\":\"{SCHEMA}\",\"command\":{command:?},\"diagnostics\":[{{\"severity\":\"error\",\"rule\":\"cli.serialize\",\"file\":null,\"message\":{:?}}}],\"notes\":[],\"counts\":{{\"pass\":0,\"warn\":0,\"unknown\":0,\"error\":1,\"worst\":\"error\"}},\"verdict\":\"not_ready\",\"verdict_line\":\"NOT READY\",\"exit_code\":2}}",
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
}
