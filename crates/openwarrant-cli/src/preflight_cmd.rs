// SPDX-License-Identifier: Apache-2.0
//! Read-only legacy Preflight. Unknown checks block; this report is not a
//! signed receipt, execution permission, or an assurance verdict.
use crate::{
    diagnostic::{Diagnostic, Report, Severity},
    repo::{RepoError, Repository},
};
use openwarrant_core::{
    ExecutionCondition,
    preflight::{self, CheckGroup, CheckOutcome, PreflightReceipt, Readiness},
};
use serde::Serialize;
use std::{collections::BTreeMap, fmt::Write as _};

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pass,
    Fail,
    Unknown,
}
#[derive(Serialize)]
pub struct Check {
    name: String,
    status: Status,
    observation: String,
}
#[derive(Serialize)]
pub struct Dimension {
    group: CheckGroup,
    status: Status,
    checks: Vec<Check>,
}
#[derive(Serialize)]
pub struct ResultView {
    schema: &'static str,
    warrant: String,
    contract_digest: Option<String>,
    pub readiness: Readiness,
    execution_condition: ExecutionCondition,
    state_persisted: bool,
    meaning: &'static str,
    limits: &'static str,
    dimensions: Vec<Dimension>,
}
const LIMITS: &str = "Partial local assessment. No live actor path, provider, secret, protected gate or side-effect authority is exercised. No receipt is ingested and no recorded state changes. Prototype work is governed by its own explicit action gates.";

