// SPDX-License-Identifier: AGPL-3.0-or-later
//! Receipt-first Gate evidence admission.
//!
//! A raw [`GateRun`] is history, not admissible evidence. This module starts
//! from a receipt commit marker, derives every immutable sibling object from
//! its unique run identity, verifies the complete bundle, then returns an
//! [`AdmissibleGateRun`]. Resolution cannot construct that witness itself.

use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{DigestDomain, sha256_digest, sha256_hex};
use openwarrant_core::authority::{ActorRole, AuthorityRegister};
use openwarrant_core::gate::{GateBinding, GateDefinition};
use openwarrant_core::{GateReceipt, GateRun, ReasonCode, TestSelectionObservation, Verdict};

use crate::authorize::AuthorizedGateBinding;
use crate::gate_adapter::GateRecordingAdapter;
use crate::gate_cmd::canonical_run_id;
use crate::repo::{Repository, read_repository_regular_bounded};

const MAX_GATE_EVIDENCE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_GATE_RECEIPT_BYTES: u64 = 1024 * 1024;
const MAX_GATE_BUNDLES: usize = 4096;

/// Proof that one passing Gate Run survived complete receipt-bundle admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AdmissibleGateRun {
    run: GateRun,
}

impl AdmissibleGateRun {
    #[must_use]
    pub(crate) fn id(&self) -> &str {
        &self.run.id
    }

    #[must_use]
    pub(crate) fn gate(&self) -> &str {
        &self.run.gate
    }
}

/// Valid runs plus fail-closed diagnostics for rejected candidate bundles.
#[derive(Debug, Default)]
pub(crate) struct GateEvidenceSet {
    pub(crate) runs: Vec<AdmissibleGateRun>,
    pub(crate) failures: Vec<(String, String)>,
}

pub(crate) struct ObservedStreams<'a> {
    pub(crate) gate_run_digest: &'a str,
    pub(crate) selection_ref: &'a str,
    pub(crate) selection_digest: &'a str,
    pub(crate) selection: &'a TestSelectionObservation,
    pub(crate) stdout_ref: &'a str,
    pub(crate) stdout_digest: &'a str,
    pub(crate) stderr_ref: &'a str,
    pub(crate) stderr_digest: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolutionEvidenceArtifact {
    pub(crate) deliverable_id: String,
    pub(crate) target_ref: String,
    pub(crate) observed_sha256: String,
}

/// Contract-bound subjects and current artifact identities available to a
/// resolver. One Gate Binding may intentionally cover only a subset of these
/// artifacts; §56.1 verifies the complete deliverable inventory separately.
#[derive(Debug, Clone)]
pub(crate) struct ResolutionEvidenceBasis {
    contract_subject: String,
    allowed_subjects: BTreeSet<String>,
    artifacts_by_id: BTreeMap<String, ResolutionEvidenceArtifact>,
    authorized_bindings_by_gate: BTreeMap<String, AuthorizedGateBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResolutionRawEvidence {
    Artifact { deliverable_id: String },
    File { path: Utf8PathBuf, sha256: String },
}

impl ResolutionEvidenceBasis {
    pub(crate) fn new(
        contract_digest: &str,
        artifacts: impl IntoIterator<Item = ResolutionEvidenceArtifact>,
        authorized_bindings: impl IntoIterator<Item = AuthorizedGateBinding>,
    ) -> Result<Self, String> {
        let contract_hex = contract_digest
            .strip_prefix("contract:sha256:")
            .or_else(|| contract_digest.strip_prefix("sha256:"))
            .unwrap_or(contract_digest);
        if !canonical_digest_hex(contract_hex) {
            return Err("Resolution contract digest is not 64 lowercase hex characters".to_owned());
        }
        let contract_subject = format!("contract:sha256:{contract_hex}");
        let mut allowed_subjects = BTreeSet::from([contract_subject.clone()]);
        let mut artifacts_by_id = BTreeMap::new();
        for artifact in artifacts {
            if artifact.deliverable_id.trim().is_empty()
                || !safe_relative_reference(&artifact.target_ref)
                || !canonical_sha256(&artifact.observed_sha256)
            {
                return Err(format!(
                    "Resolution artifact {:?} has a blank id, unsafe target, or invalid digest",
                    artifact.deliverable_id
                ));
            }
            allowed_subjects.insert(artifact.observed_sha256.clone());
            let id = artifact.deliverable_id.clone();
            if artifacts_by_id.insert(id.clone(), artifact).is_some() {
                return Err(format!(
                    "Resolution artifact id {id:?} is declared more than once"
                ));
            }
        }
        let mut authorized_bindings_by_gate = BTreeMap::new();
        let mut binding_ids = BTreeSet::new();
        for binding in authorized_bindings {
            if binding.id.trim().is_empty()
                || binding.id.trim() != binding.id
                || !canonical_gate_key(&binding.gate)
                || !canonical_sha256(&binding.digest)
                || !safe_relative_reference(&binding.source_ref)
            {
                return Err(format!(
                    "Authorized Gate Binding {:?} has noncanonical identity",
                    binding.id
                ));
            }
            if !binding_ids.insert(binding.id.clone()) {
                return Err(format!(
                    "Authorized Gate Binding id {:?} is declared more than once",
                    binding.id
                ));
            }
            let gate = binding.gate.clone();
            if authorized_bindings_by_gate
                .insert(gate.clone(), binding)
                .is_some()
            {
                return Err(format!(
                    "Contract authorization selects more than one Gate Binding for {gate}"
                ));
            }
        }
        Ok(Self {
            contract_subject,
            allowed_subjects,
            artifacts_by_id,
            authorized_bindings_by_gate,
        })
    }

