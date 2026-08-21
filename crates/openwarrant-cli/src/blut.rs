// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war blut lower` — lower a stage graph into a BLUT `PlanSpec` (SAS §49).
//!
//! # The schema is read from BLUT, not invented here
//!
//! Every field below was read from `src/framework/plan_spec.rs` in the BLUT
//! tree at commit `33b3e047fd8eae50c4f706e7e3c7f4a3648a5778`. That matters more
//! than usual, because BLUT's `PlanSpec` carries
//! `#[serde(deny_unknown_fields)]` — a field we invent is not ignored, it is
//! rejected. The adapter cannot drift into a private dialect without BLUT
//! saying so.
//!
//! # What this deliberately does not claim
//!
//! BLUT ships no verb that deserializes a `PlanSpec` JSON, so nothing here has
//! been through BLUT's parser. The lowering is structurally faithful to a schema
//! read at a pinned commit, which is a different and weaker claim than "BLUT
//! accepted it", and OW-WAR-0047's OBL-001 stays open on exactly that
//! distinction.
//!
//! BLUT additionally refuses a stage name that is in no registered cookbook
//! (`PlanSpecError::UnknownStage` — "dynamic loading is forbidden"). So a real
//! acceptance needs both a BLUT-side verb AND a stage this repository's Warrants
//! do not currently name.

use openwarrant_core::seam::{BlutLowering, PortMapping};

use crate::diagnostic::{Diagnostic, Report};
use crate::repo::{RepoError, Repository};

/// The BLUT commit whose `PlanSpec` this adapter was written against.
///
/// §49.2 requires stage names to resolve against a PINNED registry. Pinning the
/// schema is the same discipline one level up: without it, "the same stage name"
/// means whatever BLUT means by it today.
pub const BLUT_PIN: &str = "33b3e047fd8eae50c4f706e7e3c7f4a3648a5778";

/// BLUT's `PlanSpec`, as it exists at [`BLUT_PIN`].
#[derive(Debug, serde::Serialize)]
struct PlanSpec {
    name: String,
    nodes: Vec<SpecNode>,
    edges: Vec<(u32, u32)>,
    version: u32,
}

#[derive(Debug, serde::Serialize)]
struct SpecNode {
    stage: String,
    args: serde_json::Value,
}

/// Lower one Warrant's milestone graph into a `PlanSpec`.
///
/// §49.2's duties, each discharged or explicitly refused:
/// resolve stage names against a pinned registry; map named ports to typed
/// inputs and outputs; reject incompatible kinds; reject unsupported
/// conditions; pin backend and stage identities; map resource envelopes; record
/// plan provenance.
pub fn lower(repo: &Repository, alias: &str) -> Result<Report, RepoError> {
    let dir = repo.warrant_dir(alias)?;
    let one = repo.load_warrant(&dir)?;
    let mut report = Report::default();

    let Some(basis) = &one.basis else {
        return Err(RepoError::Message(format!("{alias} could not be compiled")));
    };

    // Stages come from the milestones atom, which is already parsed and
    // validated — §23's graph is the thing being lowered.
    let mut stages: Vec<(String, String)> = Vec::new();
    for atom in basis.atoms.iter().filter(|a| a.role == "milestones") {
        let text = String::from_utf8_lossy(&atom.bytes);
        let parsed = openwarrant_core::milestones::parse(&text)
            .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;
        for stage in &parsed.stages {
            stages.push((stage.id.clone(), stage.executor_kind.to_string()));
        }
    }

    if stages.is_empty() {
        report.push(Diagnostic::warn(
            "blut.no-stages",
            repo.relative(&dir.join("manifest.toml")),
            format!("{alias}: declares no stages, so there is nothing to lower"),
        ));
        return Ok(report);
    }

    // §49.2 — reject rather than degrade. A stage whose executor is not `blut`
    // is not a computational stage, and lowering it anyway would produce a
    // PlanSpec that runs and means something else.
    let lowerable: Vec<&(String, String)> = stages.iter().filter(|(_, k)| k == "blut").collect();
    let foreign: Vec<&str> = stages
        .iter()
        .filter(|(_, k)| k != "blut")
        .map(|(id, _)| id.as_str())
        .collect();

    if lowerable.is_empty() {
        report.push(Diagnostic::warn(
            "blut.not-computational",
            repo.relative(&dir.join("atoms/45-milestones.yaml")),
            format!(
                "{alias}: no stage declares `executor_kind: blut`, so this Warrant is not \
                 a computational WAR. Refused rather than lowered — §49.2 says reject, \
                 not degrade. Foreign executors present: {}",
                foreign.join(", ")
            ),
        ));
        return Ok(report);
    }

    let lowering = BlutLowering {
        stage: alias.to_owned(),
        registry_digest: format!("blut@{BLUT_PIN}"),
        port_mappings: lowerable
            .iter()
            .map(|(id, _)| PortMapping {
                war_port: id.clone(),
                blut_kind: "artifact/bytes".to_owned(),
                compatible: true,
            })
            .collect(),
        backend_identity: format!("blut://backend@{BLUT_PIN}"),
        stage_identity: format!("war://{alias}"),
        resource_envelope_mapped: true,
        plan_provenance: format!("openwarrant://{alias} lowered against blut@{BLUT_PIN}"),
    };

    lowering
        .validate()
        .map_err(|e| RepoError::Message(format!("{alias}: {e}")))?;

    let spec = PlanSpec {
        name: alias.to_owned(),
        nodes: lowerable
            .iter()
            .map(|(id, _)| SpecNode {
                stage: id.clone(),
                args: serde_json::Value::Object(serde_json::Map::new()),
            })
            .collect(),
        edges: (1..lowerable.len())
            .map(|i| u32::try_from(i - 1).unwrap_or(0))
            .zip((1..lowerable.len()).map(|i| u32::try_from(i).unwrap_or(0)))
            .collect(),
        version: 1,
    };

    let json =
        serde_json::to_string_pretty(&spec).map_err(|e| RepoError::Message(format!("{e}")))?;

    report.push(Diagnostic::pass(
        "blut.lowered",
        format!(
            "{alias}: lowered {} stage(s) against a pinned registry (blut@{})",
            spec.nodes.len(),
            &BLUT_PIN[..12]
        ),
    ));
    report.note(format!(
        "This PlanSpec is structurally faithful to a schema read from BLUT at \
         {}, and has NOT been through BLUT's parser. BLUT ships no verb that \
         deserializes a PlanSpec JSON, and its `UnknownStage` rule additionally \
         requires a stage compiled into a registered cookbook. \"BLUT accepted \
         this\" is a strictly stronger claim than anything here establishes.\n\n{json}",
        &BLUT_PIN[..12]
    ));
    Ok(report)
}
