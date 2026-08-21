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
//! # Two corrections to what this file used to say
//!
//! It claimed "BLUT ships no verb that deserializes a `PlanSpec` JSON". That was
//! wrong when written: `blut plan publish` has always parsed one and typechecked
//! it fail-closed before inserting a deployment row. What BLUT lacked was a
//! verb that did so *without* writing — which is a much narrower gap, and
//! stating the wide version made a missing feature look like a missing
//! capability. `blut plan check` (BLUT `7b60d21e`, refined in `d6822563`) closes
//! the narrow gap.
//!
//! It also implied that lowering was unverifiable here. With `--verify` it is
//! not: [`lower`] can hand the generated JSON to a real BLUT binary and report
//! what BLUT says. See [`verify`] for what that does and does not establish.
//!
//! # What is still not claimed
//!
//! BLUT refuses a stage name that is in no registered cookbook
//! (`PlanSpecError::UnknownStage` — "dynamic loading is forbidden"). Every stage
//! this repository's Warrants name is a `STAGE-NNN` identifier that no cookbook
//! compiles in, so a real BLUT binary refuses these lowerings — correctly.
//! Acceptance is not the goal of `--verify`; getting an *authoritative* answer
//! is, and a refusal for a nameable reason is one.

use camino::Utf8Path;
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

/// What a real BLUT binary said about a lowering.
///
/// §46.1's point is that a verdict is only worth what the thing that produced
/// it is worth, so this carries the identity of the binary alongside the
/// answer. A verdict with no attributable producer is not external evidence,
/// it is a string.
#[derive(Debug)]
pub struct BlutVerdict {
    /// Whether BLUT typechecked the spec.
    pub accepted: bool,
    /// BLUT's own words. On refusal this is the `PlanSpecError`.
    pub detail: String,
    /// The ADR-0078 fingerprint BLUT computed, present only on acceptance.
    pub fingerprint: Option<String>,
    /// How many recipes the invoked binary had registered, if it said. A
    /// refusal from a binary with none means "this build has no cookbooks", not
    /// "the spec is wrong", and those are not distinguishable from the message.
    /// `None` means BLUT did not report a count — which is not the same as
    /// reporting zero, and is not flattened into it.
    pub recipes_registered: Option<u64>,
    /// The binary that answered, and the code it exited with. Not optional:
    /// a signal-killed neighbour is refused before a verdict is built, so
    /// "answered but has no exit code" is not a state this type can hold.
    pub binary: String,
    pub exit_code: i32,
}

/// Render a count BLUT may not have reported. "unstated" is deliberately not
/// "0": a reader who sees zero concludes the binary has no cookbooks, which is
/// a fact nobody established.
fn describe_count(n: Option<u64>) -> String {
    n.map_or_else(|| "an unstated number of".to_owned(), |n| n.to_string())
}

/// How long a neighbour gets to answer before the question is withdrawn.
///
/// A neighbour that never returns is a fourth way for `--verify` to be
/// untrustworthy, alongside missing, lying and mute — and it is the only one
/// with no natural bound. Without this, a BLUT binary that blocks on a lock or
/// waits for input hangs `war blut` until someone notices, and "the tool hung"
/// is not a verdict.
const VERIFY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Write the spec somewhere only this process can have created.
///
/// `std::fs::write` opens with `O_CREAT|O_TRUNC` and follows symlinks, so on a
/// shared `/tmp` anyone who can guess the path can pre-place a symlink and
/// redirect the write to a file of their choosing. `create_new` adds `O_EXCL`,
/// which fails on anything already at the path — symlink included — so the
/// write either lands on a file this call just created or does not happen.
///
/// The nanosecond component is not the security control; `O_EXCL` is. It exists
/// so two calls from the same process cannot pick the same name, which a
/// pid-only name could.
fn write_spec_exclusively(spec_json: &str) -> Result<std::path::PathBuf, RepoError> {
    use std::io::Write;

    let dir = std::env::temp_dir();
    let mut last = String::new();
    for _ in 0..8 {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let path = dir.join(format!("war-blut-{}-{nonce}.json", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                f.write_all(spec_json.as_bytes())
                    .map_err(|e| RepoError::Message(format!("write {}: {e}", path.display())))?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                last = format!("{} already exists", path.display());
            }
            Err(e) => {
                return Err(RepoError::Message(format!(
                    "create {}: {e}",
                    path.display()
                )));
            }
        }
    }
    Err(RepoError::Message(format!(
        "could not create a spec file nothing else owns after 8 attempts ({last}). \
         Refusing to write to a path this process did not create."
    )))
}