    fn authorizes_binding(&self, binding: &GateBinding, receipt: &GateReceipt) -> bool {
        self.authorized_bindings_by_gate
            .get(&binding.gate.key())
            .is_some_and(|authorized| {
                authorized.id == binding.id && authorized.digest == receipt.gate_binding_digest
            })
    }

    fn admits_subjects(&self, subjects: &BTreeSet<&str>) -> bool {
        subjects.contains(self.contract_subject.as_str())
            && subjects
                .iter()
                .all(|subject| self.allowed_subjects.contains(*subject))
    }

    fn raw_artifacts_are_subject_bound(
        &self,
        subjects: &BTreeSet<&str>,
        raw_evidence: &BTreeSet<&str>,
    ) -> bool {
        raw_evidence.iter().all(|reference| {
            reference
                .strip_prefix("artifact://")
                .is_none_or(|deliverable_id| {
                    self.artifacts_by_id
                        .get(deliverable_id)
                        .is_some_and(|artifact| {
                            subjects.contains(artifact.observed_sha256.as_str())
                        })
                })
        })
    }

    pub(crate) fn classify_raw_evidence(
        &self,
        reference: &str,
    ) -> Result<ResolutionRawEvidence, String> {
        if let Some(deliverable_id) = reference.strip_prefix("artifact://") {
            if deliverable_id.is_empty() || !self.artifacts_by_id.contains_key(deliverable_id) {
                return Err(format!(
                    "raw evidence {reference:?} does not name a current Resolution artifact"
                ));
            }
            return Ok(ResolutionRawEvidence::Artifact {
                deliverable_id: deliverable_id.to_owned(),
            });
        }
        let Some((path, digest)) = reference
            .strip_prefix("file:")
            .and_then(|value| value.rsplit_once("#sha256:"))
        else {
            return Err(format!(
                "raw evidence {reference:?} is not artifact://<id> or file:<path>#sha256:<digest>"
            ));
        };
        if !safe_relative_reference(path) || !canonical_digest_hex(digest) {
            return Err(format!(
                "raw evidence {reference:?} has an unsafe path or invalid digest"
            ));
        }
        Ok(ResolutionRawEvidence::File {
            path: Utf8PathBuf::from(path),
            sha256: format!("sha256:{digest}"),
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) enum SubjectBasis<'a> {
    Exact(&'a BTreeSet<String>),
    Resolution(&'a ResolutionEvidenceBasis),
}

fn canonical_digest_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(canonical_digest_hex)
}

fn safe_relative_reference(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

#[cfg(test)]
pub(crate) fn admitted_for_test(run: GateRun) -> AdmissibleGateRun {
    assert!(run.validate().is_ok(), "test Gate Run must validate");
    assert!(
        run.satisfies_required_pass() && run.reason_code == Some(ReasonCode::Passed),
        "test admission witness may represent only a completed passing Gate Run"
    );
    AdmissibleGateRun { run }
}

fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

fn canonical_gate_key(key: &str) -> bool {
    let Some((id, version)) = key.split_once('@') else {
        return false;
    };
    !id.is_empty()
        && !version.is_empty()
        && !key.contains('/')
        && !key.contains('\\')
        && !key.contains("..")
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'@'))
}

fn exact_set<'a>(values: &'a [String], label: &str) -> Result<BTreeSet<&'a str>, String> {
    let set: BTreeSet<&str> = values.iter().map(String::as_str).collect();
    if set.len() != values.len() || set.iter().any(|value| value.trim().is_empty()) {
        return Err(format!("{label} contains a duplicate or blank value"));
    }
    Ok(set)
}

/// Validate semantic relationships after caller has opened and digested bytes.
///
/// This function owns no path traversal. Live and pinned-snapshot loaders can
/// therefore share the same admission boundary without sharing storage code.
#[allow(clippy::too_many_arguments)]
pub(crate) fn admit_components(
    run: GateRun,
    definition: &GateDefinition,
    binding: &GateBinding,
    receipt: &GateReceipt,
    streams: ObservedStreams<'_>,
    subject_basis: SubjectBasis<'_>,
    expected_raw_evidence: &BTreeSet<String>,
    authority: &AuthorityRegister,
    performer: &str,
) -> Result<AdmissibleGateRun, String> {
    run.validate()
        .map_err(|error| format!("invalid Gate Run: {error}"))?;
    definition
        .validate()
        .map_err(|error| format!("invalid Gate Definition: {error}"))?;
    receipt
        .validate()
        .map_err(|error| format!("invalid Gate receipt: {error}"))?;
    if !canonical_run_id(&run.id) || receipt.run_id != run.id {
        return Err("Gate receipt does not bind one canonical run identity".to_owned());
    }
    if !run.satisfies_required_pass()
        || run.reason_code != Some(ReasonCode::Passed)
        || receipt.verdict != Verdict::Pass
        || !receipt.exit_result.passed()
        || receipt.gate_run_digest != streams.gate_run_digest
    {
        return Err("Gate evidence does not encode one completed passing result".to_owned());
    }
    if !canonical_gate_key(&run.gate)
        || definition.key() != run.gate
        || !definition.lifecycle.is_bindable()
        || definition.mutating
        || definition.digest != receipt.gate_definition_digest
        || binding.gate.key() != run.gate
        || binding.gate.digest != definition.digest
        || receipt.arguments != definition.argv
    {
        return Err(
            "Gate evidence does not bind the exact nonmutating qualified definition".to_owned(),
        );
    }
    if !binding.parameters.is_empty() || !binding.pass_predicate.is_empty() {
        return Err(
            "Gate Binding uses parameters or pass predicates unsupported by this runner".to_owned(),
        );
    }
    if !binding.fixtures.is_empty() {
        return Err(
            "Gate Binding uses fixtures unsupported by this runner's execution adapter".to_owned(),
        );
    }
    let binding_digest = sha256_digest(DigestDomain::GateBinding, binding)
        .map(|digest| format!("sha256:{digest}"))
        .map_err(|error| format!("cannot canonicalize Gate Binding: {error}"))?;
    if binding_digest != receipt.gate_binding_digest {
        return Err("Gate receipt names a different Gate Binding digest".to_owned());
    }
    let recorded_receipt_digest = receipt.receipt_digest.clone();
    let mut receipt_preimage = receipt.clone();
    receipt_preimage.receipt_digest.clear();
    let receipt_digest = sha256_digest(DigestDomain::GateReceipt, &receipt_preimage)
        .map(|digest| format!("sha256:{digest}"))
        .map_err(|error| format!("cannot canonicalize Gate receipt: {error}"))?;
    if receipt_digest != recorded_receipt_digest {
        return Err("Gate receipt has an invalid canonical digest".to_owned());
    }

    let binding_subjects = exact_set(&binding.subjects, "Gate Binding subjects")?;
    let receipt_subjects = exact_set(&receipt.subject_digests, "Gate receipt subjects")?;
    if binding_subjects != receipt_subjects {
        return Err("Gate receipt subject inventory does not match its Binding".to_owned());
    }
    let subjects_admitted = match subject_basis {
        SubjectBasis::Exact(expected) => {
            let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
            binding_subjects == expected
        }
        SubjectBasis::Resolution(basis) => basis.admits_subjects(&binding_subjects),
    };
    if !subjects_admitted {
        return Err("Gate evidence subject inventory does not match Resolution basis".to_owned());
    }
    if let SubjectBasis::Resolution(basis) = subject_basis
        && !basis.authorizes_binding(binding, receipt)
    {
        return Err("Gate Binding is not authorized for the current Contract Revision".to_owned());
    }
    let binding_fixture_digests: Vec<String> = binding
        .fixtures
        .iter()
        .map(|fixture| fixture.digest.clone())
        .collect();
    let binding_fixtures = exact_set(&binding_fixture_digests, "Gate Binding fixture digests")?;
    let receipt_fixtures = exact_set(&receipt.fixture_digests, "Gate receipt fixture digests")?;
    if binding_fixtures != receipt_fixtures {
        return Err("Gate receipt fixture inventory does not match its Binding".to_owned());
    }
    let raw_evidence = exact_set(&receipt.raw_evidence_refs, "Gate receipt raw evidence")?;
    let adapter = GateRecordingAdapter::for_definition(definition)?;
    adapter.validate_raw_evidence(&receipt.raw_evidence_refs)?;
    let expected_raw_evidence: BTreeSet<&str> =
        expected_raw_evidence.iter().map(String::as_str).collect();
    if raw_evidence != expected_raw_evidence {
        return Err("Gate receipt raw evidence does not match Resolution artifacts".to_owned());
    }
    if let SubjectBasis::Resolution(basis) = subject_basis
        && !basis.raw_artifacts_are_subject_bound(&binding_subjects, &raw_evidence)
    {
        return Err(
            "Gate receipt artifact evidence is not bound to its current content digest subject"
                .to_owned(),
        );
    }
    streams
        .selection
        .validate()
        .map_err(|error| format!("invalid test-selection observation: {error}"))?;
    let selected_tests = exact_set(&receipt.selected_test_manifest, "selected-test manifest")?;
    if selected_tests.is_empty()
        || u64::try_from(selected_tests.len()).ok() != Some(receipt.selected_test_count)
        || receipt.selected_test_manifest != definition.selection_manifest
        || streams.selection.run_id != run.id
        || streams.selection.gate_definition_digest != definition.digest
        || streams.selection.adapter != adapter.id()
        || streams.selection.selected_test_count != receipt.selected_test_count
        || streams.selection.selected_test_manifest != receipt.selected_test_manifest
        || receipt.runner != adapter.id()
        || receipt.selection_observation_ref != streams.selection_ref
        || receipt.selection_observation_digest != streams.selection_digest
    {
        return Err("Gate receipt does not bind exact adapter-observed test selection".to_owned());
    }
    if receipt.stdout_ref != streams.stdout_ref
        || receipt.stdout_digest != streams.stdout_digest
        || receipt.stderr_ref != streams.stderr_ref
        || receipt.stderr_digest != streams.stderr_digest
    {
        return Err("Gate receipt does not bind exact output references and bytes".to_owned());
    }
    if receipt.producer_actor != binding.evidence_policy.producer {
        return Err("Gate receipt producer does not match its Gate Binding producer".to_owned());
    }
    if receipt.producer_actor == performer
        && !binding.evidence_policy.performer_authored_report_admissible
    {
        return Err("Gate Binding forbids performer-authored evidence".to_owned());
    }
    if !openwarrant_core::legacy_disposition::is_canonical_utc(&receipt.started_at)
        || !openwarrant_core::legacy_disposition::is_canonical_utc(&receipt.completed_at)
        || receipt.started_at > receipt.completed_at
    {
        return Err("Gate receipt chronology is not canonical and monotonic".to_owned());
    }
    let producer = authority.actor(&receipt.producer_actor).ok_or_else(|| {
        format!(
            "Gate receipt producer {:?} has no authority assignment",
            receipt.producer_actor
        )
    })?;
    if producer.validate().is_err()
        || !producer.holds(ActorRole::Verifier)
        || !openwarrant_core::legacy_disposition::is_canonical_utc(&producer.effective_time)
        || producer.effective_time > receipt.started_at
    {
        return Err(format!(
            "Gate receipt producer {:?} lacked effective Verifier authority",
            receipt.producer_actor
        ));
    }

    Ok(AdmissibleGateRun { run })
}

fn gate_definition_path(repo: &Repository, gate: &str) -> Result<Utf8PathBuf, String> {
    if !canonical_gate_key(gate) {
        return Err(format!("Gate key {gate:?} is not canonical"));
    }
    let root = repo.root.join(&repo.config.paths.gates);
    let yaml = root.join(format!("{gate}.yaml"));
    let yml = root.join(format!("{gate}.yml"));
    match (yaml.exists(), yml.exists()) {
        (true, false) => Ok(yaml),
        (false, true) => Ok(yml),
        (true, true) => Err(format!(
            "Gate {gate:?} has ambiguous .yaml and .yml definitions"
        )),
        (false, false) => Err(format!("Gate {gate:?} has no exact definition object")),
    }
}

fn read_bounded(
    repo: &Repository,
    path: &Utf8Path,
    label: &str,
    limit: u64,
) -> Result<Vec<u8>, String> {
    read_repository_regular_bounded(path, &repo.root, label, limit)
        .map_err(|error| error.to_string())
}

fn verify_live_raw_evidence(
    repo: &Repository,
    references: &[String],
    basis: &ResolutionEvidenceBasis,
) -> Result<BTreeSet<String>, String> {
    exact_set(references, "Gate receipt raw evidence")?;
    let mut verified = BTreeSet::new();
    for reference in references {
        match basis.classify_raw_evidence(reference)? {
            ResolutionRawEvidence::Artifact { .. } => {}
            ResolutionRawEvidence::File { path, sha256 } => {
                let bytes = read_bounded(
                    repo,
                    &path,
                    "Gate raw evidence",
                    MAX_GATE_EVIDENCE_FILE_BYTES,
                )?;
                if content_digest(&bytes) != sha256 {
                    return Err(format!(
                        "Gate raw evidence {reference:?} digest does not match current bytes"
                    ));
                }
            }
        }
        verified.insert(reference.clone());
    }
    Ok(verified)
}

#[derive(Clone, Copy)]
enum LoadBasis<'a> {
    #[cfg(test)]
    Exact {
        subjects: &'a BTreeSet<String>,
        raw_evidence: &'a BTreeSet<String>,
    },
    Resolution(&'a ResolutionEvidenceBasis),
}

fn load_one(
    repo: &Repository,
    receipt_path: &Utf8Path,
    basis: LoadBasis<'_>,
    authority: &AuthorityRegister,
) -> Result<AdmissibleGateRun, String> {
    let receipt_bytes = read_bounded(repo, receipt_path, "Gate receipt", MAX_GATE_RECEIPT_BYTES)?;
    let receipt: GateReceipt =
        openwarrant_core::legacy_disposition::parse_strict_json(&receipt_bytes)
            .map_err(|error| format!("cannot parse Gate receipt: {error}"))?;
    let verified_raw_evidence;
    let (subject_basis, expected_raw_evidence) = match basis {
        #[cfg(test)]
        LoadBasis::Exact {
            subjects,
            raw_evidence,
        } => (SubjectBasis::Exact(subjects), raw_evidence),
        LoadBasis::Resolution(resolution_basis) => {
            verified_raw_evidence =
                verify_live_raw_evidence(repo, &receipt.raw_evidence_refs, resolution_basis)?;
            (
                SubjectBasis::Resolution(resolution_basis),
                &verified_raw_evidence,
            )
        }
    };
    if !canonical_run_id(&receipt.run_id) {
        return Err(format!(
            "Gate receipt run id {:?} is not canonical",
            receipt.run_id
        ));
    }
    let dir = repo.root.join(&repo.config.paths.receipts);
    let expected_receipt_path = dir.join(format!("{}.receipt.json", receipt.run_id));
    if receipt_path != expected_receipt_path {
        return Err(format!(
            "Gate receipt path must derive exactly from run id {:?}",
            receipt.run_id
        ));
    }
    let run_path = dir.join(format!("{}.run.toml", receipt.run_id));
    let binding_path = dir.join(format!("{}.binding.json", receipt.run_id));
    let selection_path = dir.join(format!("{}.selection.json", receipt.run_id));
    let stdout_path = dir.join(format!("{}.stdout.txt", receipt.run_id));
    let stderr_path = dir.join(format!("{}.stderr.txt", receipt.run_id));

    let run_bytes = read_bounded(repo, &run_path, "Gate Run", MAX_GATE_RECEIPT_BYTES)?;
    let run_text =
        std::str::from_utf8(&run_bytes).map_err(|_| "Gate Run is not UTF-8".to_owned())?;
    let run: GateRun =
        toml::from_str(run_text).map_err(|error| format!("cannot parse Gate Run: {error}"))?;
    if run.id != receipt.run_id {
        return Err("Gate Run identity does not match its receipt".to_owned());
    }
    let binding_bytes = read_bounded(repo, &binding_path, "Gate Binding", MAX_GATE_RECEIPT_BYTES)?;
    let binding: GateBinding =
        openwarrant_core::legacy_disposition::parse_strict_json(&binding_bytes)
            .map_err(|error| format!("cannot parse Gate Binding: {error}"))?;
    let definition_path = gate_definition_path(repo, &run.gate)?;
    let definition_bytes = read_bounded(
        repo,
        &definition_path,
        "Gate Definition",
        MAX_GATE_RECEIPT_BYTES,
    )?;
    let definition_text = std::str::from_utf8(&definition_bytes)
        .map_err(|_| "Gate Definition is not UTF-8".to_owned())?;
    let document = openwarrant_core::structured::parse(definition_text)
        .map_err(|error| format!("cannot parse Gate Definition: {error}"))?;
    let mut definition = openwarrant_core::gate::definition_from_structured(&document)
        .map_err(|error| format!("invalid Gate Definition: {error}"))?;
    definition
        .bind_local_source_digest(&content_digest(&definition_bytes))
        .map_err(|error| format!("invalid Gate Definition identity: {error}"))?;

    let selection_bytes = read_bounded(
        repo,
        &selection_path,
        "test-selection observation",
        MAX_GATE_RECEIPT_BYTES,
    )?;
    let selection: TestSelectionObservation =
        openwarrant_core::legacy_disposition::parse_strict_json(&selection_bytes)
            .map_err(|error| format!("cannot parse test-selection observation: {error}"))?;
    let selection_ref = repo.relative(&selection_path);
    let selection_digest = content_digest(&selection_bytes);

    let stdout = read_bounded(
        repo,
        &stdout_path,
        "Gate stdout",
        MAX_GATE_EVIDENCE_FILE_BYTES,
    )?;
    let stderr = read_bounded(
        repo,
        &stderr_path,
        "Gate stderr",
        MAX_GATE_EVIDENCE_FILE_BYTES,
    )?;
    let stdout_ref = repo.relative(&stdout_path);
    let stderr_ref = repo.relative(&stderr_path);
    let stdout_digest = content_digest(&stdout);
    let stderr_digest = content_digest(&stderr);

    let gate_run_digest = content_digest(&run_bytes);
    admit_components(
        run,
        &definition,
        &binding,
        &receipt,
        ObservedStreams {
            gate_run_digest: &gate_run_digest,
            selection_ref: &selection_ref,
            selection_digest: &selection_digest,
            selection: &selection,
            stdout_ref: &stdout_ref,
            stdout_digest: &stdout_digest,
            stderr_ref: &stderr_ref,
            stderr_digest: &stderr_digest,
        },
        subject_basis,
        expected_raw_evidence,
        authority,
        &repo.performer(),
    )
}

fn retain_lexicographically_smallest(sample: &mut Option<String>, candidate: String) {
    if sample
        .as_ref()
        .is_none_or(|current| candidate.as_str() < current.as_str())
    {
        *sample = Some(candidate);
    }
}

fn encoded_file_name(name: &std::ffi::OsStr) -> String {
    use std::fmt::Write as _;

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;

        let bytes = name.as_bytes();
        let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
        for byte in bytes {
            let _ = write!(encoded, "{byte:02x}");
        }
        format!("unix-bytes:{encoded}")
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;

        let mut encoded = String::new();
        for unit in name.encode_wide() {
            let _ = write!(encoded, "{unit:04x}");
        }
        return format!("windows-wide:{encoded}");
    }