pub fn run(repo: &Repository, alias: &str) -> Result<(ResultView, Report), RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let loaded = repo.load_warrant(&dir)?;
    let mut observations: BTreeMap<&str, (Status, String)> = BTreeMap::new();
    let mut record = |name, status, message: String| {
        let entry = observations.entry(name).or_insert((status, String::new()));
        entry.0 = worst(entry.0, status);
        if !entry.1.is_empty() {
            entry.1.push_str("; ");
        }
        entry.1.push_str(&message);
    };
    if loaded.validated.is_some() {
        record(
            "profile valid",
            Status::Pass,
            "Manifest validated against the built-in profile vocabulary.".into(),
        );
    }
    let errors: Vec<_> = loaded
        .report
        .diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    if !errors.is_empty() {
        // A loader error need not mean a required atom is absent. Preserve the
        // observed failure as a contract compilation failure, without inventing
        // an atom-presence result for unrelated invalid inputs.
        record(
            "contract digest reproducible",
            Status::Fail,
            errors.join("; "),
        );
        if loaded.validated.is_none() {
            record("profile valid", Status::Fail, errors.join("; "));
        } else if let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated)
            && validated
                .raw
                .atoms
                .iter()
                .filter(|a| a.required && a.path.is_some())
                .any(|a| !basis.atoms.iter().any(|loaded| loaded.ordinal == a.ordinal))
        {
            record(
                "required atoms present",
                Status::Fail,
                "At least one required local atom could not be loaded.".into(),
            );
        }
    } else if let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated) {
        let has_unresolved = validated
            .raw
            .atoms
            .iter()
            .filter(|a| a.required)
            .any(|a| a.path.is_none());
        if !has_unresolved {
            record(
                "required atoms present",
                Status::Pass,
                "All required local atom files loaded without loader errors.".into(),
            );
        }
        for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
            match std::str::from_utf8(&atom.bytes)
                .map_err(|e| e.to_string())
                .and_then(|text| {
                    openwarrant_core::milestones::parse(text).map_err(|e| e.to_string())
                }) {
                Ok(graph) => {
                    record("no stage or milestone cycle", Status::Pass, "Existing milestone parser validated local references and dependency cycles; executor semantics are assessed separately.".into());
                    let unreferenced = graph.unreferenced_stages();
                    record(
                        "required stages reachable",
                        if unreferenced.is_empty() {
                            Status::Pass
                        } else {
                            Status::Unknown
                        },
                        if unreferenced.is_empty() {
                            "Every declared stage is referenced by a milestone in the parsed local graph.".into()
                        } else {
                            format!(
                                "Stages outside milestone references require scope assessment: {}",
                                unreferenced.join(", ")
                            )
                        },
                    );
                }
                Err(error) => {
                    record(
                        "no stage or milestone cycle",
                        Status::Fail,
                        format!("Local graph could not be validated: {error}"),
                    );
                    record(
                        "required stages reachable",
                        Status::Unknown,
                        "Reachability cannot be established for an invalid graph.".into(),
                    );
                }
            }
        }
    }
    let mut contract_digest = None;
    if errors.is_empty()
        && let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated)
    {
        match openwarrant_compiler::lower(basis, validated).and_then(|ir| ir.contract_digest()) {
            Ok(digest) => {
                contract_digest = Some(digest.clone());
                // Reproduce a recorded commitment, rather than treating one
                // successful hash computation as evidence of reproducibility.
                let authorization = (|| {
                    let path = dir.join("authorization.toml");
                    match std::fs::symlink_metadata(&path) {
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                        Err(e) => return Err(format!("Cannot inspect authorization record: {e}")),
                        Ok(_) => {}
                    }
                    let metadata = std::fs::metadata(&path)
                        .map_err(|e| format!("Cannot inspect authorization record: {e}"))?;
                    if !metadata.is_file() {
                        return Err("Authorization record has wrong container type.".into());
                    }
                    repo.load_authorization(&dir).map_err(|e| e.to_string())
                })();
                match authorization {
                    Ok(Some(authorization)) if authorization.revision.contract_digest == digest => {
                        record(
                            "contract digest reproducible",
                            Status::Pass,
                            format!(
                                "Fresh compilation reproduces recorded contract digest {digest}. This does not validate the authorizer's authority."
                            ),
                        );
                    }
                    Ok(Some(_)) => record(
                        "authorization valid",
                        Status::Fail,
                        "Recorded authorization does not bind the current compiled contract."
                            .into(),
                    ),
                    Ok(None) => record(
                        "authorization valid",
                        Status::Fail,
                        "No authorization record exists for this legacy contract.".into(),
                    ),
                    Err(error) => record("authorization valid", Status::Unknown, error),
                }
            }
            Err(error) => record(
                "contract digest reproducible",
                Status::Fail,
                format!("Contract compilation failed: {error}"),
            ),
        }
    }
    let mut receipt = PreflightReceipt {
        warrant: alias.into(),
        ..Default::default()
    };
    let mut dimensions: Vec<Dimension> = Vec::new();
    let mut report = Report::default();
    for (group, name) in preflight::all_checks() {
        let (status, observation) = observations.remove(name).unwrap_or_else(|| (Status::Unknown, match group {
            CheckGroup::Runtime => "No live actor-path observation supplied; local CLI access is not proof of actor/runtime availability.",
            CheckGroup::Gates => "Gate askability, protected fixtures and verifier execution have not been exercised by this command.",
            CheckGroup::Authority => "Assignments and effective side-effect authority have not been established for a concrete execution attempt.",
            CheckGroup::Context => "Complete versioned context, policy and Workspace Basis have not been established for a concrete attempt.",
            CheckGroup::Graph => "This graph property requires additional port, output or executor observations beyond local syntax validation.",
            CheckGroup::Contract => "This requirement has no sufficient local observation; record syntax or a digest alone cannot establish it.",
        }.into()));
        receipt.outcomes.insert(
            name.into(),
            match status {
                Status::Pass => CheckOutcome::Passed,
                Status::Fail => CheckOutcome::Failed,
                Status::Unknown => CheckOutcome::NotRun,
            },
        );
        let severity = match status {
            Status::Pass => Severity::Pass,
            Status::Fail => Severity::Error,
            Status::Unknown => Severity::Unknown,
        };
        report.push(Diagnostic::new(
            severity,
            format!("preflight.{group}"),
            None,
            format!("{name}: {observation}"),
        ));
        if dimensions.last().is_none_or(|d| d.group != group) {
            dimensions.push(Dimension {
                group,
                status: Status::Pass,
                checks: vec![],
            });
        }
        let dimension = dimensions.last_mut().expect("group just inserted");
        dimension.status = worst(dimension.status, status);
        dimension.checks.push(Check {
            name: name.into(),
            status,
            observation,
        });
    }
    let readiness = receipt.readiness();
    report.note(PreflightReceipt::meaning());
    report.note(LIMITS);
    Ok((
        ResultView {
            schema: "oh.war/preflight-report/v1",
            warrant: alias.into(),
            contract_digest,
            readiness,
            execution_condition: match readiness {
                Readiness::NotReady => ExecutionCondition::Blocked,
                Readiness::Ready => ExecutionCondition::Clear,
            },
            state_persisted: false,
            meaning: PreflightReceipt::meaning(),
            limits: LIMITS,
            dimensions,
        },
        report,
    ))
}
pub fn render(result: &ResultView) -> String {
    let mut out = format!("{}: {}\n", result.warrant, result.readiness);
    for dimension in &result.dimensions {
        let label = |s| match s {
            Status::Pass => "PASS",
            Status::Fail => "FAIL",
            Status::Unknown => "UNKNOWN",
        };
        let _ = writeln!(out, "{}: {}", dimension.group, label(dimension.status));
        for check in &dimension.checks {
            let _ = writeln!(
                out,
                "  {} {} — {}",
                label(check.status),
                check.name,
                check.observation
            );
        }
    }
    let _ = writeln!(out, "{}\n{}", result.meaning, result.limits);
    out
}

fn worst(left: Status, right: Status) -> Status {
    match (left, right) {
        (Status::Fail, _) | (_, Status::Fail) => Status::Fail,
        (Status::Unknown, _) | (_, Status::Unknown) => Status::Unknown,
        _ => Status::Pass,
    }
}