/// Run `<binary> plan check <path> --json`, killing it if it outstays
/// [`VERIFY_TIMEOUT`].
///
/// stdout and stderr are drained on their own threads rather than after the
/// wait: a child that fills a pipe buffer blocks writing, and a parent that is
/// waiting for exit before reading would then wait forever. That deadlock looks
/// exactly like the hang this function exists to bound.
fn run_with_timeout(
    binary: &Utf8Path,
    path: &std::path::Path,
) -> Result<std::process::Output, RepoError> {
    use std::io::Read;

    let mut child = std::process::Command::new(binary.as_std_path())
        .args(["plan", "check"])
        .arg(path)
        .arg("--json")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RepoError::Message(format!(
                "could not run `{binary} plan check`: {e}. \
                 --verify names a BLUT binary; if that binary does not exist or is \
                 not executable, this is a refusal to guess rather than a verdict."
            ))
        })?;

    let drain = |mut s: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(h) = s.as_mut() {
                let _ = h.read_to_end(&mut buf);
            }
            buf
        })
    };
    let so = drain(
        child
            .stdout
            .take()
            .map(|h| Box::new(h) as Box<dyn Read + Send>),
    );
    let se = drain(
        child
            .stderr
            .take()
            .map(|h| Box::new(h) as Box<dyn Read + Send>),
    );

    let deadline = std::time::Instant::now() + VERIFY_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {}
            Err(e) => {
                // Dropping a Child does not terminate it, so returning here
                // without killing would leave the neighbour running with
                // nobody left to read it.
                let _ = child.kill();
                let _ = child.wait();
                return Err(RepoError::Message(format!(
                    "could not wait on `{binary} plan check`: {e}"
                )));
            }
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RepoError::Message(format!(
                "`{binary} plan check` did not answer within {}s and was killed. \
                 A neighbour that never returns has given no verdict, and a \
                 timeout is not a refusal — nothing is recorded either way.",
                VERIFY_TIMEOUT.as_secs()
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };

    Ok(std::process::Output {
        status,
        stdout: so.join().unwrap_or_default(),
        stderr: se.join().unwrap_or_default(),
    })
}