    #[cfg(not(any(unix, windows)))]
    {
        format!("platform-debug:{name:?}")
    }
}

fn stable_io_error(error: &std::io::Error) -> String {
    format!(
        "kind={:?}, raw_os_error={:?}",
        error.kind(),
        error.raw_os_error()
    )
}

/// Load only receipt-committed bundles. Bare runs and orphan staging objects
/// are never candidates, even when they encode `askable/completed/pass`.
#[cfg(test)]
pub(crate) fn load_admissible(
    repo: &Repository,
    expected_subjects: &BTreeSet<String>,
    expected_raw_evidence: &BTreeSet<String>,
    authority: &AuthorityRegister,
) -> GateEvidenceSet {
    load_admissible_with_basis(
        repo,
        LoadBasis::Exact {
            subjects: expected_subjects,
            raw_evidence: expected_raw_evidence,
        },
        authority,
    )
}

pub(crate) fn load_admissible_for_resolution(
    repo: &Repository,
    basis: &ResolutionEvidenceBasis,
    authority: &AuthorityRegister,
) -> GateEvidenceSet {
    load_admissible_with_basis(repo, LoadBasis::Resolution(basis), authority)
}

fn load_admissible_with_basis(
    repo: &Repository,
    basis: LoadBasis<'_>,
    authority: &AuthorityRegister,
) -> GateEvidenceSet {
    let dir = repo.root.join(&repo.config.paths.receipts);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return GateEvidenceSet::default();
        }
        Err(error) => {
            return GateEvidenceSet {
                runs: Vec::new(),
                failures: vec![(
                    repo.relative(&dir),
                    format!(
                        "cannot read Gate receipt directory ({})",
                        stable_io_error(&error)
                    ),
                )],
            };
        }
    };
    // `ReadDir` order is unspecified. Keep a bounded lexical prefix while still
    // scanning every entry so late discovery failures cannot disappear.
    let mut paths = BTreeSet::<Utf8PathBuf>::new();
    let mut receipt_count = 0_u64;
    let mut entry_error_count = 0_u64;
    let mut entry_error_sample = None;
    let mut non_utf8_count = 0_u64;
    let mut non_utf8_sample = None;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                entry_error_count = entry_error_count.saturating_add(1);
                retain_lexicographically_smallest(&mut entry_error_sample, stable_io_error(&error));
                continue;
            }
        };
        let path = match Utf8PathBuf::from_path_buf(entry.path()) {
            Ok(path) => path,
            Err(_) => {
                non_utf8_count = non_utf8_count.saturating_add(1);
                retain_lexicographically_smallest(
                    &mut non_utf8_sample,
                    encoded_file_name(&entry.file_name()),
                );
                continue;
            }
        };
        if !path.as_str().ends_with(".receipt.json") {
            continue;
        }

        receipt_count = receipt_count.saturating_add(1);
        if paths.len() < MAX_GATE_BUNDLES {
            paths.insert(path);
        } else if paths.last().is_some_and(|last| path < *last) {
            paths.pop_last();
            paths.insert(path);
        }
    }

    let mut set = GateEvidenceSet::default();
    let directory_reference = repo.relative(&dir);
    if entry_error_count > 0 {
        set.failures.push((
            directory_reference.clone(),
            format!(
                "Gate receipt directory enumeration failed for {entry_error_count} entr{} (smallest error class: {})",
                if entry_error_count == 1 { "y" } else { "ies" },
                entry_error_sample.as_deref().unwrap_or("unknown"),
            ),
        ));
    }
    if non_utf8_count > 0 {
        set.failures.push((
            directory_reference.clone(),
            format!(
                "Gate receipt directory contains {non_utf8_count} non-UTF-8 entr{} (smallest encoded name: {})",
                if non_utf8_count == 1 { "y" } else { "ies" },
                non_utf8_sample.as_deref().unwrap_or("unknown"),
            ),
        ));
    }
    if receipt_count > MAX_GATE_BUNDLES as u64 {
        set.failures.push((
            directory_reference,
            format!(
                "Gate receipt count {receipt_count} exceeds {MAX_GATE_BUNDLES}; only the lexicographically first {MAX_GATE_BUNDLES} candidates were selected for validation"
            ),
        ));
    }
    let mut seen_runs = BTreeMap::<String, String>::new();
    for path in paths {
        let reference = repo.relative(&path);
        match load_one(repo, &path, basis, authority) {
            Ok(run) => {
                if let Some(first) = seen_runs.insert(run.id().to_owned(), reference.clone()) {
                    set.failures.push((
                        reference,
                        format!("duplicate Gate Run identity already admitted from {first}"),
                    ));
                } else {
                    set.runs.push(run);
                }
            }
            Err(error) => set.failures.push((reference, error)),
        }
    }
    set.failures.sort();
    set
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use camino::Utf8PathBuf;
    use openwarrant_core::{Namespace, RepositoryConfig};

    use super::*;

    fn empty_repository() -> (tempfile::TempDir, Repository) {
        let scratch = tempfile::Builder::new()
            .prefix("openwarrant-gate-evidence-")
            .tempdir()
            .expect("create temporary repository");
        let root = Utf8PathBuf::from_path_buf(scratch.path().to_owned())
            .expect("temporary repository path is UTF-8");
        let repo = Repository {
            root,
            config: RepositoryConfig::new(
                "Gate evidence fixture",
                Namespace::parse("OW").expect("fixture namespace"),
            ),
        };
        std::fs::create_dir_all(repo.root.join(&repo.config.paths.receipts))
            .expect("create receipt directory");
        (scratch, repo)
    }

    fn resolution_basis() -> ResolutionEvidenceBasis {
        ResolutionEvidenceBasis::new(
            &"a".repeat(64),
            [
                ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-A".to_owned(),
                    target_ref: "artifacts/shared.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", "b".repeat(64)),
                },
                ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-B".to_owned(),
                    target_ref: "artifacts/shared.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", "b".repeat(64)),
                },
            ],
            [],
        )
        .expect("shared artifact target is valid")
    }

    #[test]
    fn resolution_subjects_require_contract_but_allow_gate_specific_artifact_subset() {
        let basis = resolution_basis();
        let contract = format!("contract:sha256:{}", "a".repeat(64));
        let artifact = format!("sha256:{}", "b".repeat(64));

        assert!(basis.admits_subjects(&BTreeSet::from([contract.as_str()])));
        assert!(basis.admits_subjects(&BTreeSet::from([contract.as_str(), artifact.as_str(),])));
        assert!(!basis.admits_subjects(&BTreeSet::from([artifact.as_str()])));
        let unknown = format!("sha256:{}", "c".repeat(64));
        assert!(!basis.admits_subjects(&BTreeSet::from([contract.as_str(), unknown.as_str(),])));
    }

    #[test]
    fn artifact_raw_reference_requires_matching_digest_subject() {
        let basis = resolution_basis();
        let contract = format!("contract:sha256:{}", "a".repeat(64));
        let artifact = format!("sha256:{}", "b".repeat(64));
        let raw = BTreeSet::from(["artifact://DEL-A"]);

        assert!(
            !basis.raw_artifacts_are_subject_bound(&BTreeSet::from([contract.as_str()]), &raw,)
        );
        assert!(basis.raw_artifacts_are_subject_bound(
            &BTreeSet::from([contract.as_str(), artifact.as_str()]),
            &raw,
        ));
    }

    #[test]
    fn resolution_basis_rejects_duplicate_artifact_ids() {
        let error = ResolutionEvidenceBasis::new(
            &"a".repeat(64),
            [
                ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-A".to_owned(),
                    target_ref: "a.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", "b".repeat(64)),
                },
                ResolutionEvidenceArtifact {
                    deliverable_id: "DEL-A".to_owned(),
                    target_ref: "b.bin".to_owned(),
                    observed_sha256: format!("sha256:{}", "c".repeat(64)),
                },
            ],
            [],
        )
        .expect_err("duplicate logical artifact identity must fail closed");
        assert!(error.contains("declared more than once"));
    }

    #[test]
    fn live_file_raw_evidence_is_content_bound() {
        let (_scratch, repo) = empty_repository();
        let basis = resolution_basis();
        let path = repo.root.join("evidence.json");
        std::fs::write(&path, b"evidence-a").expect("write raw evidence");
        let matching = format!("file:evidence.json#sha256:{}", sha256_hex(b"evidence-a"));
        assert_eq!(
            verify_live_raw_evidence(&repo, std::slice::from_ref(&matching), &basis)
                .expect("matching evidence"),
            BTreeSet::from([matching])
        );

        let stale = format!("file:evidence.json#sha256:{}", sha256_hex(b"evidence-b"));
        let error = verify_live_raw_evidence(&repo, &[stale], &basis)
            .expect_err("stale content digest must reject");
        assert!(error.contains("digest does not match"));
    }

    #[cfg(unix)]
    #[test]
    fn load_admissible_reports_non_utf8_entries_and_preserves_other_candidates() {
        use std::os::unix::ffi::OsStringExt;

        let (_scratch, repo) = empty_repository();
        let receipt_dir = repo.root.join(&repo.config.paths.receipts);
        let receipt_path = receipt_dir.join("GR-readable.receipt.json");
        std::fs::write(&receipt_path, b"{}\n").expect("write readable receipt candidate");
        let file_name = std::ffi::OsString::from_vec(b"bad-\xff.receipt.json".to_vec());
        std::fs::File::create(receipt_dir.as_std_path().join(file_name))
            .expect("create non-UTF-8 directory entry");

        let evidence = load_admissible(
            &repo,
            &BTreeSet::new(),
            &BTreeSet::new(),
            &AuthorityRegister::default(),
        );

        assert!(evidence.runs.is_empty());
        assert_eq!(evidence.failures.len(), 2);
        assert!(evidence.failures.contains(&(
            repo.relative(&receipt_dir),
            "Gate receipt directory contains 1 non-UTF-8 entry (smallest encoded name: unix-bytes:6261642dff2e726563656970742e6a736f6e)"
                .to_owned(),
        )));
        assert!(
            evidence
                .failures
                .iter()
                .any(|(path, error)| path == &repo.relative(&receipt_path)
                    && error.starts_with("cannot parse Gate receipt:")),
            "readable receipt candidate was not retained for validation: {:?}",
            evidence.failures,
        );
    }
}