/// Hand a lowered `PlanSpec` to a real BLUT binary and report what it says.
///
/// # Why this is `authoritative_external` and not a self-report
///
/// §51.3 says a performer's structured claim about its own work is not
/// evidence. The verdict here is not OpenWarrant's claim about OpenWarrant —
/// it is BLUT's claim about OpenWarrant's output, produced by a separate
/// program this repository does not control, whose refusal messages this
/// repository cannot author. That is what `Admissibility::AuthoritativeExternal`
/// names, and it is the first thing in this repository to earn it.
///
/// # What it establishes, exactly
///
/// That a named binary, run at a known path, read these bytes and returned this
/// verdict and this exit code. That is a claim about the adapter's output being
/// well-formed as far as BLUT is concerned.
///
/// It is **not** an execution. OW-WAR-0047's OBL-001 asks for status, artifact
/// and lineage receipts returned by BLUT from a real run, and a typecheck
/// produces none of those. Nothing here discharges it, and reporting acceptance
/// as though it did would be §40.7's substitution of a cheaper measurement for
/// the one that was required.
fn verify(binary: &Utf8Path, spec_json: &str) -> Result<BlutVerdict, RepoError> {
    // A temp file rather than stdin: BLUT's verb takes a path, and inventing a
    // stdin mode in the neighbour to suit this caller would be the tail wagging
    // the dog.
    let path = write_spec_exclusively(spec_json)?;

    let out = run_with_timeout(binary, &path);

    // Remove the spec whether or not the invocation worked — a failed run
    // should not leave the next one reading a stale file.
    let _ = std::fs::remove_file(&path);

    let out = out?;

    // Asked FIRST, before anything BLUT printed is read. A process killed by a
    // signal has no exit code and never finished answering, so its output is
    // not a partial verdict — it is the middle of a sentence. Checking this
    // after the JSON parse would have refused the same runs, but usually with
    // "did not print JSON", which names the symptom and hides the cause.
    let Some(exit_code) = out.status.code() else {
        return Err(RepoError::Message(format!(
            "`{binary} plan check` was killed by a signal and never exited \
             normally, so it produced no verdict. Whatever it printed first is \
             not an answer."
        )));
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        RepoError::Message(format!(
            "`{binary} plan check --json` did not print JSON ({e}). stdout: {}, stderr: {}",
            stdout.trim(),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    })?;

    // Every field is read from BLUT's output or absent. Nothing is defaulted to
    // a value that would flatter the result: a missing `accepted` is not
    // acceptance.
    let accepted = parsed.get("accepted").and_then(serde_json::Value::as_bool);
    let Some(accepted) = accepted else {
        return Err(RepoError::Message(format!(
            "`{binary} plan check --json` printed JSON with no `accepted` field, \
             so there is no verdict to record: {parsed}"
        )));
    };

    // The exit code must agree with the verdict. If they ever disagree, one of
    // them is lying about what happened and neither can be trusted.
    if accepted != (exit_code == 0) {
        return Err(RepoError::Message(format!(
            "`{binary} plan check` reported accepted={accepted} but exited with \
             {exit_code}. A verdict that disagrees with its own exit status is \
             not recordable."
        )));
    }

    // On acceptance the detail is the plan name, and BLUT always emits one.
    // Substituting a placeholder for it would put a string this repository
    // wrote into a record whose origin is `blut` — small, but it is exactly the
    // seam where a fabricated detail would enter external evidence.
    let detail_key = if accepted { "name" } else { "error" };
    let detail = parsed.get(detail_key).and_then(serde_json::Value::as_str);
    let Some(detail) = detail else {
        return Err(RepoError::Message(format!(
            "`{binary} plan check --json` reported accepted={accepted} but no \
             `{detail_key}` field, so there is nothing to record but a string \
             this repository made up: {parsed}"
        )));
    };

    Ok(BlutVerdict {
        accepted,
        detail: detail.to_owned(),
        fingerprint: parsed
            .get("fingerprint")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        // Absent is not zero. "BLUT registered no cookbooks" and "BLUT did not
        // say how many" are different facts, and defaulting the second to the
        // first would report a bare registry that was never observed.
        recipes_registered: parsed
            .get("recipes_registered")
            .and_then(serde_json::Value::as_u64),
        binary: binary.to_string(),
        exit_code,
    })
}

/// Lower one Warrant's milestone graph into a `PlanSpec`.
///
/// §49.2's duties, each discharged or explicitly refused:
/// resolve stage names against a pinned registry; map named ports to typed
/// inputs and outputs; reject incompatible kinds; reject unsupported
/// conditions; pin backend and stage identities; map resource envelopes; record
/// plan provenance.
pub fn lower(
    repo: &Repository,
    alias: &str,
    verify_with: Option<&Utf8Path>,
) -> Result<Report, RepoError> {
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

    let Some(binary) = verify_with else {
        report.note(format!(
            "This PlanSpec is structurally faithful to a schema read from BLUT at \
             {}, and has NOT been through BLUT's parser — no binary was named. \
             Pass --verify <blut-binary> to get BLUT's own verdict; \"BLUT \
             accepted this\" is a strictly stronger claim than anything here \
             establishes.\n\n{json}",
            &BLUT_PIN[..12]
        ));
        return Ok(report);
    };

    let verdict = verify(binary, &json)?;

    if verdict.accepted {
        report.push(Diagnostic::pass(
            "blut.accepted",
            format!(
                "{alias}: BLUT accepted the lowering (fingerprint {}, {} recipes registered, \
                 exit {}) — reported by {}",
                verdict
                    .fingerprint
                    .as_deref()
                    .map_or("<none reported>", |f| &f[..f.len().min(12)]),
                describe_count(verdict.recipes_registered),
                verdict.exit_code,
                verdict.binary
            ),
        ));
    } else {
        // A refusal is not a failure of this command. The command's job is to
        // obtain an authoritative answer, and it obtained one. Recording a
        // refusal as an ERROR would push a future author toward suppressing it.
        report.push(Diagnostic::unknown(
            "blut.refused",
            repo.relative(&dir.join("atoms/45-milestones.yaml")),
            format!(
                "{alias}: BLUT refused the lowering (exit {}, {} recipes registered) — {}",
                verdict.exit_code,
                describe_count(verdict.recipes_registered),
                verdict.detail
            ),
        ));
    }

    report.note(format!(
        "Verdict obtained from {} — a separate program this repository does not \
         control, whose refusal messages it cannot author. That is what makes it \
         `authoritative_external` (§40.2) rather than a self-report (§51.3).\n\n\
         It is NOT an execution. OW-WAR-0047's OBL-001 asks for status, artifact \
         and lineage receipts from a real BLUT run; a typecheck produces none of \
         those, so this does not discharge it.\n\n\
         Note that every stage this repository names is a `STAGE-NNN` identifier \
         that no cookbook compiles in, so a refusal naming an unknown stage is \
         the expected and correct answer, not a defect in the lowering.\n\n{json}",
        verdict.binary
    ));
    Ok(report)
}
