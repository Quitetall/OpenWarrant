// SPDX-License-Identifier: AGPL-3.0-or-later
//! Verify ADR 0186 disposition evidence without creating a judgment.
//!
//! This command is read-only. It validates declarations against exact Git
//! objects, imported bodies, repository artifacts, current human authority,
//! and the exact authorized Warrant revision. It cannot authorize or resolve.

use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::process::Command;
use std::process::Output;
use std::str::FromStr;
use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use openwarrant_compiler::{DigestDomain, lower, sha256_digest, sha256_hex};
use openwarrant_core::authority::{ActorRole, AuthorityRegister, RoleAssignment};
use openwarrant_core::deliverable::Deliverable;
use openwarrant_core::epistemic::Judgment;
#[cfg(test)]
use openwarrant_core::legacy_disposition::LegacyAdrDisposition;
use openwarrant_core::legacy_disposition::{
    ContentBinding, GitObjectFormat, ImportBinding, LegacyAdrDispositionManifest,
    LegacyAdrDispositionReceipt, LegacyAdrDispositionReviewResponse, LegacyAdrSuccessor,
    LegacyAdrSupport, SourceBinding, WarrantBinding, is_canonical_utc,
};
use openwarrant_core::rationale::Assumption;
use openwarrant_core::resolution::Resolution;
use openwarrant_core::verification::Verification;
use openwarrant_core::{
    ActorKind, GateBinding, GateDefinition, GateReceipt, GateRun, RepositoryConfig, WarUuid,
};
use serde::{Deserialize, de::DeserializeOwned};

use crate::authorize::{
    AUTHORIZATION_SCHEMA, AuthorizationRecord, JudgmentRecord, authorizes_current_contract,
};
use crate::migrate::{ImportArtifact, committed_adr_paths, import_from_repository};
use crate::repo::{Loaded, RepoError, Repository, read_repository_regular_bounded};

const FINAL_ADR_ID: &str = "0186";
const GOVERNING_WARRANT_ID: &str = "OW-WAR-0043";
const GOVERNING_WARRANT_UUID: &str = "01a021a2-b571-794a-acf0-1559844cf662";
const SHA_F: &str = "ba9ed833faa9a52940d5e9d424566466e9066867";
const EXPECTED_BASE_RECORDS: usize = 172;
const EXPECTED_RECEIPTS: usize = 184;
const SOURCE_REPOSITORY_IDENTITY: &str = "github.com/Quitetall/LamQuant";
const SOURCE_CORPUS_ROOT: &str = "docs/decisions";
const MAX_MANIFEST_BYTES: u64 = 16 * 1024 * 1024;
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_AUTHORITY_BYTES: u64 = 1024 * 1024;
const MAX_RESOLUTION_BYTES: u64 = 1024 * 1024;
const MAX_RESOLUTION_EVIDENCE_FILE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RESOLUTION_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RESOLUTION_EVIDENCE_FILES: usize = 4096;
const MAX_RESOLUTION_DELIVERABLES: usize = 4096;
const MAX_RESOLUTION_ARTIFACT_TARGETS: usize = 2048;
const MAX_RESOLUTION_GATE_FIXTURES: usize = 512;
const MAX_RESOLUTION_COMMIT_PATHS: usize = 8192;
const MAX_REVIEW_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_REVIEW_RESPONSE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_SUPPORT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_SUPPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SUPPORT_BINDINGS: usize = 4096;
const MAX_UNIQUE_SUPPORT_ARTIFACTS: usize = 2048;
const MAX_GIT_BLOB_BYTES: u64 = 4 * 1024 * 1024;
const MAX_GIT_CONTROL_BYTES: usize = 1024 * 1024;
const MAX_GIT_INVENTORY_BYTES: usize = 64 * 1024 * 1024;
const REVIEW_RESPONSE_PREFIX: &str = "docs/warrants/OW-WAR-0043/responses/legacy-adr-dispositions/";
const OFF_BRANCH_IDS: [&str; 12] = [
    "0147", "0148", "0149", "0169", "0170", "0171", "0172", "0173", "0174", "0175", "0187", "0188",
];
const CONSOLIDATE_REF: &str = "refs/heads/integration/consolidate";
const ARCHIVE_REF: &str = "refs/tags/archive/feat/blut-1.0-integrations";
const ARCHIVE_TIP: &str = "65289d35aeb11485de280a12a647640d6c302440";
const ADR0167_REF: &str = "refs/heads/integration/adr0167";
const ADR0167_TIP: &str = "79854e61202312f8108ff6caac8f848d404cfc5b";
const CASCADE_REF: &str = "refs/heads/integration/adr0168-cascade";
const CASCADE_TIP: &str = "1cb0d82655d1260c64b1f6fe2d88100c0a215a9b";
const ARCHIVE_CENSUS_REF: [(&str, &str); 1] = [(ARCHIVE_REF, ARCHIVE_TIP)];
const ADR0167_CENSUS_REF: [(&str, &str); 1] = [(ADR0167_REF, ADR0167_TIP)];
const CASCADE_CENSUS_REF: [(&str, &str); 1] = [(CASCADE_REF, CASCADE_TIP)];
const BOTH_BRANCH_CENSUS_REFS: [(&str, &str); 2] =
    [(ADR0167_REF, ADR0167_TIP), (CASCADE_REF, CASCADE_TIP)];

struct FrozenOffBranchSource {
    adr_id: &'static str,
    refs: &'static [(&'static str, &'static str)],
    commit: &'static str,
    path: &'static str,
    source_sha256: &'static str,
}

const OFF_BRANCH_SOURCES: [FrozenOffBranchSource; 14] = [
    FrozenOffBranchSource {
        adr_id: "0147",
        refs: &ARCHIVE_CENSUS_REF,
        commit: "ce8ef6c8c34018efc79b28fb8f3826669d978b53",
        path: "docs/decisions/0147-blut-signed-platform-toolchain-routing.md",
        source_sha256: "sha256:0b1c7792af7e226b6166d0752115b99cf1de1af57fb623928a8e8bfe078e9ddc",
    },
    FrozenOffBranchSource {
        adr_id: "0148",
        refs: &ARCHIVE_CENSUS_REF,
        commit: "ce8ef6c8c34018efc79b28fb8f3826669d978b53",
        path: "docs/decisions/0148-blut-content-locked-starlark-modules.md",
        source_sha256: "sha256:afab395d9399e4d7729d6fd4ee7354dc59cb201fa9a1091a61ab4e8346157233",
    },
    FrozenOffBranchSource {
        adr_id: "0149",
        refs: &ARCHIVE_CENSUS_REF,
        commit: "ce8ef6c8c34018efc79b28fb8f3826669d978b53",
        path: "docs/decisions/0149-blut-outbound-reapi-adapter.md",
        source_sha256: "sha256:6890e1f383096e10b650b70a03bca082bc591ef23f788d90beff7a34d4e365e0",
    },
    FrozenOffBranchSource {
        adr_id: "0169",
        refs: &ADR0167_CENSUS_REF,
        commit: "e6a448a4ba140f0853f4f5f3556462e32a83ab8e",
        path: "docs/decisions/0169-one-provenance-identity-abir-content-id.md",
        source_sha256: "sha256:31bbd961d463f781b257e5833901f7167c14628c3ba093fb0e00e8e715fed0ce",
    },
    FrozenOffBranchSource {
        adr_id: "0169",
        refs: &CASCADE_CENSUS_REF,
        commit: "4a16de543314e083daf1fe9fc22deb32a27c6738",
        path: "docs/decisions/0169-one-provenance-identity-abir-content-id.md",
        source_sha256: "sha256:92373ac18794fd70643d7c5aad811efec9f0d976e907a7b48b7f8c226f429e9e",
    },
    FrozenOffBranchSource {
        adr_id: "0170",
        refs: &BOTH_BRANCH_CENSUS_REFS,
        commit: "835a0b2de43dba451c83412e38534270850eb3ae",
        path: "docs/decisions/0170-one-dataset-command-surface.md",
        source_sha256: "sha256:00c0bbe30f0f3e83b406bba64ee9331238cd69622d9267ad4a7e63b83f10dee1",
    },
    FrozenOffBranchSource {
        adr_id: "0171",
        refs: &ADR0167_CENSUS_REF,
        commit: "e6a448a4ba140f0853f4f5f3556462e32a83ab8e",
        path: "docs/decisions/0171-one-neural-kernel-set-tritium.md",
        source_sha256: "sha256:39a565516a8766c55dcb79b492da50b45aaffd24a9fc42ed1ef678a67166ef14",
    },
    FrozenOffBranchSource {
        adr_id: "0171",
        refs: &CASCADE_CENSUS_REF,
        commit: "331c14dd81095d77061dd18b09f154f64c810aad",
        path: "docs/decisions/0171-one-neural-kernel-set-tritium.md",
        source_sha256: "sha256:d84e2dc5907ca098f5f9632d2efb31de32691fb6071b96a913ecc1a9903c8757",
    },
    FrozenOffBranchSource {
        adr_id: "0172",
        refs: &BOTH_BRANCH_CENSUS_REFS,
        commit: "187bc57e957a411924d3e527908f404c1f5b6b50",
        path: "docs/decisions/0172-one-build-topology-and-dependency-mode.md",
        source_sha256: "sha256:dbb4d8f91153c6a136f6c124c439f32514b49e344704f3ff09fb8ed1f8e9d4a0",
    },
    FrozenOffBranchSource {
        adr_id: "0173",
        refs: &BOTH_BRANCH_CENSUS_REFS,
        commit: "67a922ffd7829dab0a02d336cd6adae37f88a368",
        path: "docs/decisions/0173-one-blut-cookbook-core.md",
        source_sha256: "sha256:769c175a88724765c1e2e107d8f8b24ae3692db192d2f3f960ced9b189c2b19a",
    },
    FrozenOffBranchSource {
        adr_id: "0174",
        refs: &BOTH_BRANCH_CENSUS_REFS,
        commit: "32faf19e19f9c78e2473dfc240223aad2b2c57a6",
        path: "docs/decisions/0174-no-orphaned-artifacts.md",
        source_sha256: "sha256:bb23ea68fddc87e1a0344e784c3a57dbda21678dd971e532ab622cc5e7a0d51a",
    },
    FrozenOffBranchSource {
        adr_id: "0175",
        refs: &BOTH_BRANCH_CENSUS_REFS,
        commit: "155a031cc6ce3f2262670b6340779c35ebbad43e",
        path: "docs/decisions/0175-stable-sdk-surface-after-consolidation.md",
        source_sha256: "sha256:6aa72e09ba6791618a5a6c77e658b41f85b0ee9012eec9f9a1d48eea7357701f",
    },
    FrozenOffBranchSource {
        adr_id: "0187",
        refs: &ADR0167_CENSUS_REF,
        commit: "73d0a005a322ec8521b730946bcd7d007558d260",
        path: "docs/decisions/0187-close-blut-core-with-auditable-history.md",
        source_sha256: "sha256:07d71cf30fc9f2220d4434cc22f25b6959fd84fc4956dc7fb2925ce1a55d67d2",
    },
    FrozenOffBranchSource {
        adr_id: "0188",
        refs: &ADR0167_CENSUS_REF,
        commit: "73d0a005a322ec8521b730946bcd7d007558d260",
        path: "docs/decisions/0188-make-blut-caller-evidence-exhaustive.md",
        source_sha256: "sha256:0862cad8c9286b906cbe0e6261172da5dae30eb83432011fc4edba2b3bb80452",
    },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub receipts: usize,
    pub migrated: usize,
    pub deliberately_excluded: usize,
    pub warrant_repository_revision: String,
}

#[derive(Debug)]
struct CachedReviewResponse {
    digest: String,
    response: LegacyAdrDispositionReviewResponse,
}

type CheckedRefs = BTreeMap<(String, String), String>;

#[derive(Debug, Default)]
struct ReviewState {
    responses: BTreeMap<String, CachedReviewResponse>,
    used_judgments: BTreeSet<(String, String)>,
    total_bytes: u64,
}

#[derive(Debug, Clone)]
struct CachedSupport {
    reference: String,
    digest: String,
    identity: Option<(String, String)>,
    bytes: Arc<[u8]>,
}

#[derive(Debug, Default)]
struct SupportState {
    artifacts: BTreeMap<String, CachedSupport>,
    bindings: usize,
    total_bytes: u64,
}

#[derive(Debug, Default)]
struct InternalArtifacts {
    references: BTreeSet<String>,
    digests: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy)]
struct ResolutionVerificationContext<'a> {
    repo: &'a Repository,
    authority: &'a AuthorityRegister,
    verification_as_of: &'a str,
}

#[derive(Debug)]
struct BoundResolution {
    resolution: Resolution,
    actor: String,
}

#[derive(Debug)]
struct VerifiedWarrant {
    loaded: Loaded,
    authorization: AuthorizationRecord,
    current_contract_digest: String,
    authorization_effective_at: String,
}

#[derive(Debug)]
struct PinnedResolutionEvidence {
    verifications: Vec<Verification>,
    verification_refs: Vec<String>,
    deliverables: Vec<Deliverable>,
    deliverables_ref: String,
    gate_runs: Vec<GateRun>,
    admissible_gate_runs: Vec<crate::gate_evidence::AdmissibleGateRun>,
    gate_run_source_refs: Vec<String>,
    gate_definitions: Vec<crate::resolve::RecordedGateDefinition>,
    gate_bindings: Vec<GateBinding>,
    gate_binding_source_refs: Vec<String>,
    gate_selection_source_refs: Vec<String>,
    gate_fixture_source_refs: Vec<String>,
    gate_raw_evidence_source_refs: Vec<String>,
    gate_receipts: Vec<GateReceipt>,
    gate_receipt_source_refs: Vec<String>,
    gate_outputs: Vec<crate::resolve::RecordedGateOutput>,
    judgments: Vec<Judgment>,
    judgments_ref: Option<String>,
    assumptions: Option<Vec<Assumption>>,
    rationale_ref: Option<String>,
    artifact_checks: crate::resolve::ArtifactChecks,
    artifact_entries: Vec<crate::resolve::ArtifactSnapshotEntry>,
}

impl InternalArtifacts {
    fn insert(&mut self, binding: &ContentBinding) {
        self.references.insert(binding.reference.clone());
        self.digests.insert(binding.sha256.clone());
    }

    fn contains(&self, binding: &ContentBinding) -> bool {
        self.references.contains(&binding.reference) || self.digests.contains(&binding.sha256)
    }
}

pub fn run(
    manifest_path: &Utf8Path,
    source_repo: &Utf8Path,
    source_import_path: &Utf8Path,
    warrant_repo_path: &Utf8Path,
) -> Result<Report, RepoError> {
    let warrant_root = canonical_utf8(warrant_repo_path, "Warrant repository root")?;
    let manifest_path = repository_input_path(&warrant_root, manifest_path);
    let source_import_path = repository_input_path(&warrant_root, source_import_path);
    let snapshot = RepositorySnapshot::capture(&warrant_root)?;
    let manifest_bytes = read_repository_regular_bounded(
        &manifest_path,
        &warrant_root,
        "legacy disposition manifest",
        MAX_MANIFEST_BYTES,
    )?;
    let manifest: LegacyAdrDispositionManifest =
        openwarrant_core::legacy_disposition::parse_legacy_json(&manifest_bytes).map_err(
            |error| {
                RepoError::Message(format!(
                    "legacy disposition manifest {manifest_path} is not valid JSON: {error}"
                ))
            },
        )?;
    manifest.validate_structure().map_err(|error| {
        RepoError::Message(format!(
            "legacy disposition manifest {manifest_path} is invalid: {error}"
        ))
    })?;
    verify_receipt_digests(&manifest)?;
    verify_manifest_digest(&manifest)?;
    verify_fixed_scope(&manifest)?;

    let manifest_ref = repository_relative(&snapshot.root, &manifest_path)?;
    snapshot.verify_bytes(&manifest_ref, &manifest_bytes, MAX_MANIFEST_BYTES)?;
    let internal_artifacts = collect_internal_artifacts(
        &manifest,
        ContentBinding {
            reference: manifest_ref,
            sha256: content_digest(&manifest_bytes),
            extensions: Default::default(),
        },
    );
    let config_path = warrant_root.join("openwarrant.toml");
    let config_bytes = read_repository_regular_bounded(
        &config_path,
        &warrant_root,
        "OpenWarrant repository config",
        MAX_AUTHORITY_BYTES,
    )?;
    snapshot.verify_bytes("openwarrant.toml", &config_bytes, MAX_AUTHORITY_BYTES)?;
    let config_text = std::str::from_utf8(&config_bytes).map_err(|_| {
        RepoError::Message(format!(
            "OpenWarrant repository config {config_path} is not UTF-8"
        ))
    })?;
    let config: RepositoryConfig =
        toml::from_str(config_text).map_err(|source| RepoError::ConfigParse {
            path: config_path.clone(),
            source,
        })?;
    config
        .validate()
        .map_err(|source| RepoError::ConfigInvalid {
            path: config_path,
            source,
        })?;
    let warrant_repo = Repository {
        root: warrant_root,
        config,
    };
    let authority = verify_authority_register(&warrant_repo, &manifest, &snapshot)?;
    let governing_warrant = verify_warrant(
        &warrant_repo,
        &manifest.payload.warrant,
        GOVERNING_WARRANT_UUID,
        &authority,
        &snapshot,
        &manifest.payload.verification_as_of,
    )?;

    let source_import_ref = repository_relative(&warrant_repo.root, &source_import_path)?;
    if manifest.payload.source_import.reference != source_import_ref {
        return Err(RepoError::Message(format!(
            "manifest binds source import {:?}, but command received {source_import_ref:?}",
            manifest.payload.source_import.reference
        )));
    }
    let source_import_bytes = read_repository_regular_bounded(
        &source_import_path,
        &warrant_repo.root,
        "source import artifact",
        MAX_IMPORT_BYTES,
    )?;
    verify_raw_binding(&manifest.payload.source_import, &source_import_bytes)?;
    let source_import = parse_import(&source_import_path, &source_import_bytes)?;
    snapshot.verify_bytes(
        &manifest.payload.source_import.reference,
        &source_import_bytes,
        MAX_IMPORT_BYTES,
    )?;
    let regenerated = import_from_repository(source_repo, SOURCE_CORPUS_ROOT, SHA_F, false)
        .map_err(|error| RepoError::Message(format!("cannot reproduce source import: {error}")))?;
    let regenerated_bytes = crate::migrate::render(&regenerated)
        .map_err(|error| RepoError::Message(format!("cannot render reproduced import: {error}")))?;
    if source_import_bytes != regenerated_bytes.as_bytes() {
        return Err(RepoError::Message(
            "source import artifact is not byte-identical to deterministic war migrate output"
                .to_owned(),
        ));
    }
    let committed_paths =
        committed_adr_paths(source_repo, SOURCE_CORPUS_ROOT, SHA_F).map_err(|error| {
            RepoError::Message(format!("cannot derive exact SHA-F corpus: {error}"))
        })?;
    verify_base_inventory(&manifest, &source_import, &committed_paths)?;
    let mut checked_commits = BTreeSet::<String>::new();
    verify_import_corpus_bytes(
        source_repo,
        &source_import,
        &committed_paths,
        &mut checked_commits,
    )?;

    let mut artifacts = BTreeMap::<String, (String, ImportArtifact)>::new();
    artifacts.insert(
        source_import_ref,
        (content_digest(&source_import_bytes), source_import),
    );
    let mut imported_artifact_bytes = source_import_bytes.len() as u64;
    let mut reviews = ReviewState::default();
    let mut support = SupportState::default();
    let mut checked_refs = CheckedRefs::new();
    let mut migrated = 0;
    let mut excluded = 0;
    for receipt in &manifest.payload.receipts {
        verify_human_review(
            &authority,
            receipt,
            &governing_warrant.authorization_effective_at,
            &manifest.payload.verification_as_of,
            &warrant_repo.root,
            &snapshot,
            &mut reviews,
        )?;
        let source_bytes = verify_source(
            source_repo,
            receipt,
            &mut checked_commits,
            &mut checked_refs,
        )?;
        verify_support(
            &warrant_repo.root,
            source_repo,
            receipt,
            &mut checked_commits,
            &mut checked_refs,
            &snapshot,
            &mut support,
            &internal_artifacts,
            Some(ResolutionVerificationContext {
                repo: &warrant_repo,
                authority: &authority,
                verification_as_of: &manifest.payload.verification_as_of,
            }),
        )?;
        match &receipt.payload.migration {
            ImportBinding::Migrated { .. } => {
                migrated += 1;
                verify_import_binding(
                    receipt,
                    &warrant_repo.root,
                    &mut artifacts,
                    &mut imported_artifact_bytes,
                    &source_bytes,
                    source_repo,
                    &snapshot,
                )?;
            }
            ImportBinding::DeliberatelyExcluded { source_capsule, .. } => {
                excluded += 1;
                verify_source_capsule(
                    receipt,
                    source_capsule,
                    &warrant_repo.root,
                    &source_bytes,
                    &snapshot,
                )?;
            }
        }
    }
    verify_review_coverage(&reviews)?;
    verify_refs_unchanged(source_repo, &checked_refs)?;
    snapshot.verify_unchanged()?;

    Ok(Report {
        receipts: manifest.payload.receipts.len(),
        migrated,
        deliberately_excluded: excluded,
        warrant_repository_revision: snapshot.revision,
    })
}

pub fn print(report: &Report) {
    println!(
        "legacy dispositions: {} receipt(s) — {} migrated, {} deliberately excluded",
        report.receipts, report.migrated, report.deliberately_excluded
    );
    println!(
        "exact corpus, pinned Warrant snapshot {}, repository-authority human responses, source bytes, import bodies, support, and JCS digests: PASS",
        report.warrant_repository_revision
    );
    println!(
        "note: repository custody and role bindings are not cryptographic identity; this verifies content binding, not evidentiary sufficiency, authorization, or Resolution"
    );
}

fn collect_internal_artifacts(
    manifest: &LegacyAdrDispositionManifest,
    manifest_binding: ContentBinding,
) -> InternalArtifacts {
    let mut internal = InternalArtifacts::default();
    internal.insert(&manifest_binding);
    internal.insert(&manifest.payload.source_import);
    for receipt in &manifest.payload.receipts {
        internal.insert(&receipt.payload.review.response);
        match &receipt.payload.migration {
            ImportBinding::Migrated { artifact, .. } => internal.insert(artifact),
            ImportBinding::DeliberatelyExcluded { source_capsule, .. } => {
                internal.insert(source_capsule);
            }
        }
    }
    internal
}

fn verify_fixed_scope(manifest: &LegacyAdrDispositionManifest) -> Result<(), RepoError> {
    let payload = &manifest.payload;
    if payload.final_adr_id != FINAL_ADR_ID
        || payload.sha_f != SHA_F
        || payload.warrant.warrant_id != GOVERNING_WARRANT_ID
        || payload.sha_f_predecessor_ids.len() != EXPECTED_BASE_RECORDS
        || payload.receipts.len() != EXPECTED_RECEIPTS
    {
        return Err(RepoError::Message(format!(
            "ADR 0186 scope mismatch: final {}, SHA-F {}, base {}, receipts {}; expected {FINAL_ADR_ID}, {SHA_F}, {EXPECTED_BASE_RECORDS}, {EXPECTED_RECEIPTS}",
            payload.final_adr_id,
            payload.sha_f,
            payload.sha_f_predecessor_ids.len(),
            payload.receipts.len()
        )));
    }
    let expected_off = OFF_BRANCH_IDS.map(str::to_owned).to_vec();
    if payload.off_branch_ids != expected_off {
        return Err(RepoError::Message(format!(
            "off-branch inventory differs from ADR 0186's explicit twelve-record set: {:?}",
            payload.off_branch_ids
        )));
    }
    if payload.receipts.iter().any(|receipt| {
        receipt.payload.source.repository_identity != SOURCE_REPOSITORY_IDENTITY
            || receipt.payload.source.git_object_format != GitObjectFormat::Sha1
    }) {
        return Err(RepoError::Message(format!(
            "every receipt must bind SHA-1 repository identity {SOURCE_REPOSITORY_IDENTITY:?}"
        )));
    }
    for receipt in &payload.receipts {
        let source = &receipt.payload.source;
        let is_base = payload
            .sha_f_predecessor_ids
            .binary_search(&source.adr_id)
            .is_ok();
        verify_frozen_source_binding(source, is_base)?;
    }
    Ok(())
}

fn verify_frozen_source_binding(source: &SourceBinding, is_base: bool) -> Result<(), RepoError> {
    let matches = if is_base {
        source.commit_sha == SHA_F
            && source.exact_ref == CONSOLIDATE_REF
            && source.ref_tip_at_review == SHA_F
    } else {
        OFF_BRANCH_SOURCES.iter().any(|candidate| {
            candidate.adr_id == source.adr_id
                && candidate.commit == source.commit_sha
                && candidate.path == source.path
                && candidate.source_sha256 == source.source_sha256
                && candidate.refs.iter().any(|(reference, tip)| {
                    *reference == source.exact_ref && *tip == source.ref_tip_at_review
                })
        })
    };
    if !matches {
        return Err(RepoError::Message(format!(
            "ADR {} source binding is absent from frozen ADR 0186 source census",
            source.adr_id
        )));
    }
    Ok(())
}

fn verify_warrant(
    repo: &Repository,
    binding: &WarrantBinding,
    expected_uuid: &str,
    authority: &AuthorityRegister,
    snapshot: &RepositorySnapshot,
    verification_as_of: &str,
) -> Result<VerifiedWarrant, RepoError> {
    let dir = repo.warrant_dir(&binding.warrant_id)?;
    let loaded = repo.load_warrant(&dir)?;
    let (Some(basis), Some(validated)) = (&loaded.basis, &loaded.validated) else {
        return Err(RepoError::Message(format!(
            "{} does not compile, so no exact contract can be verified",
            binding.warrant_id
        )));
    };
    if !loaded.report.is_ready() {
        return Err(RepoError::Message(format!(
            "{} has blocking load diagnostics: {}",
            binding.warrant_id,
            loaded.report.verdict_line()
        )));
    }
    if validated.alias.to_string() != binding.warrant_id
        || validated.uuid.to_string() != expected_uuid
    {
        return Err(RepoError::Message(format!(
            "{} directory contains Warrant alias {} UUID {}; expected alias {} UUID {}",
            binding.warrant_id, validated.alias, validated.uuid, binding.warrant_id, expected_uuid
        )));
    }
    verify_compilation_basis_snapshot(snapshot, &dir, basis)?;
    let ir = lower(basis, validated).map_err(|error| {
        RepoError::Message(format!(
            "{} cannot lower to canonical contract: {error}",
            binding.warrant_id
        ))
    })?;
    let current_digest = ir.contract_digest().map_err(|error| {
        RepoError::Message(format!(
            "{} contract cannot be canonicalized: {error}",
            binding.warrant_id
        ))
    })?;
    if current_digest != binding.contract_digest {
        return Err(RepoError::Message(format!(
            "{} binds contract {}, current contract is {}",
            binding.warrant_id, binding.contract_digest, current_digest
        )));
    }

    let authorization_path = dir.join("authorization.toml");
    let expected_ref = repository_relative(&repo.root, &authorization_path)?;
    if binding.authorization_ref != expected_ref {
        return Err(RepoError::Message(format!(
            "{} authorization ref {:?} is not canonical {:?}",
            binding.warrant_id, binding.authorization_ref, expected_ref
        )));
    }
    let authorization_bytes = read_repository_regular_bounded(
        &authorization_path,
        &repo.root,
        "authorization record",
        MAX_AUTHORITY_BYTES,
    )?;
    snapshot.verify_bytes(
        &binding.authorization_ref,
        &authorization_bytes,
        MAX_AUTHORITY_BYTES,
    )?;
    if content_digest(&authorization_bytes) != binding.authorization_sha256 {
        return Err(RepoError::Message(format!(
            "{} authorization bytes do not match {}",
            binding.warrant_id, binding.authorization_sha256
        )));
    }
    let authorization_text = std::str::from_utf8(&authorization_bytes).map_err(|_| {
        RepoError::Message(format!(
            "{} authorization record is not UTF-8",
            binding.warrant_id
        ))
    })?;
    let authorization: AuthorizationRecord =
        toml::from_str(authorization_text).map_err(|error| {
            RepoError::Message(format!(
                "could not parse bound authorization record {authorization_path}: {error}"
            ))
        })?;
    if authorization.schema != AUTHORIZATION_SCHEMA
        || authorization.warrant != binding.warrant_id
        || authorization.revision.coverage != ir.contract_coverage
    {
        return Err(RepoError::Message(format!(
            "{} authorization record has wrong schema, Warrant identity, or contract coverage",
            binding.warrant_id
        )));
    }
    let embedded = authorization
        .revision
        .authorization
        .as_ref()
        .ok_or_else(|| {
            RepoError::Message(format!(
                "{} authorization record has no embedded authorization",
                binding.warrant_id
            ))
        })?;
    let predecessor_valid = if authorization.revision.revision == 1 {
        authorization.revision.predecessor_digest.is_none()
    } else {
        authorization
            .revision
            .predecessor_digest
            .as_deref()
            .is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
    };
    if !predecessor_valid
        || authorization
            .revision
            .proposer
            .as_deref()
            .is_none_or(|proposer| proposer.trim().is_empty())
    {
        return Err(RepoError::Message(format!(
            "{} authorization revision has invalid predecessor or blank proposer",
            binding.warrant_id
        )));
    }
    let assignment = authority.actor(&embedded.authorizer).ok_or_else(|| {
        RepoError::Message(format!(
            "{} authorizer {:?} has no authority assignment",
            binding.warrant_id, embedded.authorizer
        ))
    })?;
    let proposer = authorization.revision.proposer.as_deref().ok_or_else(|| {
        RepoError::Message(format!(
            "{} authorized revision has no proposer",
            binding.warrant_id
        ))
    })?;
    assignment.may_authorize(proposer).map_err(|error| {
        RepoError::Message(format!(
            "{} persisted authorizer is not permitted: {error}",
            binding.warrant_id
        ))
    })?;
    if embedded.actor_kind != assignment.actor_kind
        || embedded.acting_role != ActorRole::Authorizer.to_string()
        || embedded.meaning.trim().is_empty()
        || !is_canonical_utc(&assignment.effective_time)
        || !is_canonical_utc(&embedded.effective_time)
        || assignment.effective_time > embedded.effective_time
        || embedded.effective_time.as_str() > verification_as_of
    {
        return Err(RepoError::Message(format!(
            "{} embedded authorization does not match effective Authorizer authority",
            binding.warrant_id
        )));
    }
    if !authorizes_current_contract(&authorization, &current_digest)
        || authorization.revision.revision != binding.authorized_revision
    {
        return Err(RepoError::Message(format!(
            "{} is not authorized at bound revision {} and contract {}",
            binding.warrant_id, binding.authorized_revision, binding.contract_digest
        )));
    }
    let authorization_effective_at = embedded.effective_time.clone();
    Ok(VerifiedWarrant {
        loaded,
        authorization,
        current_contract_digest: current_digest,
        authorization_effective_at,
    })
}

fn verify_authority_register(
    repo: &Repository,
    manifest: &LegacyAdrDispositionManifest,
    snapshot: &RepositorySnapshot,
) -> Result<AuthorityRegister, RepoError> {
    let canonical_ref = "docs/authority/roles.toml";
    let path = repo.root.join(canonical_ref);
    let bytes = read_repository_regular_bounded(
        &path,
        &repo.root,
        "authority register",
        MAX_AUTHORITY_BYTES,
    )?;
    snapshot.verify_bytes(canonical_ref, &bytes, MAX_AUTHORITY_BYTES)?;
    let digest = content_digest(&bytes);
    for receipt in &manifest.payload.receipts {
        let review = &receipt.payload.review;
        if review.role_assignment_ref != canonical_ref || review.role_assignment_sha256 != digest {
            return Err(RepoError::Message(format!(
                "ADR {} review is not bound to current authority register {} ({digest})",
                receipt.payload.source.adr_id, canonical_ref
            )));
        }
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct AuthorityFile {
        #[serde(default)]
        assignment: Vec<RoleAssignment>,
    }

    let text = std::str::from_utf8(&bytes)
        .map_err(|_| RepoError::Message(format!("authority register {path} is not UTF-8")))?;
    let file: AuthorityFile = toml::from_str(text).map_err(|error| {
        RepoError::Message(format!(
            "could not parse bound authority register {path}: {error}"
        ))
    })?;
    for assignment in &file.assignment {
        assignment
            .validate()
            .map_err(|error| RepoError::Message(format!("{path}: {error}")))?;
    }
    let mut actors = BTreeSet::new();
    if let Some(duplicate) = file
        .assignment
        .iter()
        .find(|assignment| !actors.insert(assignment.actor.as_str()))
    {
        return Err(RepoError::Message(format!(
            "{path}: actor {:?} has multiple authority assignments",
            duplicate.actor
        )));
    }
    Ok(AuthorityRegister::new(file.assignment))
}

fn verify_reviewer_authority(
    register: &AuthorityRegister,
    receipt: &LegacyAdrDispositionReceipt,
    authorization_effective_at: &str,
    verification_as_of: &str,
) -> Result<(), RepoError> {
    let id = &receipt.payload.source.adr_id;
    let review = &receipt.payload.review;
    let assignment = register.actor(&review.actor).ok_or_else(|| {
        RepoError::Message(format!(
            "ADR {id} reviewer {:?} has no authority assignment",
            review.actor
        ))
    })?;
    if assignment.actor_kind != ActorKind::Human || !assignment.holds(ActorRole::Judge) {
        return Err(RepoError::Message(format!(
            "ADR {id} reviewer {:?} is not a human Judge",
            review.actor
        )));
    }
    if !is_canonical_utc(verification_as_of)
        || !is_canonical_utc(&review.reviewed_at)
        || review.reviewed_at.as_str() > verification_as_of
    {
        return Err(RepoError::Message(format!(
            "ADR {id} review at {:?} is after verification cutoff {verification_as_of:?}",
            review.reviewed_at
        )));
    }
    if !is_canonical_utc(&assignment.effective_time)
        || assignment.effective_time > review.reviewed_at
        || authorization_effective_at > review.reviewed_at.as_str()
    {
        return Err(RepoError::Message(format!(
            "ADR {id} review predates Judge assignment {:?} or governing authorization {authorization_effective_at:?}",
            assignment.effective_time,
        )));
    }
    Ok(())
}

fn verify_human_review(
    register: &AuthorityRegister,
    receipt: &LegacyAdrDispositionReceipt,
    authorization_effective_at: &str,
    verification_as_of: &str,
    warrant_repo: &Utf8Path,
    snapshot: &RepositorySnapshot,
    state: &mut ReviewState,
) -> Result<(), RepoError> {
    verify_reviewer_authority(
        register,
        receipt,
        authorization_effective_at,
        verification_as_of,
    )?;
    let id = &receipt.payload.source.adr_id;
    let review = &receipt.payload.review;

    if !review
        .response
        .reference
        .starts_with(REVIEW_RESPONSE_PREFIX)
        || review.response.reference == REVIEW_RESPONSE_PREFIX
    {
        return Err(RepoError::Message(format!(
            "ADR {id} review response {:?} is outside canonical prefix {REVIEW_RESPONSE_PREFIX:?}",
            review.response.reference
        )));
    }
    if !state.responses.contains_key(&review.response.reference) {
        let (path, bytes) = read_repository_bounded(
            warrant_repo,
            &review.response.reference,
            "legacy disposition review response",
            MAX_REVIEW_RESPONSE_BYTES,
        )?;
        state.total_bytes = state
            .total_bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| RepoError::Message("review response byte budget overflow".to_owned()))?;
        if state.total_bytes > MAX_TOTAL_REVIEW_RESPONSE_BYTES {
            return Err(RepoError::Message(format!(
                "review responses total {} bytes; aggregate limit is {MAX_TOTAL_REVIEW_RESPONSE_BYTES}",
                state.total_bytes
            )));
        }
        verify_raw_binding(&review.response, &bytes)?;
        snapshot.verify_bytes(
            &review.response.reference,
            &bytes,
            MAX_REVIEW_RESPONSE_BYTES,
        )?;
        let response: LegacyAdrDispositionReviewResponse =
            openwarrant_core::legacy_disposition::parse_legacy_json(&bytes).map_err(|error| {
                RepoError::Message(format!("review response {path} is not valid JSON: {error}"))
            })?;
        response.validate_structure().map_err(|error| {
            RepoError::Message(format!("review response {path} is invalid: {error}"))
        })?;
        state.responses.insert(
            review.response.reference.clone(),
            CachedReviewResponse {
                digest: content_digest(&bytes),
                response,
            },
        );
    }
    let cached = &state.responses[&review.response.reference];
    if cached.digest != review.response.sha256
        || cached.response.warrant != receipt.payload.warrant
        || cached.response.reviewed_at != review.reviewed_at
    {
        return Err(RepoError::Message(format!(
            "ADR {id} review response digest, Warrant, or review time differs from receipt"
        )));
    }
    let mut matches = cached
        .response
        .judgments
        .iter()
        .filter(|judgment| judgment.id == review.judgment_id);
    let judgment = matches.next().ok_or_else(|| {
        RepoError::Message(format!(
            "ADR {id} review response contains no judgment {:?}",
            review.judgment_id
        ))
    })?;
    if matches.next().is_some() {
        return Err(RepoError::Message(format!(
            "ADR {id} review response duplicates judgment {:?}",
            review.judgment_id
        )));
    }
    let subject_digest = artifact_digest(&receipt.payload.review_subject_preimage())?;
    if judgment.adr_id != *id
        || judgment.review_subject_digest != subject_digest
        || judgment.actor != review.actor
        || judgment.acting_role != ActorRole::Judge.to_string()
    {
        return Err(RepoError::Message(format!(
            "ADR {id} review judgment does not bind exact subject, actor, and Judge role"
        )));
    }
    let use_key = (
        review.response.reference.clone(),
        review.judgment_id.clone(),
    );
    if !state.used_judgments.insert(use_key) {
        return Err(RepoError::Message(format!(
            "ADR {id} reuses review judgment {:?}",
            review.judgment_id
        )));
    }
    Ok(())
}

fn verify_review_coverage(state: &ReviewState) -> Result<(), RepoError> {
    for (reference, cached) in &state.responses {
        for judgment in &cached.response.judgments {
            if !state
                .used_judgments
                .contains(&(reference.clone(), judgment.id.clone()))
            {
                return Err(RepoError::Message(format!(
                    "review response {reference:?} contains unused judgment {:?}",
                    judgment.id
                )));
            }
        }
    }
    Ok(())
}

fn verify_receipt_digests(manifest: &LegacyAdrDispositionManifest) -> Result<(), RepoError> {
    for receipt in &manifest.payload.receipts {
        let recomputed = artifact_digest(&receipt.digest_preimage())?;
        if recomputed != receipt.receipt_digest {
            return Err(RepoError::Message(format!(
                "ADR {} receipt digest mismatch: recorded {}, recomputed {}",
                receipt.payload.source.adr_id, receipt.receipt_digest, recomputed
            )));
        }
    }
    Ok(())
}

fn verify_manifest_digest(manifest: &LegacyAdrDispositionManifest) -> Result<(), RepoError> {
    let recomputed = artifact_digest(&manifest.digest_preimage())?;
    if recomputed != manifest.manifest_digest {
        return Err(RepoError::Message(format!(
            "disposition manifest digest mismatch: recorded {}, recomputed {}",
            manifest.manifest_digest, recomputed
        )));
    }
    Ok(())
}

fn artifact_digest<T: serde::Serialize>(value: &T) -> Result<String, RepoError> {
    sha256_digest(DigestDomain::Artifact, value)
        .map(|digest| format!("sha256:{digest}"))
        .map_err(|error| RepoError::Message(format!("cannot canonicalize disposition: {error}")))
}

fn content_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256_hex(bytes))
}

#[derive(Debug)]
struct RepositorySnapshot {
    root: Utf8PathBuf,
    revision: String,
}

impl RepositorySnapshot {
    fn capture(root: &Utf8Path) -> Result<Self, RepoError> {
        let root = canonical_utf8(root, "Warrant repository root")?;
        let object_format = git(
            &root,
            "resolve Warrant repository object format",
            &["rev-parse", "--show-object-format"],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if !object_format.status.success() {
            return Err(git_failure(
                "resolve Warrant repository object format",
                &object_format,
            ));
        }
        let object_format = match object_format.stdout.as_slice() {
            b"sha1\n" => GitObjectFormat::Sha1,
            b"sha256\n" => GitObjectFormat::Sha256,
            other => {
                return Err(RepoError::Message(format!(
                    "Warrant repository uses unsupported Git object format {:?}",
                    String::from_utf8_lossy(other).trim()
                )));
            }
        };
        let revision = git(
            &root,
            "resolve Warrant repository snapshot",
            &["rev-parse", "--verify", "HEAD^{commit}"],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if !revision.status.success() {
            return Err(git_failure(
                "resolve Warrant repository snapshot",
                &revision,
            ));
        }
        let revision = std::str::from_utf8(&revision.stdout)
            .map_err(|_| RepoError::Message("Warrant repository HEAD is not UTF-8".to_owned()))?
            .trim()
            .to_owned();
        let expected_len = match object_format {
            GitObjectFormat::Sha1 => 40,
            GitObjectFormat::Sha256 => 64,
        };
        if revision.len() != expected_len
            || !revision
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(RepoError::Message(format!(
                "Warrant repository HEAD is not a literal lowercase {expected_len}-character commit id: {revision:?}"
            )));
        }
        let snapshot = Self { root, revision };
        snapshot.require_clean()?;
        Ok(snapshot)
    }

    fn require_clean(&self) -> Result<(), RepoError> {
        let status = git(
            &self.root,
            "check Warrant repository snapshot cleanliness",
            &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
            MAX_GIT_INVENTORY_BYTES,
        )?;
        if !status.status.success() {
            return Err(git_failure(
                "check Warrant repository snapshot cleanliness",
                &status,
            ));
        }
        if !status.stdout.is_empty() {
            return Err(RepoError::Message(
                "Warrant repository has tracked or untracked changes; disposition verification requires one committed snapshot"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    fn verify_bytes(
        &self,
        reference: &str,
        observed: &[u8],
        max_bytes: u64,
    ) -> Result<(), RepoError> {
        validate_git_path(reference)?;
        let expected = read_snapshot_blob(&self.root, &self.revision, reference, max_bytes)?;
        if expected != observed {
            return Err(RepoError::Message(format!(
                "repository artifact {reference:?} differs from pinned snapshot {}",
                self.revision
            )));
        }
        Ok(())
    }

    fn verify_unchanged(&self) -> Result<(), RepoError> {
        let current = git(
            &self.root,
            "recheck Warrant repository snapshot",
            &["rev-parse", "--verify", "HEAD^{commit}"],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if !current.status.success()
            || std::str::from_utf8(&current.stdout).map(str::trim).ok()
                != Some(self.revision.as_str())
        {
            return Err(RepoError::Message(
                "Warrant repository HEAD changed during disposition verification".to_owned(),
            ));
        }
        self.require_clean()
    }

    fn list_files(&self, prefix: &str) -> Result<Vec<String>, RepoError> {
        validate_git_path(prefix)?;
        let exact_pathspec = format!(":(top,literal){prefix}");
        let output = git(
            &self.root,
            "list Warrant snapshot evidence",
            &[
                "ls-tree",
                "-r",
                "--name-only",
                "-z",
                &self.revision,
                "--",
                &exact_pathspec,
            ],
            MAX_GIT_INVENTORY_BYTES,
        )?;
        if !output.status.success() {
            return Err(git_failure("list Warrant snapshot evidence", &output));
        }
        let mut paths = Vec::new();
        for raw in output.stdout.split(|byte| *byte == 0) {
            if raw.is_empty() {
                continue;
            }
            let path = std::str::from_utf8(raw).map_err(|_| {
                RepoError::Message("Warrant snapshot contains a non-UTF-8 path".to_owned())
            })?;
            validate_git_path(path)?;
            let within_prefix = path == prefix
                || path
                    .strip_prefix(prefix)
                    .is_some_and(|suffix| suffix.starts_with('/'));
            if !within_prefix {
                continue;
            }
            paths.push(path.to_owned());
        }
        if paths.len() > MAX_RESOLUTION_EVIDENCE_FILES {
            return Err(RepoError::Message(format!(
                "Warrant snapshot evidence under {prefix:?} contains {} files; limit is {MAX_RESOLUTION_EVIDENCE_FILES}",
                paths.len()
            )));
        }
        paths.sort();
        Ok(paths)
    }

    fn read_blob(&self, path: &str, max_bytes: u64) -> Result<Vec<u8>, RepoError> {
        read_snapshot_blob(&self.root, &self.revision, path, max_bytes)
    }

    fn path_commit_identity(
        &self,
        path: &str,
    ) -> Result<crate::resolve::RecordedEvidenceCommit, RepoError> {
        validate_git_path(path)?;
        let exact_pathspec = format!(":(top,literal){path}");
        let output = git(
            &self.root,
            "read Warrant evidence commit time",
            &[
                "log",
                "-1",
                "--format=%H%x00%ct",
                &self.revision,
                "--",
                &exact_pathspec,
            ],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if !output.status.success() {
            return Err(git_failure("read Warrant evidence commit time", &output));
        }
        let separator = output
            .stdout
            .iter()
            .position(|byte| *byte == 0)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "Warrant snapshot {} has no valid commit identity for evidence {path:?}",
                    self.revision,
                ))
            })?;
        let commit = std::str::from_utf8(&output.stdout[..separator])
            .map(str::to_owned)
            .map_err(|_| {
                RepoError::Message(format!(
                    "Warrant snapshot {} returned a non-UTF-8 commit identity for evidence {path:?}",
                    self.revision,
                ))
            })?;
        let committed_unix_seconds = std::str::from_utf8(&output.stdout[separator + 1..])
            .ok()
            .and_then(|value| value.trim().parse::<i64>().ok())
            .filter(|seconds| *seconds >= 0)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "Warrant snapshot {} has no valid commit time for evidence {path:?}",
                    self.revision,
                ))
            })?;
        if commit.len() != self.revision.len()
            || !commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(RepoError::Message(format!(
                "Warrant snapshot {} returned an invalid commit identity {commit:?} for evidence {path:?}",
                self.revision,
            )));
        }
        Ok(crate::resolve::RecordedEvidenceCommit {
            source_ref: path.to_owned(),
            commit,
            committed_unix_seconds,
        })
    }

    fn require_strict_ancestor(&self, ancestor: &str, descendant: &str) -> Result<(), RepoError> {
        if ancestor == descendant {
            return Err(RepoError::Message(format!(
                "evidence commit {ancestor} is not earlier than Resolution commit {descendant}"
            )));
        }
        let output = git(
            &self.root,
            "verify Warrant evidence ancestry",
            &["merge-base", "--is-ancestor", ancestor, descendant],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if output.status.success() {
            return Ok(());
        }
        if output.status.code() == Some(1) {
            return Err(RepoError::Message(format!(
                "evidence commit {ancestor} is not an ancestor of Resolution commit {descendant}"
            )));
        }
        Err(git_failure("verify Warrant evidence ancestry", &output))
    }
}

fn read_snapshot_blob(
    repository: &Utf8Path,
    revision: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<u8>, RepoError> {
    require_snapshot_regular_blob(repository, revision, path)?;
    let object = format!("{revision}:{path}");
    let size = git(
        repository,
        "measure Warrant snapshot blob",
        &["cat-file", "-s", &object],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !size.status.success() {
        return Err(git_failure("measure Warrant snapshot blob", &size));
    }
    let size = std::str::from_utf8(&size.stdout)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .ok_or_else(|| {
            RepoError::Message(format!("Git returned invalid blob size for {object:?}"))
        })?;
    if size > max_bytes {
        return Err(RepoError::Message(format!(
            "Warrant snapshot blob {object:?} is {size} bytes; limit is {max_bytes}"
        )));
    }
    let output = git(
        repository,
        "read Warrant snapshot blob",
        &["cat-file", "blob", &object],
        usize::try_from(size).map_err(|_| {
            RepoError::Message(format!(
                "Warrant snapshot blob {object:?} is too large for this host"
            ))
        })?,
    )?;
    if !output.status.success() {
        return Err(git_failure("read Warrant snapshot blob", &output));
    }
    if output.stdout.len() as u64 != size {
        return Err(RepoError::Message(format!(
            "Warrant snapshot blob {object:?} changed size while read"
        )));
    }
    Ok(output.stdout)
}

fn require_snapshot_regular_blob(
    repository: &Utf8Path,
    revision: &str,
    path: &str,
) -> Result<(), RepoError> {
    validate_git_path(path)?;
    let exact_pathspec = format!(":(top,literal){path}");
    let output = git(
        repository,
        "inspect Warrant snapshot object mode",
        &["ls-tree", "-z", revision, "--", &exact_pathspec],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !output.status.success() {
        return Err(git_failure("inspect Warrant snapshot object mode", &output));
    }
    let mut exact_mode = None;
    for entry in output.stdout.split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else {
            return Err(RepoError::Message(format!(
                "Git returned malformed tree entry for {path:?}"
            )));
        };
        let (metadata, raw_path_with_tab) = entry.split_at(tab);
        let raw_path = &raw_path_with_tab[1..];
        if raw_path != path.as_bytes() {
            continue;
        }
        let metadata = std::str::from_utf8(metadata).map_err(|_| {
            RepoError::Message(format!("Git returned non-UTF-8 tree metadata for {path:?}"))
        })?;
        let mut fields = metadata.split_ascii_whitespace();
        let mode = fields.next().unwrap_or_default();
        let kind = fields.next().unwrap_or_default();
        let object = fields.next().unwrap_or_default();
        if fields.next().is_some() || object.is_empty() {
            return Err(RepoError::Message(format!(
                "Git returned malformed tree metadata for {path:?}"
            )));
        }
        exact_mode = Some((mode, kind));
        break;
    }
    match exact_mode {
        Some(("100644" | "100755", "blob")) => Ok(()),
        Some((mode, kind)) => Err(RepoError::Message(format!(
            "Warrant snapshot object {path:?} is {mode} {kind}, not a regular Git blob"
        ))),
        None => Err(RepoError::Message(format!(
            "Warrant snapshot has no exact object {path:?}"
        ))),
    }
}

fn verify_compilation_basis_snapshot(
    snapshot: &RepositorySnapshot,
    warrant_dir: &Utf8Path,
    basis: &openwarrant_compiler::CompilationBasis,
) -> Result<(), RepoError> {
    snapshot.verify_bytes(
        &basis.manifest_source,
        &basis.manifest_bytes,
        MAX_AUTHORITY_BYTES,
    )?;
    let warrant_dir_ref = repository_relative(&snapshot.root, warrant_dir)?;
    for atom in &basis.atoms {
        let reference = format!("{warrant_dir_ref}/{}", atom.source);
        snapshot.verify_bytes(&reference, &atom.bytes, MAX_SUPPORT_BYTES)?;
    }
    if let Some(scope) = &basis.scope {
        snapshot.verify_bytes(&scope.source, &scope.bytes, MAX_AUTHORITY_BYTES)?;
    }
    Ok(())
}

fn read_repository_bounded(
    root: &Utf8Path,
    reference: &str,
    label: &str,
    max_bytes: u64,
) -> Result<(Utf8PathBuf, Vec<u8>), RepoError> {
    let root = canonical_utf8(root, "repository root")?;
    let path = safe_repository_path(&root, reference)?;
    let bytes = read_repository_regular_bounded(&path, &root, label, max_bytes)?;
    Ok((path, bytes))
}

fn parse_import(path: &Utf8Path, bytes: &[u8]) -> Result<ImportArtifact, RepoError> {
    openwarrant_core::legacy_disposition::parse_strict_json(bytes).map_err(|error| {
        RepoError::Message(format!(
            "migration artifact {path} is not valid JSON: {error}"
        ))
    })
}

fn verify_base_inventory(
    manifest: &LegacyAdrDispositionManifest,
    source_import: &ImportArtifact,
    committed_paths: &[(String, String)],
) -> Result<(), RepoError> {
    if source_import.commit_sha != SHA_F
        || source_import.corpus_relative_root != SOURCE_CORPUS_ROOT
        || source_import.adr_count != source_import.adrs.len()
        || source_import.adr_count != EXPECTED_BASE_RECORDS + 1
        || committed_paths.len() != EXPECTED_BASE_RECORDS + 1
        || !source_import.preservation_failures.is_empty()
    {
        return Err(RepoError::Message(format!(
            "source import is not exact SHA-F corpus: commit {}, root {:?}, adr_count {}, records {}, Git records {}, preservation failures {}",
            source_import.commit_sha,
            source_import.corpus_relative_root,
            source_import.adr_count,
            source_import.adrs.len(),
            committed_paths.len(),
            source_import.preservation_failures.len()
        )));
    }

    let committed_names: BTreeSet<&str> = committed_paths
        .iter()
        .map(|(filename, _)| filename.as_str())
        .collect();
    let imported_names: BTreeSet<&str> = source_import
        .adrs
        .iter()
        .map(|adr| adr.source.as_str())
        .collect();
    if committed_names.len() != committed_paths.len()
        || imported_names.len() != source_import.adrs.len()
        || committed_names != imported_names
    {
        return Err(RepoError::Message(format!(
            "source import inventory differs from exact SHA-F Git tree: Git unique {}, import unique {}",
            committed_names.len(),
            imported_names.len()
        )));
    }

    let mut ids = Vec::with_capacity(committed_paths.len());
    for (filename, _) in committed_paths {
        let id = filename
            .get(..4)
            .filter(|id| id.bytes().all(|byte| byte.is_ascii_digit()))
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "SHA-F Git record {filename:?} has no four-digit ADR id"
                ))
            })?;
        if id != FINAL_ADR_ID {
            ids.push(id.to_owned());
        }
    }
    ids.sort();
    let before_dedup = ids.len();
    ids.dedup();
    let final_count = committed_paths
        .iter()
        .filter(|(filename, _)| filename.starts_with("0186-"))
        .count();
    if before_dedup != EXPECTED_BASE_RECORDS
        || ids.len() != EXPECTED_BASE_RECORDS
        || final_count != 1
        || ids != manifest.payload.sha_f_predecessor_ids
    {
        return Err(RepoError::Message(format!(
            "SHA-F predecessor inventory differs from exact Git tree: manifest {}, derived {}, final records {final_count}",
            manifest.payload.sha_f_predecessor_ids.len(),
            ids.len()
        )));
    }
    Ok(())
}

fn git(
    source_repo: &Utf8Path,
    operation: &str,
    args: &[&str],
    max_stdout_bytes: usize,
) -> Result<Output, RepoError> {
    crate::git_cmd::output(source_repo.as_std_path(), args, max_stdout_bytes).map_err(|source| {
        RepoError::Io {
            context: format!("could not run Git to {operation}"),
            source,
        }
    })
}

fn git_failure(operation: &str, output: &Output) -> RepoError {
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    RepoError::Message(format!("Git could not {operation}: {detail}"))
}

fn require_literal_commit(
    source_repo: &Utf8Path,
    commit: &str,
    checked_commits: &mut BTreeSet<String>,
) -> Result<(), RepoError> {
    if checked_commits.insert(commit.to_owned()) {
        let output = git(
            source_repo,
            "resolve disposition source commit",
            &["cat-file", "-t", commit],
            MAX_GIT_CONTROL_BYTES,
        )?;
        if !output.status.success() || output.stdout != b"commit\n" {
            return Err(git_failure("resolve a literal commit object", &output));
        }
    }
    Ok(())
}

fn read_git_blob(
    source_repo: &Utf8Path,
    commit: &str,
    path: &str,
    checked_commits: &mut BTreeSet<String>,
) -> Result<Vec<u8>, RepoError> {
    require_literal_commit(source_repo, commit, checked_commits)?;
    let object = format!("{commit}:{path}");
    let size = git(
        source_repo,
        "measure disposition source blob",
        &["cat-file", "-s", &object],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !size.status.success() {
        return Err(git_failure("measure disposition source blob", &size));
    }
    let size = std::str::from_utf8(&size.stdout)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .ok_or_else(|| {
            RepoError::Message(format!("Git returned invalid blob size for {object:?}"))
        })?;
    if size > MAX_GIT_BLOB_BYTES {
        return Err(RepoError::Message(format!(
            "Git blob {object:?} is {size} bytes; limit is {MAX_GIT_BLOB_BYTES}"
        )));
    }
    let output = git(
        source_repo,
        "read disposition source blob",
        &["cat-file", "blob", &object],
        usize::try_from(size).map_err(|_| {
            RepoError::Message(format!("Git blob {object:?} is too large for this host"))
        })?,
    )?;
    if !output.status.success() {
        return Err(git_failure("read disposition source blob", &output));
    }
    if output.stdout.len() as u64 != size {
        return Err(RepoError::Message(format!(
            "Git blob {object:?} changed size while being read: expected {size}, read {}",
            output.stdout.len()
        )));
    }
    Ok(output.stdout)
}

fn verify_source(
    source_repo: &Utf8Path,
    receipt: &LegacyAdrDispositionReceipt,
    checked_commits: &mut BTreeSet<String>,
    checked_refs: &mut CheckedRefs,
) -> Result<Vec<u8>, RepoError> {
    let source = &receipt.payload.source;
    verify_ref_contains_tip(
        source_repo,
        &source.exact_ref,
        &source.ref_tip_at_review,
        checked_refs,
    )?;
    require_literal_commit(source_repo, &source.ref_tip_at_review, checked_commits)?;
    require_literal_commit(source_repo, &source.commit_sha, checked_commits)?;
    let ancestry = git(
        source_repo,
        "verify source commit ancestry",
        &[
            "merge-base",
            "--is-ancestor",
            &source.commit_sha,
            &source.ref_tip_at_review,
        ],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !ancestry.status.success() {
        return Err(git_failure("verify source commit ancestry", &ancestry));
    }
    let bytes = read_git_blob(
        source_repo,
        &source.commit_sha,
        &source.path,
        checked_commits,
    )?;
    let recomputed = content_digest(&bytes);
    if recomputed != source.source_sha256 {
        return Err(RepoError::Message(format!(
            "ADR {} source digest mismatch: recorded {}, recomputed {}",
            source.adr_id, source.source_sha256, recomputed
        )));
    }
    Ok(bytes)
}

fn verify_ref_contains_tip(
    repository: &Utf8Path,
    exact_ref: &str,
    historical_tip: &str,
    checked_refs: &mut CheckedRefs,
) -> Result<(), RepoError> {
    let binding = (exact_ref.to_owned(), historical_tip.to_owned());
    if checked_refs.contains_key(&binding) {
        return Ok(());
    }
    let current_tip = resolve_ref_tip(repository, exact_ref)?;
    let ancestry = git(
        repository,
        "verify historical ref-tip reachability",
        &["merge-base", "--is-ancestor", historical_tip, &current_tip],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !ancestry.status.success() {
        return Err(RepoError::Message(format!(
            "frozen tip {historical_tip} is no longer reachable from {exact_ref} at {current_tip}"
        )));
    }
    checked_refs.insert(binding, current_tip);
    Ok(())
}

fn resolve_ref_tip(repository: &Utf8Path, exact_ref: &str) -> Result<String, RepoError> {
    let expression = format!("{exact_ref}^{{commit}}");
    let resolved = git(
        repository,
        "resolve frozen source ref",
        &["rev-parse", "--verify", &expression],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !resolved.status.success() {
        return Err(git_failure("resolve frozen source ref", &resolved));
    }
    let current_tip = std::str::from_utf8(&resolved.stdout)
        .map_err(|_| RepoError::Message(format!("ref {exact_ref:?} resolved to non-UTF-8")))?
        .trim();
    if current_tip.len() != 40
        || !current_tip
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(RepoError::Message(format!(
            "ref {exact_ref:?} resolved to malformed commit {current_tip:?}"
        )));
    }
    Ok(current_tip.to_owned())
}

fn verify_refs_unchanged(
    repository: &Utf8Path,
    checked_refs: &CheckedRefs,
) -> Result<(), RepoError> {
    for ((exact_ref, historical_tip), observed_tip) in checked_refs {
        let current_tip = resolve_ref_tip(repository, exact_ref)?;
        if &current_tip != observed_tip {
            return Err(RepoError::Message(format!(
                "source ref {exact_ref} changed from {observed_tip} to {current_tip} while verifying historical tip {historical_tip}"
            )));
        }
    }
    Ok(())
}

fn verify_import_binding(
    receipt: &LegacyAdrDispositionReceipt,
    warrant_repo: &Utf8Path,
    artifacts: &mut BTreeMap<String, (String, ImportArtifact)>,
    imported_artifact_bytes: &mut u64,
    source_bytes: &[u8],
    source_repo: &Utf8Path,
    snapshot: &RepositorySnapshot,
) -> Result<(), RepoError> {
    let ImportBinding::Migrated {
        artifact,
        imported_body_sha256,
        ..
    } = &receipt.payload.migration
    else {
        unreachable!("caller dispatches migrated bindings")
    };
    if !artifacts.contains_key(&artifact.reference) {
        let (path, bytes) = read_repository_bounded(
            warrant_repo,
            &artifact.reference,
            "migration artifact",
            MAX_IMPORT_BYTES,
        )?;
        *imported_artifact_bytes = imported_artifact_bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| {
                RepoError::Message("migration artifact byte budget overflow".to_owned())
            })?;
        if *imported_artifact_bytes > MAX_TOTAL_IMPORT_BYTES {
            return Err(RepoError::Message(format!(
                "migration artifacts total {} bytes; aggregate limit is {MAX_TOTAL_IMPORT_BYTES}",
                *imported_artifact_bytes
            )));
        }
        verify_raw_binding(artifact, &bytes)?;
        snapshot.verify_bytes(&artifact.reference, &bytes, MAX_IMPORT_BYTES)?;
        let parsed = parse_import(&path, &bytes)?;
        verify_auxiliary_import_structure(&parsed, &artifact.reference)?;
        if parsed.commit_sha != receipt.payload.source.commit_sha
            || parsed.corpus_relative_root != SOURCE_CORPUS_ROOT
        {
            return Err(RepoError::Message(format!(
                "ADR {} auxiliary import names commit {} and root {:?}, expected {} and {SOURCE_CORPUS_ROOT:?}",
                receipt.payload.source.adr_id,
                parsed.commit_sha,
                parsed.corpus_relative_root,
                receipt.payload.source.commit_sha,
            )));
        }
        let regenerated =
            import_from_repository(source_repo, SOURCE_CORPUS_ROOT, &parsed.commit_sha, false)
                .map_err(|error| {
                    RepoError::Message(format!(
                        "cannot reproduce auxiliary import {}: {error}",
                        artifact.reference
                    ))
                })?;
        let regenerated_bytes = crate::migrate::render(&regenerated).map_err(|error| {
            RepoError::Message(format!(
                "cannot render reproduced auxiliary import {}: {error}",
                artifact.reference
            ))
        })?;
        if bytes != regenerated_bytes.as_bytes() {
            return Err(RepoError::Message(format!(
                "auxiliary import {} is not byte-identical to deterministic war migrate output",
                artifact.reference
            )));
        }
        artifacts.insert(artifact.reference.clone(), (content_digest(&bytes), parsed));
    }
    let (artifact_digest, imported) = &artifacts[&artifact.reference];
    if artifact.sha256 != *artifact_digest {
        return Err(RepoError::Message(format!(
            "ADR {} migration artifact digest differs from {}",
            receipt.payload.source.adr_id, artifact.reference
        )));
    }
    if imported.commit_sha != receipt.payload.source.commit_sha {
        return Err(RepoError::Message(format!(
            "ADR {} source commit {} differs from migration artifact commit {}",
            receipt.payload.source.adr_id, receipt.payload.source.commit_sha, imported.commit_sha
        )));
    }
    let filename = receipt
        .payload
        .source
        .path
        .rsplit('/')
        .next()
        .expect("validated source path has filename");
    let mut matching = imported.adrs.iter().filter(|adr| adr.source == filename);
    let record = matching.next().ok_or_else(|| {
        RepoError::Message(format!(
            "ADR {} source {} is absent from migration artifact {}",
            receipt.payload.source.adr_id, filename, artifact.reference
        ))
    })?;
    if matching.next().is_some() {
        return Err(RepoError::Message(format!(
            "ADR {} source {} is duplicated in migration artifact {}",
            receipt.payload.source.adr_id, filename, artifact.reference
        )));
    }
    let exact_digest = verify_migrated_body(
        &receipt.payload.source.adr_id,
        record,
        source_bytes,
        &artifact.reference,
    )?;
    if imported_body_sha256 != &exact_digest {
        return Err(RepoError::Message(format!(
            "ADR {} receipt body digest differs from exact Git source body in {}",
            receipt.payload.source.adr_id, artifact.reference
        )));
    }
    Ok(())
}

fn verify_auxiliary_import_structure(
    imported: &ImportArtifact,
    artifact: &str,
) -> Result<(), RepoError> {
    let unique: BTreeSet<&str> = imported
        .adrs
        .iter()
        .map(|record| record.source.as_str())
        .collect();
    if imported.adr_count != imported.adrs.len()
        || unique.len() != imported.adrs.len()
        || imported.promoted_resolutions != 0
        || !imported.preservation_failures.is_empty()
    {
        return Err(RepoError::Message(format!(
            "migration artifact {artifact} has inconsistent inventory, duplicate sources, promoted resolutions, or preservation failures"
        )));
    }
    Ok(())
}

fn verify_import_corpus_bytes(
    source_repo: &Utf8Path,
    source_import: &ImportArtifact,
    committed_paths: &[(String, String)],
    checked_commits: &mut BTreeSet<String>,
) -> Result<(), RepoError> {
    let records: BTreeMap<&str, _> = source_import
        .adrs
        .iter()
        .map(|record| (record.source.as_str(), record))
        .collect();
    for (filename, path) in committed_paths {
        let record = records.get(filename.as_str()).ok_or_else(|| {
            RepoError::Message(format!(
                "exact SHA-F record {filename:?} is absent from source import"
            ))
        })?;
        let source_bytes = read_git_blob(source_repo, SHA_F, path, checked_commits)?;
        let adr_id = filename.get(..4).unwrap_or("????");
        verify_migrated_body(
            adr_id,
            record,
            &source_bytes,
            "bound source import artifact",
        )?;
    }
    Ok(())
}

fn verify_migrated_body(
    adr_id: &str,
    record: &openwarrant_core::migration::MigratedAdr,
    source_bytes: &[u8],
    artifact: &str,
) -> Result<String, RepoError> {
    record
        .validate(crate::migrate::body_digest)
        .map_err(|error| {
            RepoError::Message(format!(
                "ADR {adr_id} migration artifact record is invalid: {error}"
            ))
        })?;
    let source_text = std::str::from_utf8(source_bytes).map_err(|_| {
        RepoError::Message(format!(
            "ADR {adr_id} source is not UTF-8 and cannot match text migration artifact"
        ))
    })?;
    let (_, exact_body) = crate::migrate::split_frontmatter(source_text);
    let exact_digest = crate::migrate::body_digest(exact_body);
    if record.preserved_body.as_bytes() != exact_body.as_bytes()
        || record.preserved_body_digest != exact_digest
    {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} imported body differs from exact Git source body in {artifact}"
        )));
    }
    Ok(exact_digest)
}

fn verify_source_capsule(
    receipt: &LegacyAdrDispositionReceipt,
    binding: &ContentBinding,
    warrant_repo: &Utf8Path,
    source_bytes: &[u8],
    snapshot: &RepositorySnapshot,
) -> Result<(), RepoError> {
    let (path, bytes) = read_repository_bounded(
        warrant_repo,
        &binding.reference,
        "excluded source capsule",
        MAX_SUPPORT_BYTES,
    )?;
    verify_raw_binding(binding, &bytes)?;
    snapshot.verify_bytes(&binding.reference, &bytes, MAX_SUPPORT_BYTES)?;
    let capsule: openwarrant_core::legacy_disposition::LegacyAdrSourceCapsule =
        openwarrant_core::legacy_disposition::parse_legacy_json(&bytes).map_err(|error| {
            RepoError::Message(format!(
                "excluded source capsule {path} is invalid: {error}"
            ))
        })?;
    if capsule.schema != "oh.war/legacy-adr-source-capsule/v1"
        || capsule.source != receipt.payload.source
        || capsule.exact_source.as_bytes() != source_bytes
    {
        return Err(RepoError::Message(format!(
            "ADR {} excluded source capsule does not preserve exact bound source",
            receipt.payload.source.adr_id
        )));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn verify_support(
    warrant_repo: &Utf8Path,
    source_repo: &Utf8Path,
    receipt: &LegacyAdrDispositionReceipt,
    checked_commits: &mut BTreeSet<String>,
    checked_refs: &mut CheckedRefs,
    snapshot: &RepositorySnapshot,
    state: &mut SupportState,
    internal_artifacts: &InternalArtifacts,
    resolution_context: Option<ResolutionVerificationContext<'_>>,
) -> Result<(), RepoError> {
    for support in &receipt.payload.evidence {
        match support {
            LegacyAdrSupport::ImplementationArtifact { artifact, .. } => {
                reject_internal_support(receipt, artifact, internal_artifacts)?;
                let cached = verify_support_binding(
                    artifact,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let Some((commit, path)) = cached.identity else {
                    return Err(RepoError::Message(format!(
                        "ADR {} implementation evidence must be git:<commit>:<path>",
                        receipt.payload.source.adr_id
                    )));
                };
                reject_source_derived_support(receipt, artifact, Some((&commit, &path)))?;
                if is_adr_record_path(&path) {
                    return Err(RepoError::Message(format!(
                        "ADR {} implementation evidence is another ADR record, which is historical context",
                        receipt.payload.source.adr_id
                    )));
                }
            }
            LegacyAdrSupport::RequiredGatePass {
                run,
                definition,
                binding,
                selection,
                stdout,
                stderr,
                fixtures,
                receipt: gate_receipt,
                subject_source_sha256,
                ..
            } => {
                let context = resolution_context.ok_or_else(|| {
                    RepoError::Message(format!(
                        "ADR {} Gate evidence requires repository and authority context",
                        receipt.payload.source.adr_id
                    ))
                })?;
                let structural_refs: BTreeSet<&str> = [
                    run,
                    definition,
                    binding,
                    selection,
                    stdout,
                    stderr,
                    gate_receipt,
                ]
                .into_iter()
                .map(|artifact| artifact.reference.as_str())
                .chain(fixtures.iter().map(|fixture| fixture.reference.as_str()))
                .collect();
                if structural_refs.len() != 7 + fixtures.len() {
                    return Err(RepoError::Message(format!(
                        "ADR {} Gate receipt bundle reuses a structural artifact reference",
                        receipt.payload.source.adr_id
                    )));
                }
                if subject_source_sha256 != &receipt.payload.source.source_sha256 {
                    return Err(RepoError::Message(format!(
                        "ADR {} gate evidence binds a different source digest",
                        receipt.payload.source.adr_id
                    )));
                }
                for artifact in [
                    run,
                    definition,
                    binding,
                    selection,
                    stdout,
                    stderr,
                    gate_receipt,
                ] {
                    reject_internal_support(receipt, artifact, internal_artifacts)?;
                }
                for fixture in fixtures {
                    reject_internal_support(receipt, fixture, internal_artifacts)?;
                }
                let run = verify_support_binding(
                    run,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let definition = verify_support_binding(
                    definition,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let binding = verify_support_binding(
                    binding,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let selection = verify_support_binding(
                    selection,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let stdout = verify_support_binding(
                    stdout,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let stderr = verify_support_binding(
                    stderr,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let fixtures = fixtures
                    .iter()
                    .map(|fixture| {
                        verify_support_binding(
                            fixture,
                            warrant_repo,
                            source_repo,
                            checked_commits,
                            snapshot,
                            state,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let gate_receipt = verify_support_binding(
                    gate_receipt,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                let _admitted = verify_required_gate_pass(
                    &receipt.payload.source.adr_id,
                    subject_source_sha256,
                    &receipt.payload.review.reviewed_at,
                    &run,
                    &definition,
                    &binding,
                    &selection,
                    &stdout,
                    &stderr,
                    &fixtures,
                    &gate_receipt,
                    context,
                )?;
            }
            LegacyAdrSupport::FalsificationObservation {
                record,
                subject_source_sha256,
                ..
            } => {
                if subject_source_sha256 != &receipt.payload.source.source_sha256 {
                    return Err(RepoError::Message(format!(
                        "ADR {} falsification evidence binds a different source digest",
                        receipt.payload.source.adr_id
                    )));
                }
                reject_internal_support(receipt, record, internal_artifacts)?;
                let record = verify_support_binding(
                    record,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
                verify_falsification_record(
                    receipt,
                    subject_source_sha256,
                    &record,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                    internal_artifacts,
                )?;
            }
            LegacyAdrSupport::HistoricalContext { artifact, .. } => {
                reject_internal_support(receipt, artifact, internal_artifacts)?;
                verify_support_binding(
                    artifact,
                    warrant_repo,
                    source_repo,
                    checked_commits,
                    snapshot,
                    state,
                )?;
            }
        }
    }
    for successor in &receipt.payload.successors {
        match successor {
            LegacyAdrSuccessor::Adr { source, .. } => verify_successor_source(
                source_repo,
                receipt,
                source,
                checked_commits,
                checked_refs,
            )?,
            LegacyAdrSuccessor::ResolvedWarrant {
                warrant_uuid,
                warrant,
                resolution,
                resolution_digest,
                ..
            } => {
                let context = resolution_context.ok_or_else(|| {
                    RepoError::Message(format!(
                        "ADR {} resolved Warrant successor {} has no repository verification context",
                        receipt.payload.source.adr_id, warrant.warrant_id
                    ))
                })?;
                let expected_uuid = warrant_uuid.to_string();
                verify_resolved_warrant(
                    context,
                    &expected_uuid,
                    warrant,
                    resolution,
                    resolution_digest,
                    snapshot,
                )?;
            }
        }
    }
    Ok(())
}

fn verify_resolved_warrant(
    context: ResolutionVerificationContext<'_>,
    expected_uuid: &str,
    warrant: &WarrantBinding,
    resolution_binding: &ContentBinding,
    resolution_digest: &str,
    snapshot: &RepositorySnapshot,
) -> Result<(), RepoError> {
    let verified = verify_warrant(
        context.repo,
        warrant,
        expected_uuid,
        context.authority,
        snapshot,
        context.verification_as_of,
    )?;
    let (path, bytes) = read_repository_bounded(
        &context.repo.root,
        &resolution_binding.reference,
        "Warrant Resolution",
        MAX_RESOLUTION_BYTES,
    )?;
    snapshot.verify_bytes(&resolution_binding.reference, &bytes, MAX_RESOLUTION_BYTES)?;
    let bound = parse_bound_resolution(
        resolution_binding,
        resolution_digest,
        warrant,
        &bytes,
        context.authority,
        &verified.authorization_effective_at,
        context.verification_as_of,
    )
    .map_err(|error| {
        RepoError::Message(format!(
            "resolved Warrant {} has invalid Resolution {path}: {error}",
            warrant.warrant_id
        ))
    })?;

    let evidence =
        load_pinned_resolution_evidence(context.repo, &verified, context.authority, snapshot)?;
    let recorded_at =
        canonical_utc_unix_seconds(&bound.resolution.recorded_at).ok_or_else(|| {
            RepoError::Message(format!(
                "Resolution {} recorded_at is not a canonical UTC instant",
                bound.resolution.id
            ))
        })?;
    if evidence
        .gate_receipts
        .iter()
        .any(|receipt| receipt.completed_at > bound.resolution.recorded_at)
    {
        return Err(RepoError::Message(format!(
            "Resolution {} predates a bound Gate receipt completion",
            bound.resolution.id
        )));
    }
    let evidence_paths =
        pinned_resolution_evidence_paths(context.repo, &verified, &evidence, warrant)?;
    let resolution_commit = snapshot.path_commit_identity(&resolution_binding.reference)?;
    let mut evidence_commits = Vec::with_capacity(evidence_paths.len());
    for path in evidence_paths {
        let committed = snapshot.path_commit_identity(&path)?;
        if committed.committed_unix_seconds > recorded_at {
            return Err(RepoError::Message(format!(
                "Resolution {} predates pinned evidence {path:?}",
                bound.resolution.id
            )));
        }
        snapshot.require_strict_ancestor(&committed.commit, &resolution_commit.commit)?;
        evidence_commits.push(committed);
    }
    let performer = context.repo.performer();
    let evaluation =
        crate::resolve::evaluate_recorded_resolution(crate::resolve::RecordedResolutionInputs {
            one: &verified.loaded,
            verifications: &evidence.verifications,
            deliverables: &evidence.deliverables,
            gate_runs: &evidence.gate_runs,
            admissible_gate_runs: &evidence.admissible_gate_runs,
            gate_definitions: &evidence.gate_definitions,
            gate_bindings: &evidence.gate_bindings,
            gate_receipts: &evidence.gate_receipts,
            gate_outputs: &evidence.gate_outputs,
            authorization: &verified.authorization,
            current_contract_digest: &verified.current_contract_digest,
            judgments: &evidence.judgments,
            assumptions: evidence.assumptions.as_deref(),
            register: context.authority,
            policy_allows_automated_resolution: context
                .repo
                .config
                .policy
                .allow_automated_resolution,
            performer: &performer,
            resolver: crate::resolve::RecordedResolver {
                actor: &bound.actor,
                effective_at: &bound.resolution.effective_at,
            },
            artifacts: evidence.artifact_checks,
            artifact_entries: &evidence.artifact_entries,
            sources: crate::resolve::RecordedEvidenceSources {
                verification_refs: &evidence.verification_refs,
                deliverables_ref: &evidence.deliverables_ref,
                gate_run_source_refs: &evidence.gate_run_source_refs,
                gate_binding_source_refs: &evidence.gate_binding_source_refs,
                gate_selection_source_refs: &evidence.gate_selection_source_refs,
                gate_fixture_source_refs: &evidence.gate_fixture_source_refs,
                gate_raw_evidence_source_refs: &evidence.gate_raw_evidence_source_refs,
                gate_receipt_source_refs: &evidence.gate_receipt_source_refs,
                evidence_commits: &evidence_commits,
                judgments_ref: evidence.judgments_ref.as_deref(),
                rationale_ref: evidence.rationale_ref.as_deref(),
            },
        })?;
    validate_resolution_evaluation(&bound.resolution, &evaluation)
}

fn pinned_resolution_evidence_paths(
    repo: &Repository,
    verified: &VerifiedWarrant,
    evidence: &PinnedResolutionEvidence,
    warrant: &WarrantBinding,
) -> Result<Vec<String>, RepoError> {
    let mut paths = evidence.verification_refs.clone();
    paths.push(evidence.deliverables_ref.clone());
    paths.extend(evidence.gate_run_source_refs.iter().cloned());
    paths.extend(
        evidence
            .gate_definitions
            .iter()
            .map(|definition| definition.source_ref.clone()),
    );
    paths.extend(evidence.gate_binding_source_refs.iter().cloned());
    paths.extend(evidence.gate_selection_source_refs.iter().cloned());
    paths.extend(evidence.gate_fixture_source_refs.iter().cloned());
    paths.extend(evidence.gate_raw_evidence_source_refs.iter().cloned());
    paths.extend(evidence.gate_receipt_source_refs.iter().cloned());
    paths.extend(
        evidence
            .gate_outputs
            .iter()
            .map(|output| output.source_ref.clone()),
    );
    paths.extend(evidence.judgments_ref.iter().cloned());
    paths.extend(evidence.rationale_ref.iter().cloned());
    paths.push("openwarrant.toml".to_owned());
    paths.push("docs/authority/roles.toml".to_owned());
    paths.extend(
        evidence
            .artifact_entries
            .iter()
            .map(|entry| entry.target_ref.clone()),
    );
    paths.push(warrant.authorization_ref.clone());
    if let Some(basis) = &verified.loaded.basis {
        paths.push(basis.manifest_source.clone());
        let warrant_ref = repository_relative(&repo.root, &verified.loaded.dir)?;
        for atom in &basis.atoms {
            paths.push(normalize_git_path(&format!(
                "{warrant_ref}/{}",
                atom.source
            ))?);
        }
        if let Some(scope) = &basis.scope {
            paths.push(scope.source.clone());
        }
    }
    paths.sort();
    paths.dedup();
    if paths.len() > MAX_RESOLUTION_COMMIT_PATHS {
        return Err(RepoError::Message(format!(
            "Resolution commit-bound evidence path count {} exceeds {MAX_RESOLUTION_COMMIT_PATHS}",
            paths.len()
        )));
    }
    Ok(paths)
}

fn read_pinned_toml<T: DeserializeOwned>(
    snapshot: &RepositorySnapshot,
    path: &str,
    label: &str,
    total_bytes: &mut u64,
) -> Result<T, RepoError> {
    read_pinned_toml_with_bytes(snapshot, path, label, total_bytes).map(|(value, _)| value)
}

fn read_pinned_toml_with_bytes<T: DeserializeOwned>(
    snapshot: &RepositorySnapshot,
    path: &str,
    label: &str,
    total_bytes: &mut u64,
) -> Result<(T, Vec<u8>), RepoError> {
    let bytes = snapshot.read_blob(path, MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
    *total_bytes = total_bytes
        .checked_add(bytes.len() as u64)
        .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
        .ok_or_else(|| {
            RepoError::Message(format!(
                "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
            ))
        })?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| RepoError::Message(format!("{label} {path:?} is not UTF-8")))?;
    let value = toml::from_str(text).map_err(|error| {
        RepoError::Message(format!("could not parse {label} {path:?}: {error}"))
    })?;
    Ok((value, bytes))
}

fn read_pinned_json<T: DeserializeOwned>(
    snapshot: &RepositorySnapshot,
    path: &str,
    label: &str,
    total_bytes: &mut u64,
) -> Result<T, RepoError> {
    read_pinned_json_with_bytes(snapshot, path, label, total_bytes).map(|(value, _)| value)
}

fn read_pinned_json_with_bytes<T: DeserializeOwned>(
    snapshot: &RepositorySnapshot,
    path: &str,
    label: &str,
    total_bytes: &mut u64,
) -> Result<(T, Vec<u8>), RepoError> {
    let bytes = snapshot.read_blob(path, MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
    *total_bytes = total_bytes
        .checked_add(bytes.len() as u64)
        .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
        .ok_or_else(|| {
            RepoError::Message(format!(
                "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
            ))
        })?;
    let value =
        openwarrant_core::legacy_disposition::parse_strict_json(&bytes).map_err(|error| {
            RepoError::Message(format!("could not parse {label} {path:?}: {error}"))
        })?;
    Ok((value, bytes))
}

fn load_pinned_resolution_evidence(
    repo: &Repository,
    verified: &VerifiedWarrant,
    authority: &AuthorityRegister,
    snapshot: &RepositorySnapshot,
) -> Result<PinnedResolutionEvidence, RepoError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct DeliverableFile {
        #[serde(default)]
        deliverable: Vec<Deliverable>,
    }

    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct RationaleFile {
        #[serde(default)]
        assumption: Vec<Assumption>,
    }

    let warrant_ref = repository_relative(&repo.root, &verified.loaded.dir)?;
    let mut total_bytes = 0_u64;

    let verification_prefix = format!("{warrant_ref}/verifications");
    let verification_refs: Vec<String> = snapshot
        .list_files(&verification_prefix)?
        .into_iter()
        .filter(|path| path.ends_with(".toml"))
        .collect();
    let mut verifications = Vec::with_capacity(verification_refs.len());
    for path in &verification_refs {
        verifications.push(read_pinned_toml::<Verification>(
            snapshot,
            path,
            "verification record",
            &mut total_bytes,
        )?);
    }

    let deliverables_ref = format!("{warrant_ref}/deliverables.toml");
    let deliverables = if snapshot
        .list_files(&deliverables_ref)?
        .iter()
        .any(|path| path == &deliverables_ref)
    {
        read_pinned_toml::<DeliverableFile>(
            snapshot,
            &deliverables_ref,
            "deliverables record",
            &mut total_bytes,
        )?
        .deliverable
    } else {
        Vec::new()
    };
    if deliverables.len() > MAX_RESOLUTION_DELIVERABLES {
        return Err(RepoError::Message(format!(
            "Resolution declares {} deliverables; limit is {MAX_RESOLUTION_DELIVERABLES}",
            deliverables.len()
        )));
    }

    let cited_gates: BTreeSet<String> = verified
        .loaded
        .basis
        .as_ref()
        .into_iter()
        .flat_map(|basis| &basis.atoms)
        .filter(|atom| atom.role == "assurance")
        .flat_map(|atom| {
            openwarrant_core::gate::cited_gate_uris(&String::from_utf8_lossy(&atom.bytes))
        })
        .map(|uri| uri.trim_start_matches("gate://").to_owned())
        .collect();
    let gate_prefix = repo.config.paths.gates.as_str();
    let mut gate_definitions_by_key = BTreeMap::new();
    for path in snapshot
        .list_files(gate_prefix)?
        .into_iter()
        .filter(|path| path.ends_with(".yaml") || path.ends_with(".yml"))
    {
        let file_key = path.rsplit('/').next().and_then(|name| {
            name.strip_suffix(".yaml")
                .or_else(|| name.strip_suffix(".yml"))
        });
        let path_names_cited_gate = file_key.is_some_and(|key| cited_gates.contains(key));
        let bytes = snapshot.read_blob(&path, MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
        let text = match std::str::from_utf8(&bytes) {
            Ok(text) => text,
            Err(_) if !path_names_cited_gate => continue,
            Err(_) => {
                return Err(RepoError::Message(format!(
                    "Gate Definition {path:?} is not UTF-8"
                )));
            }
        };
        let document = match openwarrant_core::structured::parse(text) {
            Ok(document) => document,
            Err(_) if !path_names_cited_gate => continue,
            Err(error) => {
                return Err(RepoError::Message(format!(
                    "could not parse Gate Definition {path:?}: {error}"
                )));
            }
        };
        let declared_key = document
            .scalar("gate_id")
            .zip(document.scalar("version"))
            .map(|(id, version)| format!("{id}@{version}"));
        if !path_names_cited_gate
            && !declared_key
                .as_ref()
                .is_some_and(|key| cited_gates.contains(key))
        {
            continue;
        }
        let mut definition = openwarrant_core::gate::definition_from_structured(&document)
            .map_err(|error| {
                RepoError::Message(format!("invalid Gate Definition {path:?}: {error}"))
            })?;
        let observed_sha256 = content_digest(&bytes);
        definition
            .bind_local_source_digest(&observed_sha256)
            .map_err(|error| {
                RepoError::Message(format!("invalid Gate Definition {path:?}: {error}"))
            })?;
        let key = definition.key();
        if !cited_gates.contains(&key) {
            continue;
        }
        total_bytes = total_bytes
            .checked_add(bytes.len() as u64)
            .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
                ))
            })?;
        let recorded = crate::resolve::RecordedGateDefinition {
            source_ref: path,
            observed_sha256,
            definition,
        };
        if gate_definitions_by_key
            .insert(key.clone(), recorded)
            .is_some()
        {
            return Err(RepoError::Message(format!(
                "Resolution evidence contains duplicate Gate Definitions for {key:?}"
            )));
        }
    }
    if let Some(missing) = cited_gates
        .iter()
        .find(|gate| !gate_definitions_by_key.contains_key(*gate))
    {
        return Err(RepoError::Message(format!(
            "Resolution evidence cannot resolve cited Gate Definition {missing:?}"
        )));
    }
    let receipt_prefix = repo.config.paths.receipts.as_str();
    let receipt_inventory = snapshot.list_files(receipt_prefix)?;

    // A receipt is the bundle commit marker. Raw `.run.toml` files are history,
    // not candidates: start from each exact receipt name, then derive its one
    // sibling run. This keeps abandoned or hand-written runs outside the
    // Resolution evidence set even when they claim askable/completed/pass.
    let receipt_files: BTreeSet<&str> = receipt_inventory.iter().map(String::as_str).collect();
    let receipt_prefix = receipt_prefix.trim_end_matches('/');
    let mut gate_bundle_candidates = Vec::new();
    let mut total_gate_fixture_refs = 0_usize;
    for receipt_path in receipt_inventory
        .iter()
        .filter(|path| path.ends_with(".receipt.json"))
    {
        let Some(run_id) = receipt_path
            .rsplit('/')
            .next()
            .and_then(|name| name.strip_suffix(".receipt.json"))
        else {
            continue;
        };
        if !crate::gate_cmd::canonical_run_id(run_id) {
            continue;
        }
        let expected_receipt_path = format!("{receipt_prefix}/{run_id}.receipt.json");
        if *receipt_path != expected_receipt_path {
            continue;
        }
        let run_path = format!("{receipt_prefix}/{run_id}.run.toml");
        if !receipt_files.contains(run_path.as_str()) {
            continue;
        }
        let Ok((run, run_bytes)) = read_pinned_toml_with_bytes::<GateRun>(
            snapshot,
            &run_path,
            "Gate Run",
            &mut total_bytes,
        ) else {
            continue;
        };
        let gate_run_digest = content_digest(&run_bytes);
        if !cited_gates.contains(&run.gate) {
            continue;
        }
        run.validate().map_err(|error| {
            RepoError::Message(format!("invalid Gate Run {run_path:?}: {error}"))
        })?;
        if run.id != run_id {
            return Err(RepoError::Message(format!(
                "Gate Run {run_path:?} identity does not match receipt {receipt_path:?}"
            )));
        }
        let receipt = read_pinned_json::<GateReceipt>(
            snapshot,
            receipt_path,
            "Gate receipt",
            &mut total_bytes,
        )?;
        receipt.validate().map_err(|error| {
            RepoError::Message(format!("invalid Gate receipt {receipt_path:?}: {error}"))
        })?;
        if receipt.run_id != run_id {
            return Err(RepoError::Message(format!(
                "Gate receipt {receipt_path:?} identity does not match its exact bundle path"
            )));
        }
        let recorded_digest = receipt.receipt_digest.clone();
        let mut preimage = receipt.clone();
        preimage.receipt_digest.clear();
        let recomputed_digest = sha256_digest(DigestDomain::GateReceipt, &preimage)
            .map(|digest| format!("sha256:{digest}"))
            .map_err(|error| {
                RepoError::Message(format!(
                    "cannot canonicalize Gate receipt {receipt_path:?}: {error}"
                ))
            })?;
        if recorded_digest != recomputed_digest {
            return Err(RepoError::Message(format!(
                "Gate receipt {receipt_path:?} has invalid canonical receipt digest"
            )));
        }
        let binding_path = format!("{receipt_prefix}/{run_id}.binding.json");
        if !receipt_files.contains(binding_path.as_str()) {
            return Err(RepoError::Message(format!(
                "cited Gate receipt {receipt_path:?} has no exact sibling Gate Binding"
            )));
        }
        let binding = read_pinned_json::<GateBinding>(
            snapshot,
            &binding_path,
            "Gate Binding",
            &mut total_bytes,
        )?;
        if binding.id.trim().is_empty()
            || binding.gate.id.trim().is_empty()
            || binding.gate.version.trim().is_empty()
            || binding.gate.digest.trim().is_empty()
            || binding.subjects.is_empty()
        {
            return Err(RepoError::Message(format!(
                "Gate Binding {binding_path:?} is incomplete"
            )));
        }
        total_gate_fixture_refs = total_gate_fixture_refs
            .checked_add(binding.fixtures.len())
            .filter(|count| *count <= MAX_RESOLUTION_GATE_FIXTURES)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "Resolution Gate Binding fixture count exceeds {MAX_RESOLUTION_GATE_FIXTURES}"
                ))
            })?;
        let binding_digest = sha256_digest(DigestDomain::GateBinding, &binding)
            .map(|digest| format!("sha256:{digest}"))
            .map_err(|error| {
                RepoError::Message(format!(
                    "cannot canonicalize Gate Binding {binding_path:?}: {error}"
                ))
            })?;
        if binding_digest != receipt.gate_binding_digest {
            return Err(RepoError::Message(format!(
                "cited Gate receipt {receipt_path:?} does not bind its exact sibling Gate Binding"
            )));
        }
        let selection_path = format!("{receipt_prefix}/{run_id}.selection.json");
        if !receipt_files.contains(selection_path.as_str()) {
            return Err(RepoError::Message(format!(
                "cited Gate receipt {receipt_path:?} has no exact sibling test-selection observation"
            )));
        }
        let (selection, selection_bytes) =
            read_pinned_json_with_bytes::<openwarrant_core::TestSelectionObservation>(
                snapshot,
                &selection_path,
                "test-selection observation",
                &mut total_bytes,
            )?;
        selection.validate().map_err(|error| {
            RepoError::Message(format!(
                "invalid test-selection observation {selection_path:?}: {error}"
            ))
        })?;
        let selection_digest = content_digest(&selection_bytes);
        if receipt.selection_observation_ref != selection_path
            || receipt.selection_observation_digest != selection_digest
        {
            return Err(RepoError::Message(format!(
                "cited Gate receipt {receipt_path:?} does not bind its exact sibling test-selection observation"
            )));
        }
        gate_bundle_candidates.push((
            run_path,
            run,
            gate_run_digest,
            receipt_path.clone(),
            receipt,
            binding_path,
            binding,
            selection_path,
            selection,
            selection_digest,
        ));
    }

    let judgments_path = format!("{warrant_ref}/judgments.toml");
    let (judgments, judgments_ref) = if snapshot
        .list_files(&judgments_path)?
        .iter()
        .any(|path| path == &judgments_path)
    {
        let record = read_pinned_toml::<JudgmentRecord>(
            snapshot,
            &judgments_path,
            "judgment record",
            &mut total_bytes,
        )?;
        if record.warrant != verified.loaded.alias() {
            return Err(RepoError::Message(format!(
                "judgment record {judgments_path:?} belongs to {:?}, not {}",
                record.warrant,
                verified.loaded.alias()
            )));
        }
        (record.judgment, Some(judgments_path))
    } else {
        (Vec::new(), None)
    };

    let rationale_path = format!("{warrant_ref}/rationale.toml");
    let (assumptions, rationale_ref) = if snapshot
        .list_files(&rationale_path)?
        .iter()
        .any(|path| path == &rationale_path)
    {
        let record = read_pinned_toml::<RationaleFile>(
            snapshot,
            &rationale_path,
            "rationale record",
            &mut total_bytes,
        )?;
        for assumption in &record.assumption {
            assumption.validate().map_err(|error| {
                RepoError::Message(format!(
                    "invalid assumption {:?} in {rationale_path:?}: {error}",
                    assumption.id
                ))
            })?;
        }
        (Some(record.assumption), Some(rationale_path))
    } else {
        (None, None)
    };

    let declared = crate::resolve::declared_obligations(&verified.loaded);
    let required: Vec<&Deliverable> = deliverables
        .iter()
        .filter(|deliverable| deliverable.required)
        .collect();
    let addressed: Vec<&Deliverable> = deliverables
        .iter()
        .filter(|deliverable| deliverable.content_addressed)
        .collect();
    let mut observed = BTreeMap::<String, String>::new();
    let mut inspected_targets = BTreeSet::new();
    for deliverable in required.iter().chain(addressed.iter()) {
        if !inspected_targets.insert(deliverable.target_ref.clone()) {
            continue;
        }
        if inspected_targets.len() > MAX_RESOLUTION_ARTIFACT_TARGETS {
            return Err(RepoError::Message(format!(
                "Resolution artifact target count exceeds {MAX_RESOLUTION_ARTIFACT_TARGETS}"
            )));
        }
        validate_git_path(&deliverable.target_ref)?;
        let exact = snapshot
            .list_files(&deliverable.target_ref)?
            .into_iter()
            .any(|path| path == deliverable.target_ref);
        if !exact {
            continue;
        }
        let bytes = snapshot.read_blob(&deliverable.target_ref, MAX_RESOLUTION_ARTIFACT_BYTES)?;
        total_bytes = total_bytes
            .checked_add(bytes.len() as u64)
            .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
            .ok_or_else(|| {
                RepoError::Message(format!(
                    "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
                ))
            })?;
        observed.insert(deliverable.target_ref.clone(), content_digest(&bytes));
    }
    let required_deliverables_exist = !required.is_empty()
        && required.iter().all(|deliverable| {
            deliverable.validate(&declared).is_ok()
                && observed.contains_key(&deliverable.target_ref)
        });
    let artifact_digests_verify = !addressed.is_empty()
        && addressed.iter().all(|deliverable| {
            deliverable.provenance.as_ref().is_some_and(|provenance| {
                observed
                    .get(&deliverable.target_ref)
                    .is_some_and(|digest| digest == &provenance.content_digest)
            })
        });
    let artifact_entries: Vec<crate::resolve::ArtifactSnapshotEntry> = deliverables
        .iter()
        .filter_map(|deliverable| {
            observed.get(&deliverable.target_ref).map(|digest| {
                crate::resolve::ArtifactSnapshotEntry {
                    deliverable_id: deliverable.id.clone(),
                    target_ref: deliverable.target_ref.clone(),
                    observed_sha256: digest.clone(),
                }
            })
        })
        .collect();

    let resolution_evidence_basis = crate::gate_evidence::ResolutionEvidenceBasis::new(
        &verified.current_contract_digest,
        artifact_entries.iter().map(
            |artifact| crate::gate_evidence::ResolutionEvidenceArtifact {
                deliverable_id: artifact.deliverable_id.clone(),
                target_ref: artifact.target_ref.clone(),
                observed_sha256: artifact.observed_sha256.clone(),
            },
        ),
        crate::authorize::gate_bindings_for_current(
            &verified.authorization,
            &verified.current_contract_digest,
        )
        .into_iter()
        .flatten()
        .cloned(),
    )
    .map_err(|error| RepoError::Message(format!("invalid Resolution evidence basis: {error}")))?;
    let mut gate_runs = Vec::new();
    let mut admissible_gate_runs = Vec::new();
    let mut gate_run_source_refs = Vec::new();
    let mut gate_bindings = Vec::new();
    let mut gate_binding_source_refs = Vec::new();
    let mut gate_selection_source_refs = Vec::new();
    let mut gate_fixture_source_refs = Vec::new();
    let mut gate_raw_evidence_source_refs = Vec::new();
    let mut inspected_gate_fixtures = BTreeMap::<String, String>::new();
    let mut gate_receipts = Vec::new();
    let mut gate_receipt_source_refs = Vec::new();
    let mut gate_outputs = Vec::new();
    let mut inspected_gate_output_refs = BTreeSet::new();
    let mut inspected_gate_raw_evidence = BTreeMap::<String, String>::new();
    let performer = repo.performer();
    for (
        run_path,
        run,
        gate_run_digest,
        receipt_path,
        receipt,
        binding_path,
        binding,
        selection_path,
        selection,
        selection_digest,
    ) in gate_bundle_candidates
    {
        let recorded_definition = gate_definitions_by_key.get(&run.gate).ok_or_else(|| {
            RepoError::Message(format!(
                "cited Gate Run {} has no exact Gate Definition",
                run.id
            ))
        })?;
        let definition = &recorded_definition.definition;
        let binding_subjects: BTreeSet<String> = binding.subjects.iter().cloned().collect();
        let receipt_subjects: BTreeSet<String> = receipt.subject_digests.iter().cloned().collect();
        let binding_fixture_digests: BTreeSet<String> = binding
            .fixtures
            .iter()
            .map(|item| item.digest.clone())
            .collect();
        let binding_fixture_refs: BTreeSet<String> = binding
            .fixtures
            .iter()
            .map(|item| item.reference.clone())
            .collect();
        let receipt_fixture_digests: BTreeSet<String> =
            receipt.fixture_digests.iter().cloned().collect();
        let selected_tests: BTreeSet<String> =
            receipt.selected_test_manifest.iter().cloned().collect();
        let raw_evidence: BTreeSet<String> = receipt.raw_evidence_refs.iter().cloned().collect();
        if u64::try_from(receipt.selected_test_manifest.len()).ok()
            != Some(receipt.selected_test_count)
            || selected_tests.len() != receipt.selected_test_manifest.len()
            || selected_tests.iter().any(|test| test.trim().is_empty())
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} receipt has an invalid selected-test manifest",
                run.id
            )));
        }
        if binding_fixture_digests.len() != binding.fixtures.len()
            || binding_fixture_refs.len() != binding.fixtures.len()
            || receipt_fixture_digests.len() != receipt.fixture_digests.len()
            || binding_fixture_digests != receipt_fixture_digests
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} Gate Binding fixture inventory does not match its receipt",
                run.id
            )));
        }
        if receipt.producer_actor != binding.evidence_policy.producer {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} receipt producer {:?} does not match Gate Binding producer {:?}",
                run.id, receipt.producer_actor, binding.evidence_policy.producer
            )));
        }
        if receipt.producer_actor == repo.performer()
            && !binding.evidence_policy.performer_authored_report_admissible
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} Gate Binding forbids a performer-authored report",
                run.id
            )));
        }
        if raw_evidence.len() != receipt.raw_evidence_refs.len() {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} receipt raw evidence contains duplicates",
                run.id
            )));
        }
        for reference in &receipt.raw_evidence_refs {
            match resolution_evidence_basis
                .classify_raw_evidence(reference)
                .map_err(|error| {
                    RepoError::Message(format!(
                        "cited Gate Run {} has inadmissible raw evidence: {error}",
                        run.id
                    ))
                })? {
                crate::gate_evidence::ResolutionRawEvidence::Artifact { .. } => {}
                crate::gate_evidence::ResolutionRawEvidence::File { path, sha256 } => {
                    validate_git_path(path.as_str())?;
                    if let Some(observed) = inspected_gate_raw_evidence.get(path.as_str()) {
                        if observed != &sha256 {
                            return Err(RepoError::Message(format!(
                                "cited Gate Run {} reuses raw evidence {:?} with a different digest",
                                run.id, path
                            )));
                        }
                        continue;
                    }
                    let bytes =
                        snapshot.read_blob(path.as_str(), MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
                    total_bytes = total_bytes
                        .checked_add(bytes.len() as u64)
                        .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
                        .ok_or_else(|| {
                            RepoError::Message(format!(
                                "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
                            ))
                        })?;
                    let observed = content_digest(&bytes);
                    if observed != sha256 {
                        return Err(RepoError::Message(format!(
                            "cited Gate Run {} raw evidence {:?} digest mismatch",
                            run.id, path
                        )));
                    }
                    inspected_gate_raw_evidence.insert(path.to_string(), observed);
                    gate_raw_evidence_source_refs.push(path.to_string());
                }
            }
        }
        if definition.key() != run.gate
            || !definition.lifecycle.is_bindable()
            || binding.gate.digest != definition.digest
            || receipt.gate_definition_digest != definition.digest
            || receipt.arguments != definition.argv
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} does not bind the exact qualified Gate Definition and argument vector",
                run.id
            )));
        }
        if definition.mutating {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} uses a mutating gate, which is not admissible for Resolution",
                run.id
            )));
        }
        if !run.satisfies_required_pass()
            || binding.gate.key() != run.gate
            || receipt.run_id != run.id
            || receipt.verdict != openwarrant_core::Verdict::Pass
            || receipt.gate_definition_digest != binding.gate.digest
            || receipt.selected_test_count == 0
            || binding_subjects.len() != binding.subjects.len()
            || receipt_subjects.len() != receipt.subject_digests.len()
            || binding_subjects != receipt_subjects
            || !is_canonical_utc(&receipt.started_at)
            || !is_canonical_utc(&receipt.completed_at)
            || receipt.started_at > receipt.completed_at
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} is not exactly bound to its passing receipt, Gate Binding, authorized contract, and artifact snapshot",
                run.id
            )));
        }
        let producer_assignment = authority.actor(&receipt.producer_actor).ok_or_else(|| {
            RepoError::Message(format!(
                "cited Gate Run {} producer actor {:?} has no authority assignment",
                run.id, receipt.producer_actor
            ))
        })?;
        if producer_assignment.validate().is_err()
            || !producer_assignment.holds(ActorRole::Verifier)
            || !is_canonical_utc(&producer_assignment.effective_time)
            || producer_assignment.effective_time > receipt.started_at
        {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} producer actor {:?} does not hold an effective Verifier assignment at execution",
                run.id, receipt.producer_actor
            )));
        }
        let expected_stdout_ref = format!("{receipt_prefix}/{}.stdout.txt", run.id);
        let expected_stderr_ref = format!("{receipt_prefix}/{}.stderr.txt", run.id);
        if receipt.stdout_ref != expected_stdout_ref || receipt.stderr_ref != expected_stderr_ref {
            return Err(RepoError::Message(format!(
                "cited Gate Run {} receipt does not name its exact sibling output objects",
                run.id
            )));
        }
        let mut stdout_observed_digest = None;
        let mut stderr_observed_digest = None;
        for (stream, reference, expected_digest) in [
            ("stdout", &receipt.stdout_ref, &receipt.stdout_digest),
            ("stderr", &receipt.stderr_ref, &receipt.stderr_digest),
        ] {
            validate_git_path(reference)?;
            if !inspected_gate_output_refs.insert(reference.clone()) {
                return Err(RepoError::Message(format!(
                    "cited Gate Run {} reuses Gate output reference {reference:?}",
                    run.id
                )));
            }
            let bytes = snapshot.read_blob(reference, MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
            total_bytes = total_bytes
                .checked_add(bytes.len() as u64)
                .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
                .ok_or_else(|| {
                    RepoError::Message(format!(
                        "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
                    ))
                })?;
            let observed_sha256 = content_digest(&bytes);
            if &observed_sha256 != expected_digest {
                return Err(RepoError::Message(format!(
                    "cited Gate Run {} {stream} {:?} digest mismatch",
                    run.id, reference
                )));
            }
            match stream {
                "stdout" => stdout_observed_digest = Some(observed_sha256.clone()),
                "stderr" => stderr_observed_digest = Some(observed_sha256.clone()),
                _ => unreachable!("fixed Gate output stream inventory"),
            }
            gate_outputs.push(crate::resolve::RecordedGateOutput {
                run_id: run.id.clone(),
                stream,
                source_ref: reference.clone(),
                observed_sha256,
            });
        }
        for fixture in &binding.fixtures {
            validate_git_path(&fixture.reference)?;
            if fixture.digest.trim().is_empty() {
                return Err(RepoError::Message(format!(
                    "cited Gate Run {} has a blank fixture digest",
                    run.id
                )));
            }
            if let Some(observed) = inspected_gate_fixtures.get(&fixture.reference) {
                if observed != &fixture.digest {
                    return Err(RepoError::Message(format!(
                        "cited Gate Run {} reuses fixture {:?} with a different digest",
                        run.id, fixture.reference
                    )));
                }
                continue;
            }
            let bytes =
                snapshot.read_blob(&fixture.reference, MAX_RESOLUTION_EVIDENCE_FILE_BYTES)?;
            total_bytes = total_bytes
                .checked_add(bytes.len() as u64)
                .filter(|total| *total <= MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES)
                .ok_or_else(|| {
                    RepoError::Message(format!(
                        "Resolution evidence exceeds aggregate {MAX_TOTAL_RESOLUTION_EVIDENCE_BYTES}-byte limit"
                    ))
                })?;
            let observed = content_digest(&bytes);
            if observed != fixture.digest {
                return Err(RepoError::Message(format!(
                    "cited Gate Run {} fixture {:?} digest mismatch",
                    run.id, fixture.reference
                )));
            }
            inspected_gate_fixtures.insert(fixture.reference.clone(), observed);
            gate_fixture_source_refs.push(fixture.reference.clone());
        }
        let stdout_observed_digest = stdout_observed_digest.ok_or_else(|| {
            RepoError::Message(format!("cited Gate Run {} has no stdout evidence", run.id))
        })?;
        let stderr_observed_digest = stderr_observed_digest.ok_or_else(|| {
            RepoError::Message(format!("cited Gate Run {} has no stderr evidence", run.id))
        })?;
        let admitted = crate::gate_evidence::admit_components(
            run.clone(),
            definition,
            &binding,
            &receipt,
            crate::gate_evidence::ObservedStreams {
                gate_run_digest: &gate_run_digest,
                selection_ref: &selection_path,
                selection_digest: &selection_digest,
                selection: &selection,
                stdout_ref: &receipt.stdout_ref,
                stdout_digest: &stdout_observed_digest,
                stderr_ref: &receipt.stderr_ref,
                stderr_digest: &stderr_observed_digest,
            },
            crate::gate_evidence::SubjectBasis::Resolution(&resolution_evidence_basis),
            &raw_evidence,
            authority,
            &performer,
        )
        .map_err(|error| {
            RepoError::Message(format!(
                "cited Gate Run {} failed receipt-bundle admission: {error}",
                run.id
            ))
        })?;
        gate_run_source_refs.push(run_path);
        gate_runs.push(run);
        admissible_gate_runs.push(admitted);
        gate_binding_source_refs.push(binding_path);
        gate_bindings.push(binding);
        gate_selection_source_refs.push(selection_path);
        gate_receipt_source_refs.push(receipt_path);
        gate_receipts.push(receipt);
    }

    Ok(PinnedResolutionEvidence {
        verifications,
        verification_refs,
        deliverables,
        deliverables_ref,
        gate_runs,
        admissible_gate_runs,
        gate_run_source_refs,
        gate_definitions: gate_definitions_by_key.into_values().collect(),
        gate_bindings,
        gate_binding_source_refs,
        gate_selection_source_refs,
        gate_fixture_source_refs,
        gate_raw_evidence_source_refs,
        gate_receipts,
        gate_receipt_source_refs,
        gate_outputs,
        judgments,
        judgments_ref,
        assumptions,
        rationale_ref,
        artifact_checks: crate::resolve::ArtifactChecks {
            required_deliverables_exist,
            artifact_digests_verify,
        },
        artifact_entries,
    })
}

fn parse_bound_resolution(
    binding: &ContentBinding,
    semantic_digest: &str,
    warrant: &WarrantBinding,
    bytes: &[u8],
    authority: &AuthorityRegister,
    authorization_effective_at: &str,
    verification_as_of: &str,
) -> Result<BoundResolution, RepoError> {
    verify_raw_binding(binding, bytes)?;
    reject_unknown_record_fields(
        &binding.reference,
        bytes,
        "Resolution",
        &[
            "id",
            "common_outcome",
            "profile_outcome",
            "contract_revision",
            "contract_digest",
            "assurance_case_snapshot_digest",
            "artifact_manifest_digest",
            "gate_run_refs",
            "judgment_refs",
            "residual_risk_refs",
            "resolved_by_ref",
            "acting_role_ref",
            "meaning",
            "effective_at",
            "recorded_at",
            "standing",
        ],
    )?;
    let resolution: Resolution = parse_support_record(&binding.reference, bytes, "Resolution")?;
    let resolution_id = WarUuid::from_str(&resolution.id).map_err(|error| {
        RepoError::Message(format!(
            "Resolution id {:?} is not an RFC 4122 UUIDv7: {error}",
            resolution.id
        ))
    })?;
    if resolution_id.to_string() != resolution.id {
        return Err(RepoError::Message(format!(
            "Resolution id {:?} is not canonical lowercase hyphenated UUIDv7",
            resolution.id
        )));
    }
    if !matches!(
        resolution.id.as_bytes().get(19),
        Some(b'8' | b'9' | b'a' | b'b')
    ) {
        return Err(RepoError::Message(format!(
            "Resolution id {:?} is not an RFC 4122 UUIDv7",
            resolution.id
        )));
    }
    if resolution.profile_outcome.trim().is_empty()
        || resolution.profile_outcome.chars().any(char::is_control)
    {
        return Err(RepoError::Message(format!(
            "Resolution {} has invalid blank or control-bearing profile outcome",
            resolution.id
        )));
    }
    let recomputed = sha256_digest(DigestDomain::Resolution, &resolution)
        .map(|digest| format!("sha256:{digest}"))
        .map_err(|error| RepoError::Message(format!("cannot canonicalize Resolution: {error}")))?;
    if recomputed != semantic_digest {
        return Err(RepoError::Message(format!(
            "Resolution semantic digest mismatch: recorded {semantic_digest}, recomputed {recomputed}"
        )));
    }
    if resolution.contract_revision != warrant.authorized_revision {
        return Err(RepoError::Message(format!(
            "Resolution contract revision {} does not match authorized revision {}",
            resolution.contract_revision, warrant.authorized_revision
        )));
    }
    if resolution.contract_digest != warrant.contract_digest {
        return Err(RepoError::Message(format!(
            "Resolution contract digest {} does not match authorized contract {}",
            resolution.contract_digest, warrant.contract_digest
        )));
    }
    if !resolution.common_outcome.accepts() {
        return Err(RepoError::Message(format!(
            "Resolution {} outcome {} does not accept its Warrant",
            resolution.id, resolution.common_outcome
        )));
    }
    resolution.require_reliable().map_err(|error| {
        RepoError::Message(format!("Resolution cannot be relied upon: {error}"))
    })?;
    if !is_canonical_utc(authorization_effective_at)
        || !is_canonical_utc(&resolution.effective_at)
        || !is_canonical_utc(&resolution.recorded_at)
        || !is_canonical_utc(verification_as_of)
        || authorization_effective_at > resolution.effective_at.as_str()
        || resolution.effective_at > resolution.recorded_at
        || resolution.recorded_at.as_str() > verification_as_of
    {
        return Err(RepoError::Message(format!(
            "Resolution {} chronology is invalid: authorization {authorization_effective_at:?}, effective {:?}, recorded {:?}, verification cutoff {verification_as_of:?}",
            resolution.id, resolution.effective_at, resolution.recorded_at
        )));
    }
    if resolution.acting_role_ref != "role-assignment://resolver" {
        return Err(RepoError::Message(format!(
            "Resolution {} acting role {:?} is not role-assignment://resolver",
            resolution.id, resolution.acting_role_ref
        )));
    }
    let (actor, expected_kind) = resolution_actor(&resolution.resolved_by_ref)?;
    let actor = actor.to_owned();
    let assignment = authority.actor(&actor).ok_or_else(|| {
        RepoError::Message(format!(
            "Resolution {} actor {actor:?} has no authority assignment",
            resolution.id
        ))
    })?;
    assignment.validate().map_err(|error| {
        RepoError::Message(format!(
            "Resolution {} actor {actor:?} has invalid authority assignment: {error}",
            resolution.id
        ))
    })?;
    if assignment.actor_kind != expected_kind
        || !assignment.holds(ActorRole::Resolver)
        || !is_canonical_utc(&assignment.effective_time)
        || assignment.effective_time > resolution.effective_at
    {
        return Err(RepoError::Message(format!(
            "Resolution {} actor {actor:?} lacks an effective Resolver assignment",
            resolution.id
        )));
    }

    Ok(BoundResolution { resolution, actor })
}

fn resolution_actor(reference: &str) -> Result<(&str, ActorKind), RepoError> {
    let (actor, kind) = if let Some(actor) = reference.strip_prefix("person://") {
        (actor, ActorKind::Human)
    } else if let Some(actor) = reference.strip_prefix("policy-service://") {
        (actor, ActorKind::PolicyService)
    } else {
        return Err(RepoError::Message(format!(
            "Resolution resolver reference {reference:?} must use person:// or policy-service://"
        )));
    };
    if actor.trim().is_empty() || actor.contains('/') || actor.contains(char::is_whitespace) {
        return Err(RepoError::Message(format!(
            "Resolution resolver reference {reference:?} has no canonical actor identity"
        )));
    }
    Ok((actor, kind))
}

fn validate_resolution_evaluation(
    resolution: &Resolution,
    evaluation: &crate::resolve::RecordedResolutionEvaluation,
) -> Result<(), RepoError> {
    if resolution.assurance_case_snapshot_digest != evaluation.assurance_case_snapshot_digest {
        return Err(RepoError::Message(format!(
            "Resolution {} assurance snapshot digest mismatch: recorded {}, recomputed {}",
            resolution.id,
            resolution.assurance_case_snapshot_digest,
            evaluation.assurance_case_snapshot_digest
        )));
    }
    if resolution.artifact_manifest_digest != evaluation.artifact_manifest_digest {
        return Err(RepoError::Message(format!(
            "Resolution {} artifact manifest digest mismatch: recorded {}, recomputed {}",
            resolution.id, resolution.artifact_manifest_digest, evaluation.artifact_manifest_digest
        )));
    }
    for (label, recorded, recomputed) in [
        (
            "Gate Run",
            &resolution.gate_run_refs,
            &evaluation.gate_run_refs,
        ),
        (
            "judgment",
            &resolution.judgment_refs,
            &evaluation.judgment_refs,
        ),
        (
            "residual-risk",
            &resolution.residual_risk_refs,
            &evaluation.residual_risk_refs,
        ),
    ] {
        if recorded != recomputed {
            return Err(RepoError::Message(format!(
                "Resolution {} {label} references do not match exact evaluated evidence: recorded {recorded:?}, recomputed {recomputed:?}",
                resolution.id
            )));
        }
    }
    if evaluation.outcome != Some(true) {
        let detail = match evaluation.outcome {
            Some(false) => "admissible dispositions do not permit satisfied",
            None => "at least one required obligation has no admissible disposition",
            Some(true) => unreachable!(),
        };
        return Err(RepoError::Message(format!(
            "Resolution {} claims accepted work, but recomputed §38.6 outcome refuses it: {detail}",
            resolution.id
        )));
    }
    resolution.require_reliable().map_err(|error| {
        RepoError::Message(format!("Resolution cannot be relied upon: {error}"))
    })?;
    resolution
        .validate(evaluation.checks, false)
        .map_err(|error| RepoError::Message(format!("Resolution evidence is invalid: {error}")))
}

fn canonical_utc_unix_seconds(value: &str) -> Option<i64> {
    if !is_canonical_utc(value) {
        return None;
    }
    let number = |start, end| value.get(start..end)?.parse::<i64>().ok();
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;

    // Howard Hinnant's civil-date transform. `is_canonical_utc` already
    // validated the Gregorian date and second fields; this maps it to the Unix
    // epoch without locale, timezone, or platform-specific date APIs.
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;
    Some(days_since_epoch * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn reject_internal_support(
    receipt: &LegacyAdrDispositionReceipt,
    binding: &ContentBinding,
    internal_artifacts: &InternalArtifacts,
) -> Result<(), RepoError> {
    if internal_artifacts.contains(binding) {
        return Err(RepoError::Message(format!(
            "ADR {} support reuses migration, capsule, response, or manifest material",
            receipt.payload.source.adr_id
        )));
    }
    Ok(())
}

fn reject_source_derived_support(
    receipt: &LegacyAdrDispositionReceipt,
    binding: &ContentBinding,
    identity: Option<(&str, &str)>,
) -> Result<(), RepoError> {
    let same_object = identity.is_some_and(|(commit, path)| {
        commit == receipt.payload.source.commit_sha && path == receipt.payload.source.path
    });
    if same_object || binding.sha256 == receipt.payload.source.source_sha256 {
        return Err(RepoError::Message(format!(
            "ADR {} support is its own source or copied source bytes",
            receipt.payload.source.adr_id
        )));
    }
    Ok(())
}

fn is_adr_record_path(path: &str) -> bool {
    let filename = path.rsplit('/').next().unwrap_or_default();
    filename.len() > 8
        && filename.as_bytes()[..4].iter().all(u8::is_ascii_digit)
        && filename.as_bytes()[4] == b'-'
        && filename.ends_with(".md")
}

fn parse_support_record<T: DeserializeOwned>(
    reference: &str,
    bytes: &[u8],
    label: &str,
) -> Result<T, RepoError> {
    if reference.ends_with(".json") {
        openwarrant_core::legacy_disposition::parse_strict_json(bytes).map_err(|error| {
            RepoError::Message(format!("{label} {reference:?} is not valid JSON: {error}"))
        })
    } else if reference.ends_with(".toml") {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| RepoError::Message(format!("{label} {reference:?} is not UTF-8 TOML")))?;
        toml::from_str(text).map_err(|error| {
            RepoError::Message(format!("{label} {reference:?} is not valid TOML: {error}"))
        })
    } else {
        Err(RepoError::Message(format!(
            "{label} {reference:?} must end in .json or .toml"
        )))
    }
}

fn reject_unknown_record_fields(
    reference: &str,
    bytes: &[u8],
    label: &str,
    allowed: &[&str],
) -> Result<(), RepoError> {
    let keys: Vec<String> = if reference.ends_with(".json") {
        let value: serde_json::Value =
            openwarrant_core::legacy_disposition::parse_strict_json(bytes).map_err(|error| {
                RepoError::Message(format!("{label} {reference:?} is not valid JSON: {error}"))
            })?;
        value
            .as_object()
            .ok_or_else(|| {
                RepoError::Message(format!("{label} {reference:?} must be a JSON object"))
            })?
            .keys()
            .cloned()
            .collect()
    } else if reference.ends_with(".toml") {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| RepoError::Message(format!("{label} {reference:?} is not UTF-8 TOML")))?;
        let value: toml::Value = toml::from_str(text).map_err(|error| {
            RepoError::Message(format!("{label} {reference:?} is not valid TOML: {error}"))
        })?;
        value
            .as_table()
            .ok_or_else(|| {
                RepoError::Message(format!("{label} {reference:?} must be a TOML table"))
            })?
            .keys()
            .cloned()
            .collect()
    } else {
        return Err(RepoError::Message(format!(
            "{label} {reference:?} must end in .json or .toml"
        )));
    };
    if let Some(unknown) = keys.iter().find(|key| !allowed.contains(&key.as_str())) {
        return Err(RepoError::Message(format!(
            "{label} {reference:?} contains unknown field {unknown:?}"
        )));
    }
    Ok(())
}

fn support_origin_and_path(reference: &str) -> Result<(String, &str), RepoError> {
    if let Some(path) = reference.strip_prefix("ow:") {
        return Ok(("ow".to_owned(), path));
    }
    if let Some(rest) = reference.strip_prefix("git:") {
        let (commit, path) = rest.split_once(':').ok_or_else(|| {
            RepoError::Message(format!(
                "Gate bundle reference {reference:?} is not git:<commit>:<path>"
            ))
        })?;
        return Ok((format!("git:{commit}"), path));
    }
    Err(RepoError::Message(format!(
        "Gate bundle reference {reference:?} must use ow: or git:"
    )))
}

fn parse_gate_definition_support(
    adr_id: &str,
    artifact: &CachedSupport,
) -> Result<GateDefinition, RepoError> {
    if !artifact.reference.ends_with(".yaml") && !artifact.reference.ends_with(".yml") {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate Definition {:?} must be YAML",
            artifact.reference
        )));
    }
    let text = std::str::from_utf8(&artifact.bytes).map_err(|_| {
        RepoError::Message(format!(
            "ADR {adr_id} Gate Definition {:?} is not UTF-8",
            artifact.reference
        ))
    })?;
    let document = openwarrant_core::structured::parse(text).map_err(|error| {
        RepoError::Message(format!(
            "ADR {adr_id} Gate Definition {:?} is malformed: {error}",
            artifact.reference
        ))
    })?;
    let mut definition =
        openwarrant_core::gate::definition_from_structured(&document).map_err(|error| {
            RepoError::Message(format!(
                "ADR {adr_id} Gate Definition {:?} is invalid: {error}",
                artifact.reference
            ))
        })?;
    definition
        .bind_local_source_digest(&artifact.digest)
        .map_err(|error| {
            RepoError::Message(format!(
                "ADR {adr_id} Gate Definition {:?} has invalid source identity: {error}",
                artifact.reference
            ))
        })?;
    Ok(definition)
}

#[allow(clippy::too_many_arguments)]
fn verify_required_gate_pass(
    adr_id: &str,
    subject_source_sha256: &str,
    reviewed_at: &str,
    run_artifact: &CachedSupport,
    definition_artifact: &CachedSupport,
    binding_artifact: &CachedSupport,
    selection_artifact: &CachedSupport,
    stdout_artifact: &CachedSupport,
    stderr_artifact: &CachedSupport,
    fixture_artifacts: &[CachedSupport],
    receipt_artifact: &CachedSupport,
    context: ResolutionVerificationContext<'_>,
) -> Result<crate::gate_evidence::AdmissibleGateRun, RepoError> {
    let run: GateRun =
        parse_support_record(&run_artifact.reference, &run_artifact.bytes, "Gate Run")?;
    let receipt: GateReceipt = parse_support_record(
        &receipt_artifact.reference,
        &receipt_artifact.bytes,
        "Gate receipt",
    )?;
    let definition = parse_gate_definition_support(adr_id, definition_artifact)?;
    let binding: GateBinding = parse_support_record(
        &binding_artifact.reference,
        &binding_artifact.bytes,
        "Gate Binding",
    )?;
    let selection: openwarrant_core::TestSelectionObservation = parse_support_record(
        &selection_artifact.reference,
        &selection_artifact.bytes,
        "test-selection observation",
    )?;

    let (origin, receipt_path) = support_origin_and_path(&receipt_artifact.reference)?;
    for (label, artifact) in [
        ("Gate Run", run_artifact),
        ("Gate Binding", binding_artifact),
        ("test-selection observation", selection_artifact),
        ("Gate stdout", stdout_artifact),
        ("Gate stderr", stderr_artifact),
    ] {
        let (artifact_origin, _) = support_origin_and_path(&artifact.reference)?;
        if artifact_origin != origin {
            return Err(RepoError::Message(format!(
                "ADR {adr_id} {label} is outside the Gate receipt's exact repository snapshot"
            )));
        }
    }
    if !crate::gate_cmd::canonical_run_id(&run.id) || receipt.run_id != run.id {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt does not bind one canonical run identity"
        )));
    }
    let receipt_name = format!("{}.receipt.json", run.id);
    let receipt_path = Utf8Path::new(receipt_path);
    if receipt_path.file_name() != Some(receipt_name.as_str()) {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt basename is not derived exactly from run {}",
            run.id
        )));
    }
    let directory = receipt_path.parent().unwrap_or(Utf8Path::new(""));
    let expected = |suffix: &str| {
        let name = format!("{}.{suffix}", run.id);
        if directory.as_str().is_empty() {
            name
        } else {
            format!("{directory}/{name}")
        }
    };
    if receipt_path.as_str() != expected("receipt.json") {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt path is not derived exactly from run {}",
            run.id
        )));
    }
    let (_, run_path) = support_origin_and_path(&run_artifact.reference)?;
    let (_, binding_path) = support_origin_and_path(&binding_artifact.reference)?;
    let (_, selection_path) = support_origin_and_path(&selection_artifact.reference)?;
    let (_, stdout_path) = support_origin_and_path(&stdout_artifact.reference)?;
    let (_, stderr_path) = support_origin_and_path(&stderr_artifact.reference)?;
    if run_path != expected("run.toml")
        || binding_path != expected("binding.json")
        || selection_path != expected("selection.json")
        || stdout_path != expected("stdout.txt")
        || stderr_path != expected("stderr.txt")
        || receipt.selection_observation_ref != selection_path
        || receipt.stdout_ref != stdout_path
        || receipt.stderr_ref != stderr_path
    {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt does not bind its exact sibling Run, Binding, selection, stdout, and stderr objects"
        )));
    }

    if !is_canonical_utc(reviewed_at)
        || !is_canonical_utc(context.verification_as_of)
        || receipt.completed_at.as_str() > reviewed_at
        || reviewed_at > context.verification_as_of
    {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt completion, human review, and verification cutoff are not monotonic"
        )));
    }

    let mut observed_fixtures = BTreeMap::<String, String>::new();
    for fixture in fixture_artifacts {
        let (_, path) = support_origin_and_path(&fixture.reference)?;
        if observed_fixtures
            .insert(path.to_owned(), fixture.digest.clone())
            .is_some()
        {
            return Err(RepoError::Message(format!(
                "ADR {adr_id} Gate fixture inventory repeats {path:?}"
            )));
        }
    }
    let declared_fixtures: BTreeMap<String, String> = binding
        .fixtures
        .iter()
        .map(|fixture| (fixture.reference.clone(), fixture.digest.clone()))
        .collect();
    if declared_fixtures.len() != binding.fixtures.len() || observed_fixtures != declared_fixtures {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate fixture content bindings do not match the exact Gate Binding inventory"
        )));
    }
    if !receipt.raw_evidence_refs.is_empty() {
        return Err(RepoError::Message(format!(
            "ADR {adr_id} Gate receipt names raw evidence without content bindings"
        )));
    }

    let expected_subjects = BTreeSet::from([subject_source_sha256.to_owned()]);
    let expected_raw_evidence = BTreeSet::new();
    crate::gate_evidence::admit_components(
        run,
        &definition,
        &binding,
        &receipt,
        crate::gate_evidence::ObservedStreams {
            gate_run_digest: &run_artifact.digest,
            selection_ref: selection_path,
            selection_digest: &selection_artifact.digest,
            selection: &selection,
            stdout_ref: stdout_path,
            stdout_digest: &stdout_artifact.digest,
            stderr_ref: stderr_path,
            stderr_digest: &stderr_artifact.digest,
        },
        crate::gate_evidence::SubjectBasis::Exact(&expected_subjects),
        &expected_raw_evidence,
        context.authority,
        &context.repo.performer(),
    )
    .map_err(|error| {
        RepoError::Message(format!(
            "ADR {adr_id} Gate receipt bundle is inadmissible: {error}"
        ))
    })
}

#[allow(clippy::too_many_arguments)]
fn verify_falsification_record(
    receipt: &LegacyAdrDispositionReceipt,
    subject_source_sha256: &str,
    record_artifact: &CachedSupport,
    warrant_repo: &Utf8Path,
    source_repo: &Utf8Path,
    checked_commits: &mut BTreeSet<String>,
    snapshot: &RepositorySnapshot,
    state: &mut SupportState,
    internal_artifacts: &InternalArtifacts,
) -> Result<(), RepoError> {
    let record: openwarrant_core::legacy_disposition::LegacyAdrFalsificationRecord =
        openwarrant_core::legacy_disposition::parse_legacy_json(&record_artifact.bytes).map_err(
            |error| {
                RepoError::Message(format!(
                    "falsification record {:?} is not valid JSON: {error}",
                    record_artifact.reference
                ))
            },
        )?;
    if record.schema != openwarrant_core::legacy_disposition::LEGACY_ADR_FALSIFICATION_SCHEMA
        || record.subject_source_sha256 != subject_source_sha256
        || record.statement.trim().is_empty()
        || record.observations.is_empty()
        || record
            .observations
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err(RepoError::Message(format!(
            "ADR {} falsification record has wrong schema, source, statement, or observation inventory",
            receipt.payload.source.adr_id
        )));
    }
    for observation in &record.observations {
        reject_internal_support(receipt, observation, internal_artifacts)?;
        let cached = verify_support_binding(
            observation,
            warrant_repo,
            source_repo,
            checked_commits,
            snapshot,
            state,
        )?;
        let identity = cached
            .identity
            .as_ref()
            .map(|(commit, path)| (commit.as_str(), path.as_str()));
        reject_source_derived_support(receipt, observation, identity)?;
    }
    Ok(())
}

fn verify_successor_source(
    source_repo: &Utf8Path,
    receipt: &LegacyAdrDispositionReceipt,
    successor: &SourceBinding,
    checked_commits: &mut BTreeSet<String>,
    checked_refs: &mut CheckedRefs,
) -> Result<(), RepoError> {
    if successor.repository_identity != SOURCE_REPOSITORY_IDENTITY
        || successor.git_object_format != GitObjectFormat::Sha1
        || successor.adr_id == receipt.payload.source.adr_id
    {
        return Err(RepoError::Message(format!(
            "ADR {} successor has wrong repository, object format, or self identity",
            receipt.payload.source.adr_id
        )));
    }
    let check_ref = git(
        source_repo,
        "validate successor ref",
        &["check-ref-format", &successor.exact_ref],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !check_ref.status.success() {
        return Err(git_failure("validate successor ref", &check_ref));
    }
    verify_ref_contains_tip(
        source_repo,
        &successor.exact_ref,
        &successor.ref_tip_at_review,
        checked_refs,
    )?;
    require_literal_commit(source_repo, &successor.commit_sha, checked_commits)?;
    let ancestry = git(
        source_repo,
        "verify successor commit ancestry",
        &[
            "merge-base",
            "--is-ancestor",
            &successor.commit_sha,
            &successor.ref_tip_at_review,
        ],
        MAX_GIT_CONTROL_BYTES,
    )?;
    if !ancestry.status.success() {
        return Err(git_failure("verify successor commit ancestry", &ancestry));
    }
    let bytes = read_git_blob(
        source_repo,
        &successor.commit_sha,
        &successor.path,
        checked_commits,
    )?;
    if content_digest(&bytes) != successor.source_sha256 {
        return Err(RepoError::Message(format!(
            "ADR {} successor source digest mismatch",
            receipt.payload.source.adr_id
        )));
    }
    Ok(())
}

fn verify_support_binding(
    binding: &ContentBinding,
    warrant_repo: &Utf8Path,
    source_repo: &Utf8Path,
    checked_commits: &mut BTreeSet<String>,
    snapshot: &RepositorySnapshot,
    state: &mut SupportState,
) -> Result<CachedSupport, RepoError> {
    state.bindings = state
        .bindings
        .checked_add(1)
        .ok_or_else(|| RepoError::Message("support binding count overflow".to_owned()))?;
    if state.bindings > MAX_SUPPORT_BINDINGS {
        return Err(RepoError::Message(format!(
            "support binding count {} exceeds limit {MAX_SUPPORT_BINDINGS}",
            state.bindings
        )));
    }
    if let Some(cached) = state.artifacts.get(&binding.reference) {
        if cached.digest != binding.sha256 {
            return Err(RepoError::Message(format!(
                "support reference {:?} is reused with a different digest",
                binding.reference
            )));
        }
        return Ok(cached.clone());
    }
    if state.artifacts.len() >= MAX_UNIQUE_SUPPORT_ARTIFACTS {
        return Err(RepoError::Message(format!(
            "unique support artifact count exceeds limit {MAX_UNIQUE_SUPPORT_ARTIFACTS}"
        )));
    }

    let (bytes, identity) = if let Some(rest) = binding.reference.strip_prefix("git:") {
        let (commit, path) = rest.split_once(':').ok_or_else(|| {
            RepoError::Message(format!(
                "content reference {:?} is not git:<commit>:<path>",
                binding.reference
            ))
        })?;
        if commit.len() != 40
            || !commit
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(RepoError::Message(format!(
                "content reference {:?} does not use a literal lowercase SHA-1 commit",
                binding.reference
            )));
        }
        validate_git_path(path)?;
        (
            read_git_blob(source_repo, commit, path, checked_commits)?,
            Some((commit.to_owned(), path.to_owned())),
        )
    } else if let Some(path) = binding.reference.strip_prefix("ow:") {
        let (_, bytes) = read_repository_bounded(
            warrant_repo,
            path,
            "OpenWarrant support artifact",
            MAX_SUPPORT_BYTES,
        )?;
        snapshot.verify_bytes(path, &bytes, MAX_SUPPORT_BYTES)?;
        (bytes, None)
    } else {
        return Err(RepoError::Message(format!(
            "content reference {:?} must use git: or ow:",
            binding.reference
        )));
    };
    state.total_bytes = state
        .total_bytes
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| RepoError::Message("support artifact byte budget overflow".to_owned()))?;
    if state.total_bytes > MAX_TOTAL_SUPPORT_BYTES {
        return Err(RepoError::Message(format!(
            "support artifacts total {} bytes; aggregate limit is {MAX_TOTAL_SUPPORT_BYTES}",
            state.total_bytes
        )));
    }
    verify_raw_binding(binding, &bytes)?;
    let cached = CachedSupport {
        reference: binding.reference.clone(),
        digest: content_digest(&bytes),
        identity,
        bytes: Arc::from(bytes),
    };
    state
        .artifacts
        .insert(binding.reference.clone(), cached.clone());
    Ok(cached)
}

fn validate_git_path(path: &str) -> Result<(), RepoError> {
    if path.starts_with('/')
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
    {
        return Err(RepoError::Message(format!(
            "Git content path is not canonical: {path:?}"
        )));
    }
    Ok(())
}

fn normalize_git_path(path: &str) -> Result<String, RepoError> {
    if path.starts_with('/') || path.contains('\\') || path.contains(':') {
        return Err(RepoError::Message(format!(
            "Git content path is not canonicalizable: {path:?}"
        )));
    }
    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {
                return Err(RepoError::Message(format!(
                    "Git content path is not canonicalizable: {path:?}"
                )));
            }
            ".." => {
                if components.pop().is_none() {
                    return Err(RepoError::Message(format!(
                        "Git content path escapes repository: {path:?}"
                    )));
                }
            }
            value => components.push(value),
        }
    }
    let normalized = components.join("/");
    validate_git_path(&normalized)?;
    Ok(normalized)
}

fn verify_raw_binding(binding: &ContentBinding, bytes: &[u8]) -> Result<(), RepoError> {
    let actual = content_digest(bytes);
    if actual != binding.sha256 {
        return Err(RepoError::Message(format!(
            "content digest mismatch for {:?}: recorded {}, recomputed {}",
            binding.reference, binding.sha256, actual
        )));
    }
    Ok(())
}

fn safe_repository_path(root: &Utf8Path, reference: &str) -> Result<Utf8PathBuf, RepoError> {
    let path = Utf8Path::new(reference);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component.as_str(), "" | "." | ".."))
    {
        return Err(RepoError::Message(format!(
            "repository artifact reference escapes root: {reference:?}"
        )));
    }
    let root = canonical_utf8(root, "repository root")?;
    let candidate = canonical_utf8(&root.join(path), "repository artifact")?;
    if !candidate.starts_with(&root) {
        return Err(RepoError::Message(format!(
            "repository artifact reference escapes root through a symlink: {reference:?}"
        )));
    }
    Ok(candidate)
}

fn repository_input_path(root: &Utf8Path, path: &Utf8Path) -> Utf8PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        root.join(path)
    }
}

fn repository_relative(root: &Utf8Path, path: &Utf8Path) -> Result<String, RepoError> {
    let root = canonical_utf8(root, "repository root")?;
    let path = canonical_utf8(path, "repository artifact")?;
    path.strip_prefix(&root)
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_str())
                .collect::<Vec<_>>()
                .join("/")
        })
        .map_err(|_| {
            RepoError::Message(format!(
                "artifact {path} is outside Warrant repository {root}"
            ))
        })
}

fn canonical_utf8(path: &Utf8Path, label: &str) -> Result<Utf8PathBuf, RepoError> {
    let canonical = std::fs::canonicalize(path).map_err(|source| RepoError::Io {
        context: format!("could not resolve {label} {path}"),
        source,
    })?;
    Utf8PathBuf::from_path_buf(canonical).map_err(|_| RepoError::NonUtf8Path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    use openwarrant_core::ResolutionStanding;
    use openwarrant_core::authority::RoleAssignment;
    use openwarrant_core::contract::{
        Authorization, ContractRevision, Independence as AuthIndependence,
    };
    use openwarrant_core::deliverable::{ArtifactProvenance, DeliverableKind};
    use openwarrant_core::independence::Independence as VerificationIndependence;
    use openwarrant_core::legacy_disposition::{
        HumanReview, LEGACY_ADR_DISPOSITION_MANIFEST_KIND, LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA,
        LEGACY_ADR_DISPOSITION_RECEIPT_KIND, LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA,
        LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND, LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA,
        LegacyAdrDispositionJudgment, LegacyAdrDispositionManifestPayload,
        LegacyAdrDispositionReceiptPayload,
    };
    use openwarrant_core::migration::MigratedAdr;
    use openwarrant_core::obligation::Disposition;
    use openwarrant_core::resolution::{CommonOutcome, Resolution, ResolutionChecks};
    use openwarrant_core::verification::{ActorKind as VerifierKind, Verifier};
    use openwarrant_core::{JudgmentAuthority, Namespace};

    fn scratch(label: &str) -> Utf8PathBuf {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "openwarrant-{label}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create scratch directory");
        Utf8PathBuf::from_path_buf(path).expect("UTF-8 scratch path")
    }

    fn fake_snapshot(root: &Utf8Path) -> RepositorySnapshot {
        RepositorySnapshot {
            root: root.to_owned(),
            revision: "0".repeat(40),
        }
    }

    fn commit_snapshot(root: &Utf8Path) -> RepositorySnapshot {
        commit_snapshot_at(root, None)
    }

    fn commit_snapshot_at(root: &Utf8Path, committed_at: Option<&str>) -> RepositorySnapshot {
        let run = |args: &[&str]| {
            let mut command = Command::new("git");
            command.arg("-C").arg(root).args(args);
            if args.first() == Some(&"commit")
                && let Some(committed_at) = committed_at
            {
                command
                    .env("GIT_AUTHOR_DATE", committed_at)
                    .env("GIT_COMMITTER_DATE", committed_at);
            }
            let output = command.output().expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output
        };
        run(&["init", "-q"]);
        run(&["config", "user.name", "OpenWarrant Test"]);
        run(&["config", "user.email", "test@example.invalid"]);
        run(&["add", "."]);
        run(&["commit", "-q", "-m", "fixture"]);
        let revision = String::from_utf8(run(&["rev-parse", "HEAD"]).stdout)
            .expect("UTF-8 revision")
            .trim()
            .to_owned();
        RepositorySnapshot {
            root: root.to_owned(),
            revision,
        }
    }

    #[test]
    fn ref_cache_binds_each_historical_tip() {
        let root = scratch("ref-tip-cache");
        fs::write(root.join("fixture.txt"), "fixture\n").expect("write fixture");
        let snapshot = commit_snapshot(&root);
        let run = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .expect("run git");
            assert!(
                output.status.success(),
                "git {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            output
        };
        run(&["branch", "integration/consolidate", &snapshot.revision]);
        let tree = String::from_utf8(run(&["rev-parse", "HEAD^{tree}"]).stdout)
            .expect("UTF-8 tree")
            .trim()
            .to_owned();
        let unrelated =
            String::from_utf8(run(&["commit-tree", &tree, "-m", "unrelated root commit"]).stdout)
                .expect("UTF-8 commit")
                .trim()
                .to_owned();

        let mut checked = CheckedRefs::new();
        verify_ref_contains_tip(
            &root,
            "refs/heads/integration/consolidate",
            &snapshot.revision,
            &mut checked,
        )
        .expect("reachable tip");
        let error = verify_ref_contains_tip(
            &root,
            "refs/heads/integration/consolidate",
            &unrelated,
            &mut checked,
        )
        .expect_err("same ref must not cache a different unreachable tip");
        assert!(error.to_string().contains("no longer reachable"));
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_inventory_does_not_decode_or_admit_sibling_prefix_paths() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let root = scratch("literal-inventory-prefix");
        fs::create_dir_all(root.join("evidence")).expect("evidence directory");
        fs::write(root.join("evidence/record.toml"), "id = \"one\"\n")
            .expect("write exact evidence");
        let sibling = root.join("evidence-evil");
        fs::create_dir_all(&sibling).expect("sibling directory");
        let invalid_name = OsString::from_vec(vec![b'b', b'a', b'd', 0xff]);
        fs::write(sibling.as_std_path().join(invalid_name), b"poison")
            .expect("write non-UTF-8 sibling path");
        let snapshot = commit_snapshot(&root);

        assert_eq!(
            snapshot.list_files("evidence").expect("exact subtree"),
            vec!["evidence/record.toml"]
        );
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[cfg(unix)]
    #[test]
    fn snapshot_blob_reader_rejects_symlink_mode() {
        use std::os::unix::fs::symlink;

        let root = scratch("snapshot-symlink");
        fs::write(root.join("target.txt"), b"target").expect("write target");
        symlink("target.txt", root.join("link.txt")).expect("create symlink");
        let snapshot = commit_snapshot(&root);
        let error = snapshot
            .read_blob("link.txt", 1024)
            .expect_err("Git symlink blob must not be evidence bytes");
        assert!(error.to_string().contains("not a regular Git blob"));
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[test]
    fn evidence_history_uses_an_exact_literal_path() {
        let root = scratch("literal-history");
        fs::write(root.join("evidence"), b"exact").expect("write exact path");
        let first = commit_snapshot_at(&root, Some("2026-08-25T00:00:00Z"));
        fs::write(root.join("evidence-later"), b"sibling").expect("write sibling path");
        let snapshot = commit_snapshot_at(&root, Some("2026-08-29T00:00:00Z"));

        let identity = snapshot
            .path_commit_identity("evidence")
            .expect("exact path history");
        assert_eq!(identity.commit, first.revision);
        assert_eq!(
            identity.committed_unix_seconds,
            canonical_utc_unix_seconds("2026-08-25T00:00:00Z").expect("fixture timestamp")
        );
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    fn receipt() -> LegacyAdrDispositionReceipt {
        LegacyAdrDispositionReceipt {
            schema: LEGACY_ADR_DISPOSITION_RECEIPT_SCHEMA.to_owned(),
            kind: LEGACY_ADR_DISPOSITION_RECEIPT_KIND.to_owned(),
            payload: LegacyAdrDispositionReceiptPayload {
                warrant: WarrantBinding {
                    warrant_id: "OW-WAR-0043".to_owned(),
                    authorized_revision: 1,
                    contract_digest: "a".repeat(64),
                    authorization_ref: "docs/warrants/OW-WAR-0043/authorization.toml".to_owned(),
                    authorization_sha256: format!("sha256:{}", "b".repeat(64)),
                    extensions: Default::default(),
                },
                source: SourceBinding {
                    adr_id: "0001".to_owned(),
                    repository_identity: "github.com/Quitetall/LamQuant".to_owned(),
                    git_object_format: GitObjectFormat::Sha1,
                    exact_ref: "refs/heads/integration/consolidate".to_owned(),
                    ref_tip_at_review: "1".repeat(40),
                    commit_sha: "1".repeat(40),
                    path: "docs/decisions/0001-x.md".to_owned(),
                    source_sha256: format!("sha256:{}", "a".repeat(64)),
                    extensions: Default::default(),
                },
                migration: ImportBinding::Migrated {
                    artifact: ContentBinding {
                        reference: "artifacts/import.json".to_owned(),
                        sha256: format!("sha256:{}", "b".repeat(64)),
                        extensions: Default::default(),
                    },
                    imported_body_sha256: format!("sha256:{}", "c".repeat(64)),
                    extensions: Default::default(),
                },
                review: HumanReview {
                    actor: "QuiteTall".to_owned(),
                    role_assignment_ref: "docs/authority/roles.toml".to_owned(),
                    role_assignment_sha256: format!("sha256:{}", "d".repeat(64)),
                    reviewed_at: "2026-08-26T12:00:00Z".to_owned(),
                    response: ContentBinding {
                        reference: format!("{REVIEW_RESPONSE_PREFIX}batch-001.json"),
                        sha256: format!("sha256:{}", "9".repeat(64)),
                        extensions: Default::default(),
                    },
                    judgment_id: "judgment-0001".to_owned(),
                    extensions: Default::default(),
                },
                disposition: LegacyAdrDisposition::Complete,
                reason: "individual reason".to_owned(),
                evidence: vec![LegacyAdrSupport::ImplementationArtifact {
                    artifact: ContentBinding {
                        reference:
                            "git:2222222222222222222222222222222222222222:src/implementation.rs"
                                .to_owned(),
                        sha256: format!("sha256:{}", "e".repeat(64)),
                        extensions: Default::default(),
                    },
                    extensions: Default::default(),
                }],
                successors: vec![],
                proposal_digest: None,
                extensions: Default::default(),
            },
            receipt_digest: String::new(),
            extensions: Default::default(),
        }
    }

    fn manifest(receipts: Vec<LegacyAdrDispositionReceipt>) -> LegacyAdrDispositionManifest {
        LegacyAdrDispositionManifest {
            schema: LEGACY_ADR_DISPOSITION_MANIFEST_SCHEMA.to_owned(),
            kind: LEGACY_ADR_DISPOSITION_MANIFEST_KIND.to_owned(),
            payload: LegacyAdrDispositionManifestPayload {
                final_adr_id: FINAL_ADR_ID.to_owned(),
                warrant: receipts[0].payload.warrant.clone(),
                sha_f: SHA_F.to_owned(),
                source_import: ContentBinding {
                    reference: "artifacts/import.json".to_owned(),
                    sha256: format!("sha256:{}", "f".repeat(64)),
                    extensions: Default::default(),
                },
                sha_f_predecessor_ids: vec!["0001".to_owned()],
                off_branch_ids: vec![],
                receipts,
                verification_as_of: "2026-08-30T00:00:00Z".to_owned(),
                extensions: Default::default(),
            },
            manifest_digest: String::new(),
            extensions: Default::default(),
        }
    }

    fn review_response(
        receipt: &LegacyAdrDispositionReceipt,
    ) -> LegacyAdrDispositionReviewResponse {
        LegacyAdrDispositionReviewResponse {
            schema: LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_SCHEMA.to_owned(),
            kind: LEGACY_ADR_DISPOSITION_REVIEW_RESPONSE_KIND.to_owned(),
            warrant: receipt.payload.warrant.clone(),
            reviewed_at: receipt.payload.review.reviewed_at.clone(),
            judgments: vec![LegacyAdrDispositionJudgment {
                id: receipt.payload.review.judgment_id.clone(),
                adr_id: receipt.payload.source.adr_id.clone(),
                review_subject_digest: artifact_digest(&receipt.payload.review_subject_preimage())
                    .expect("review subject digest"),
                statement: "Reviewed exact legacy ADR disposition subject".to_owned(),
                actor: receipt.payload.review.actor.clone(),
                acting_role: ActorRole::Judge.to_string(),
                meaning: "Terminal disposition recorded after individual review".to_owned(),
                authority: JudgmentAuthority::Authorized,
                limitations: vec![],
                extensions: Default::default(),
            }],
            extensions: Default::default(),
        }
    }

    fn human_judge_register() -> AuthorityRegister {
        AuthorityRegister::new(vec![RoleAssignment {
            actor: "QuiteTall".to_owned(),
            actor_kind: ActorKind::Human,
            roles: BTreeSet::from([ActorRole::Judge]),
            assigned_by: "human:owner".to_owned(),
            effective_time: "2026-08-25T00:00:00Z".to_owned(),
            note: None,
        }])
    }

    fn accepted_resolution() -> Resolution {
        Resolution {
            id: "019c0000-0000-7000-8000-000000000001".to_owned(),
            common_outcome: CommonOutcome::Satisfied,
            profile_outcome: "delivered".to_owned(),
            contract_revision: 1,
            contract_digest: "a".repeat(64),
            assurance_case_snapshot_digest: format!("sha256:{}", "b".repeat(64)),
            artifact_manifest_digest: format!("sha256:{}", "c".repeat(64)),
            gate_run_refs: vec!["gate-run://one".to_owned()],
            judgment_refs: vec![],
            residual_risk_refs: vec![],
            resolved_by_ref: "person://Resolver".to_owned(),
            acting_role_ref: "role-assignment://resolver".to_owned(),
            meaning: "Accept bounded delivery evidence.".to_owned(),
            effective_at: "2026-08-27T00:00:00Z".to_owned(),
            recorded_at: "2026-08-27T00:00:01Z".to_owned(),
            standing: ResolutionStanding::Valid,
        }
    }

    fn human_resolver_register(effective_time: &str) -> AuthorityRegister {
        AuthorityRegister::new(vec![RoleAssignment {
            actor: "Resolver".to_owned(),
            actor_kind: ActorKind::Human,
            roles: BTreeSet::from([ActorRole::Resolver]),
            assigned_by: "human:owner".to_owned(),
            effective_time: effective_time.to_owned(),
            note: None,
        }])
    }

    fn bound_resolution(
        resolution: &Resolution,
    ) -> (ContentBinding, String, Vec<u8>, WarrantBinding) {
        let bytes = serde_json::to_vec(resolution).expect("resolution JSON");
        let semantic = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::Resolution, resolution).expect("resolution digest")
        );
        let warrant = WarrantBinding {
            warrant_id: "OW-WAR-0099".to_owned(),
            authorized_revision: resolution.contract_revision,
            contract_digest: resolution.contract_digest.clone(),
            authorization_ref: "docs/warrants/OW-WAR-0099/authorization.toml".to_owned(),
            authorization_sha256: format!("sha256:{}", "d".repeat(64)),
            extensions: Default::default(),
        };
        (
            ContentBinding {
                reference: "docs/warrants/OW-WAR-0099/resolution.json".to_owned(),
                sha256: content_digest(&bytes),
                extensions: Default::default(),
            },
            semantic,
            bytes,
            warrant,
        )
    }

    fn synthetic_evaluation(
        resolution: &Resolution,
        checks: ResolutionChecks,
        outcome: Option<bool>,
    ) -> crate::resolve::RecordedResolutionEvaluation {
        crate::resolve::RecordedResolutionEvaluation {
            checks,
            outcome,
            assurance_case_snapshot_digest: resolution.assurance_case_snapshot_digest.clone(),
            artifact_manifest_digest: resolution.artifact_manifest_digest.clone(),
            gate_run_refs: resolution.gate_run_refs.clone(),
            judgment_refs: resolution.judgment_refs.clone(),
            residual_risk_refs: resolution.residual_risk_refs.clone(),
        }
    }

    fn parse_mutated_resolution(
        resolution: &Resolution,
        warrant: &WarrantBinding,
        authority: &AuthorityRegister,
        authorization_effective_at: &str,
        verification_as_of: &str,
    ) -> RepoError {
        let bytes = serde_json::to_vec(resolution).expect("Resolution JSON");
        let binding = ContentBinding {
            reference: "docs/warrants/OW-WAR-0099/resolution.json".to_owned(),
            sha256: content_digest(&bytes),
            extensions: Default::default(),
        };
        let semantic = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::Resolution, resolution).expect("Resolution digest")
        );
        parse_bound_resolution(
            &binding,
            &semantic,
            warrant,
            &bytes,
            authority,
            authorization_effective_at,
            verification_as_of,
        )
        .expect_err("planted invalid Resolution must fail")
    }

    struct PinnedResolutionFixture {
        root: Utf8PathBuf,
        repo: Repository,
        authority: AuthorityRegister,
        warrant: WarrantBinding,
        resolution: Resolution,
        resolution_binding: ContentBinding,
        resolution_digest: String,
        snapshot: RepositorySnapshot,
    }

    impl Drop for PinnedResolutionFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn fixture_atom(uuid: &str, role: &str, order: u32, body: &str) -> String {
        format!(
            "---\nschema: oh.war/atom/v1\nwarrant_uuid: {uuid}\nrole: {role}\n\
             jurisdiction: authored\norder: {order}\nclassification: internal\n---\n\n{body}\n"
        )
    }

    fn fixture_authority() -> AuthorityRegister {
        AuthorityRegister::new(vec![
            RoleAssignment {
                actor: "Authorizer".to_owned(),
                actor_kind: ActorKind::Human,
                roles: BTreeSet::from([ActorRole::Authorizer]),
                assigned_by: "human:owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                note: None,
            },
            RoleAssignment {
                actor: "Resolver".to_owned(),
                actor_kind: ActorKind::Human,
                roles: BTreeSet::from([ActorRole::Resolver]),
                assigned_by: "human:owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                note: None,
            },
            RoleAssignment {
                actor: "fixture-gate-runner".to_owned(),
                actor_kind: ActorKind::Agent,
                roles: BTreeSet::from([ActorRole::Verifier]),
                assigned_by: "human:owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                note: Some("Verifier-controlled fixture Gate runner.".to_owned()),
            },
            RoleAssignment {
                actor: "gate_runner".to_owned(),
                actor_kind: ActorKind::Agent,
                roles: BTreeSet::from([ActorRole::Verifier]),
                assigned_by: "human:owner".to_owned(),
                effective_time: "2026-08-25T00:00:00Z".to_owned(),
                note: Some("Receipt-producing fixture Gate runner.".to_owned()),
            },
        ])
    }

    fn recompute_fixture_evaluation(
        fixture: &PinnedResolutionFixture,
        snapshot: &RepositorySnapshot,
        resolution: &Resolution,
    ) -> crate::resolve::RecordedResolutionEvaluation {
        let verified = verify_warrant(
            &fixture.repo,
            &fixture.warrant,
            "019c0000-0000-7000-8000-000000000099",
            &fixture.authority,
            snapshot,
            "2026-08-30T00:00:00Z",
        )
        .expect("fixture Warrant verifies");
        let evidence =
            load_pinned_resolution_evidence(&fixture.repo, &verified, &fixture.authority, snapshot)
                .expect("fixture evidence loads from snapshot");
        let evidence_commits: Vec<crate::resolve::RecordedEvidenceCommit> =
            pinned_resolution_evidence_paths(&fixture.repo, &verified, &evidence, &fixture.warrant)
                .expect("fixture evidence paths")
                .into_iter()
                .map(|path| {
                    snapshot
                        .path_commit_identity(&path)
                        .expect("fixture evidence commit identity")
                })
                .collect();
        let performer = fixture.repo.performer();
        crate::resolve::evaluate_recorded_resolution(crate::resolve::RecordedResolutionInputs {
            one: &verified.loaded,
            verifications: &evidence.verifications,
            deliverables: &evidence.deliverables,
            gate_runs: &evidence.gate_runs,
            admissible_gate_runs: &evidence.admissible_gate_runs,
            gate_definitions: &evidence.gate_definitions,
            gate_bindings: &evidence.gate_bindings,
            gate_receipts: &evidence.gate_receipts,
            gate_outputs: &evidence.gate_outputs,
            authorization: &verified.authorization,
            current_contract_digest: &verified.current_contract_digest,
            judgments: &evidence.judgments,
            assumptions: evidence.assumptions.as_deref(),
            register: &fixture.authority,
            policy_allows_automated_resolution: fixture
                .repo
                .config
                .policy
                .allow_automated_resolution,
            performer: &performer,
            resolver: crate::resolve::RecordedResolver {
                actor: "Resolver",
                effective_at: &resolution.effective_at,
            },
            artifacts: evidence.artifact_checks,
            artifact_entries: &evidence.artifact_entries,
            sources: crate::resolve::RecordedEvidenceSources {
                verification_refs: &evidence.verification_refs,
                deliverables_ref: &evidence.deliverables_ref,
                gate_run_source_refs: &evidence.gate_run_source_refs,
                gate_binding_source_refs: &evidence.gate_binding_source_refs,
                gate_selection_source_refs: &evidence.gate_selection_source_refs,
                gate_fixture_source_refs: &evidence.gate_fixture_source_refs,
                gate_raw_evidence_source_refs: &evidence.gate_raw_evidence_source_refs,
                gate_receipt_source_refs: &evidence.gate_receipt_source_refs,
                evidence_commits: &evidence_commits,
                judgments_ref: evidence.judgments_ref.as_deref(),
                rationale_ref: evidence.rationale_ref.as_deref(),
            },
        })
        .expect("fixture Resolution evaluation")
    }

    fn write_fixture_resolution(fixture: &mut PinnedResolutionFixture, resolution: Resolution) {
        let bytes = serde_json::to_vec(&resolution).expect("Resolution JSON");
        let path = fixture
            .root
            .join("docs/warrants/OW-WAR-0099/resolution.json");
        fs::write(&path, &bytes).expect("write Resolution");
        fixture.resolution_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::Resolution, &resolution).expect("Resolution digest")
        );
        fixture.resolution_binding = ContentBinding {
            reference: "docs/warrants/OW-WAR-0099/resolution.json".to_owned(),
            sha256: content_digest(&bytes),
            extensions: Default::default(),
        };
        fixture.resolution = resolution;
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-28T00:00:00Z"));
    }

    fn pinned_resolution_fixture() -> PinnedResolutionFixture {
        #[derive(serde::Serialize)]
        struct DeliverableFile<'a> {
            deliverable: &'a [Deliverable],
        }

        let root = scratch("resolved-warrant-e2e");
        let warrant_dir = root.join("docs/warrants/OW-WAR-0099");
        fs::create_dir_all(warrant_dir.join("atoms")).expect("Warrant atoms directory");
        fs::create_dir_all(warrant_dir.join("verifications"))
            .expect("Warrant verification directory");
        fs::create_dir_all(root.join("docs/receipts")).expect("receipt directory");
        let uuid = "019c0000-0000-7000-8000-000000000099";
        fs::write(
            warrant_dir.join("manifest.toml"),
            format!(
                r#"schema = "oh.war/manifest/v1"
uuid = "{uuid}"
local_alias = "OW-WAR-0099"
enterprise_id = ""
title = "Resolved Warrant verifier fixture"
profile = "delivery"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"
required = true

[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true

[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true

[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true

[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
"#
            ),
        )
        .expect("write manifest");
        for (name, role, order) in [
            ("10-intent.md", "intent", 10),
            ("20-basis.md", "basis", 20),
            ("40-work-order.md", "work_order", 40),
        ] {
            fs::write(
                warrant_dir.join("atoms").join(name),
                fixture_atom(uuid, role, order, &format!("# {role}\n\nFixture {role}.")),
            )
            .expect("write Markdown atom");
        }
        fs::write(
            warrant_dir.join("atoms/45-milestones.yaml"),
            r#"schema: "oh.war/milestones/v1"
milestones:
  - id: "M-001"
    title: "Finish fixture"
    stage_refs: ["STAGE-001"]
    obligation_refs: ["OBL-001"]
stages:
  - id: "STAGE-001"
    title: "Produce fixture"
    executor_kind: "human"
    responsibility_tier: "T1"
"#,
        )
        .expect("write milestones atom");
        fs::write(
            warrant_dir.join("atoms/60-assurance.md"),
            fixture_atom(
                uuid,
                "assurance",
                60,
                r#"# Assurance

## Acceptance Obligations

### OBL-001 — fixture artifact is exact
- **scope:** this one committed fixture artifact.
- **gate:** `gate://fixture.pass@1.0.0`
- **evidence:** the independently verified artifact digest.

## Gate Adequacy

**Adversarial question: could a stale artifact pass?** The digest plant changes its bytes.

**Executed attacks:** stale artifact and missing gate evidence."#,
            ),
        )
        .expect("write assurance atom");

        let repo = Repository {
            root: root.clone(),
            config: RepositoryConfig::new(
                "resolved-warrant-fixture",
                Namespace::parse("OW").expect("fixture namespace"),
            ),
        };
        let loaded = repo
            .load_warrant(&warrant_dir)
            .expect("fixture Warrant loads");
        assert!(
            loaded.report.is_ready(),
            "fixture Warrant must compile: {:?}",
            loaded.report
        );
        let basis = loaded.basis.as_ref().expect("fixture basis");
        let validated = loaded.validated.as_ref().expect("fixture manifest");
        let ir = lower(basis, validated).expect("lower fixture Warrant");
        let contract_digest = ir.contract_digest().expect("fixture contract digest");
        let contract_subject = format!(
            "contract:sha256:{}",
            contract_digest
                .strip_prefix("sha256:")
                .unwrap_or(&contract_digest)
        );

        let mut authorization = AuthorizationRecord {
            schema: AUTHORIZATION_SCHEMA.to_owned(),
            warrant: "OW-WAR-0099".to_owned(),
            revision: ContractRevision::draft(
                contract_digest.clone(),
                ir.contract_coverage.clone(),
            )
            .propose("claude")
            .expect("propose fixture")
            .authorize(Authorization {
                authorizer: "Authorizer".to_owned(),
                actor_kind: ActorKind::Human,
                acting_role: ActorRole::Authorizer.to_string(),
                meaning: "Authorize exact fixture contract.".to_owned(),
                effective_time: "2026-08-26T00:00:00Z".to_owned(),
                policy_basis: None,
                independence: AuthIndependence::SeparateRole,
            })
            .expect("authorize fixture"),
            gate_bindings: Some(vec![]),
        };
        fs::write(root.join("artifact.txt"), b"exact artifact\n").expect("write artifact");
        let artifact_digest = content_digest(b"exact artifact\n");
        let deliverables = vec![Deliverable {
            id: "DEL-001".to_owned(),
            title: "Fixture artifact".to_owned(),
            kind: DeliverableKind::File,
            target_ref: "artifact.txt".to_owned(),
            required: true,
            content_addressed: true,
            provenance_required: true,
            obligation_refs: vec!["OBL-001".to_owned()],
            provenance: Some(ArtifactProvenance {
                producer: "claude".to_owned(),
                producing_attempt: "attempt://fixture".to_owned(),
                contract_digest: contract_digest.clone(),
                input_digests: vec![],
                tool_or_runtime_identity: "test".to_owned(),
                creation_method: "authored".to_owned(),
                content_digest: artifact_digest.clone(),
                media_type: "text/plain".to_owned(),
                classification: "internal".to_owned(),
                retention: "test".to_owned(),
                source_holder: "git".to_owned(),
            }),
        }];
        fs::write(
            warrant_dir.join("deliverables.toml"),
            toml::to_string_pretty(&DeliverableFile {
                deliverable: &deliverables,
            })
            .expect("serialize deliverables"),
        )
        .expect("write deliverables");
        let verification = Verification {
            obligation: "OBL-001".to_owned(),
            verifier: Verifier {
                actor: "independent-reviewer".to_owned(),
                kind: VerifierKind::Agent,
                model: Some("fixture-model".to_owned()),
                independence: VerificationIndependence::default(),
            },
            disposition: Disposition::Established,
            evidence: "gate-run://GR-1".to_owned(),
            performer: "claude".to_owned(),
        };
        fs::write(
            warrant_dir.join("verifications/OBL-001.toml"),
            toml::to_string_pretty(&verification).expect("serialize verification"),
        )
        .expect("write verification");
        fs::write(warrant_dir.join("rationale.toml"), "").expect("write empty rationale");
        let gate_run = GateRun {
            id: "GR-1".to_owned(),
            gate: "fixture.pass@1.0.0".to_owned(),
            askability: openwarrant_core::Askability::Askable,
            execution_status: openwarrant_core::ExecutionStatus::Completed,
            verdict: openwarrant_core::Verdict::Pass,
            reason_code: Some(openwarrant_core::ReasonCode::Passed),
        };
        let gate_run_bytes = toml::to_string_pretty(&gate_run).expect("serialize Gate Run");
        fs::write(
            root.join("docs/receipts/GR-1.run.toml"),
            gate_run_bytes.as_bytes(),
        )
        .expect("write Gate Run");
        let gate_definition_bytes = br#"gate_id: "fixture.pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "artifact://fixture-runner"
output_schema_ref: "schema://fixture-gate/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["fixture"]
selection_manifest: ["test://fixture-pass"]
mutating: "false"
timeout_secs: "5"
fault_model: ["fixture-failure"]
known_blind_spots: ["Fixture-only control."]
qualification_qualifier: "fixture verifier"
qualification_digest: ""
qualification_positive_controls: ["Known failure is rejected."]
qualification_negative_controls: ["Known pass is accepted."]
qualification_mutation_classes: ["fixture substitution"]
qualification_environments: ["test"]
qualification_limitations: ["Fixture-only."]
detection_results:
  - fault_class: "fixture-failure"
    mutation: "replace known pass"
    detected: "true"
"#;
        fs::create_dir_all(root.join("docs/gates")).expect("Gate Definition directory");
        fs::write(
            root.join("docs/gates/fixture.pass@1.0.0.yaml"),
            gate_definition_bytes,
        )
        .expect("write Gate Definition");
        let gate_definition_digest = content_digest(gate_definition_bytes);
        let gate_binding = GateBinding {
            id: "GB-1".to_owned(),
            gate: openwarrant_core::GateRef {
                id: "fixture.pass".to_owned(),
                version: "1.0.0".to_owned(),
                digest: gate_definition_digest.clone(),
            },
            subjects: vec![contract_subject.clone(), artifact_digest.clone()],
            fixtures: vec![],
            parameters: BTreeMap::new(),
            pass_predicate: BTreeMap::new(),
            evidence_policy: openwarrant_core::EvidencePolicy {
                producer: "gate_runner".to_owned(),
                performer_authored_report_admissible: false,
            },
        };
        let gate_binding_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, &gate_binding).expect("Gate Binding digest")
        );
        fs::write(
            root.join("docs/receipts/GR-1.binding.json"),
            serde_json::to_vec(&gate_binding).expect("serialize Gate Binding"),
        )
        .expect("write Gate Binding");
        authorization.gate_bindings = Some(vec![crate::authorize::AuthorizedGateBinding {
            id: gate_binding.id.clone(),
            gate: gate_binding.gate.key(),
            digest: gate_binding_digest.clone(),
            source_ref: "docs/receipts/GR-1.binding.json".to_owned(),
        }]);
        let authorization_bytes = toml::to_string_pretty(&authorization)
            .expect("serialize authorization")
            .into_bytes();
        fs::write(warrant_dir.join("authorization.toml"), &authorization_bytes)
            .expect("write authorization");
        let selection_observation = openwarrant_core::TestSelectionObservation {
            schema: openwarrant_core::TEST_SELECTION_OBSERVATION_SCHEMA.to_owned(),
            kind: openwarrant_core::TEST_SELECTION_OBSERVATION_KIND.to_owned(),
            run_id: "GR-1".to_owned(),
            gate_definition_digest: gate_definition_digest.clone(),
            adapter: "adapter://openwarrant/fixture-required-pass@1.0.0".to_owned(),
            selected_test_count: 1,
            selected_test_manifest: vec!["test://fixture-pass".to_owned()],
        };
        let selection_bytes =
            serde_json::to_vec(&selection_observation).expect("serialize selection observation");
        fs::write(
            root.join("docs/receipts/GR-1.selection.json"),
            &selection_bytes,
        )
        .expect("write selection observation");
        let mut gate_receipt = GateReceipt {
            schema: openwarrant_core::GATE_RECEIPT_SCHEMA.to_owned(),
            kind: openwarrant_core::GATE_RECEIPT_KIND.to_owned(),
            run_id: "GR-1".to_owned(),
            gate_run_digest: content_digest(gate_run_bytes.as_bytes()),
            gate_definition_digest,
            gate_binding_digest,
            subject_digests: vec![contract_subject, artifact_digest],
            fixture_digests: vec![],
            runner: selection_observation.adapter.clone(),
            producer_actor: "gate_runner".to_owned(),
            runtime_environment: "fixture".to_owned(),
            arguments: vec!["fixture".to_owned()],
            working_directory: ".".to_owned(),
            started_at: "2026-08-26T00:00:00Z".to_owned(),
            completed_at: "2026-08-26T00:00:01Z".to_owned(),
            exit_result: openwarrant_core::GateExitResult::ExitCode { code: 0 },
            selected_test_count: 1,
            selected_test_manifest: vec!["test://fixture-pass".to_owned()],
            selection_observation_ref: "docs/receipts/GR-1.selection.json".to_owned(),
            selection_observation_digest: content_digest(&selection_bytes),
            raw_evidence_refs: vec!["artifact://DEL-001".to_owned()],
            stdout_ref: "docs/receipts/GR-1.stdout.txt".to_owned(),
            stdout_digest: content_digest(b"fixture gate stdout\n"),
            stderr_ref: "docs/receipts/GR-1.stderr.txt".to_owned(),
            stderr_digest: content_digest(b"fixture gate stderr\n"),
            resource_usage: "fixture".to_owned(),
            verdict: openwarrant_core::Verdict::Pass,
            receipt_digest: String::new(),
            extensions: openwarrant_core::GateReceiptExtensions::default(),
        };
        gate_receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateReceipt, &gate_receipt).expect("Gate receipt digest")
        );
        fs::write(
            root.join("docs/receipts/GR-1.receipt.json"),
            serde_json::to_vec(&gate_receipt).expect("serialize Gate receipt"),
        )
        .expect("write Gate receipt");
        fs::write(
            root.join("docs/receipts/GR-1.stdout.txt"),
            b"fixture gate stdout\n",
        )
        .expect("write Gate stdout");
        fs::write(
            root.join("docs/receipts/GR-1.stderr.txt"),
            b"fixture gate stderr\n",
        )
        .expect("write Gate stderr");

        fs::write(
            root.join("openwarrant.toml"),
            toml::to_string_pretty(&repo.config).expect("serialize fixture repository config"),
        )
        .expect("write fixture repository config");
        let authority = fixture_authority();
        #[derive(serde::Serialize)]
        struct AuthorityFile<'a> {
            assignment: &'a [RoleAssignment],
        }
        fs::create_dir_all(root.join("docs/authority")).expect("authority directory");
        fs::write(
            root.join("docs/authority/roles.toml"),
            toml::to_string_pretty(&AuthorityFile {
                assignment: &authority.assignments,
            })
            .expect("serialize fixture authority"),
        )
        .expect("write fixture authority");
        let warrant = WarrantBinding {
            warrant_id: "OW-WAR-0099".to_owned(),
            authorized_revision: 1,
            contract_digest,
            authorization_ref: "docs/warrants/OW-WAR-0099/authorization.toml".to_owned(),
            authorization_sha256: content_digest(&authorization_bytes),
            extensions: Default::default(),
        };
        let first_snapshot = commit_snapshot_at(&root, Some("2026-08-26T00:00:00Z"));
        let mut fixture = PinnedResolutionFixture {
            root,
            repo,
            authority,
            warrant,
            resolution: accepted_resolution(),
            resolution_binding: ContentBinding {
                reference: String::new(),
                sha256: String::new(),
                extensions: Default::default(),
            },
            resolution_digest: String::new(),
            snapshot: first_snapshot,
        };
        fixture.resolution.contract_digest = fixture.warrant.contract_digest.clone();
        fixture.resolution.effective_at = "2026-08-27T00:00:00Z".to_owned();
        fixture.resolution.recorded_at = "2026-08-27T00:00:01Z".to_owned();
        let evaluation =
            recompute_fixture_evaluation(&fixture, &fixture.snapshot, &fixture.resolution);
        fixture.resolution.assurance_case_snapshot_digest =
            evaluation.assurance_case_snapshot_digest;
        fixture.resolution.artifact_manifest_digest = evaluation.artifact_manifest_digest;
        fixture.resolution.gate_run_refs = evaluation.gate_run_refs;
        fixture.resolution.judgment_refs = evaluation.judgment_refs;
        fixture.resolution.residual_risk_refs = evaluation.residual_risk_refs;
        let resolution = fixture.resolution.clone();
        write_fixture_resolution(&mut fixture, resolution);
        fixture
    }

    #[test]
    fn resolved_warrant_accepts_only_exact_bound_reliable_resolution() {
        let resolution = accepted_resolution();
        let (binding, semantic, bytes, warrant) = bound_resolution(&resolution);
        let parsed = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect("exact accepted Resolution");
        assert_eq!(parsed.actor, "Resolver");
        validate_resolution_evaluation(
            &parsed.resolution,
            &synthetic_evaluation(&parsed.resolution, ResolutionChecks::all_met(), Some(true)),
        )
        .expect("recomputed evidence accepts outcome");

        let mut tampered = bytes.clone();
        tampered.push(b' ');
        let error = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &tampered,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("raw ContentBinding must bind exact bytes");
        assert!(error.to_string().contains("content digest mismatch"));
    }

    #[test]
    fn resolved_warrant_refuses_semantic_or_contract_substitution() {
        let resolution = accepted_resolution();
        let (binding, semantic, bytes, mut warrant) = bound_resolution(&resolution);
        let error = parse_bound_resolution(
            &binding,
            &format!("sha256:{}", "0".repeat(64)),
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("semantic Resolution digest mismatch must fail");
        assert!(error.to_string().contains("semantic digest mismatch"));

        warrant.authorized_revision += 1;
        let error = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("different authorized revision must fail");
        assert!(error.to_string().contains("authorized revision"));
    }

    #[test]
    fn resolved_warrant_refuses_named_resolver_without_effective_role_or_actual_checks() {
        let resolution = accepted_resolution();
        let (binding, semantic, bytes, warrant) = bound_resolution(&resolution);
        let error = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-28T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("future Resolver assignment must fail");
        assert!(error.to_string().contains("effective Resolver assignment"));

        let mut checks = ResolutionChecks::all_met();
        checks.artifact_digests_verify = false;
        let error = validate_resolution_evaluation(
            &resolution,
            &synthetic_evaluation(&resolution, checks, Some(true)),
        )
        .expect_err("one unmet §56.1 check must fail");
        assert!(error.to_string().contains("artifact digests verify"));

        let error = validate_resolution_evaluation(
            &resolution,
            &synthetic_evaluation(&resolution, ResolutionChecks::all_met(), Some(false)),
        )
        .expect_err("serialized satisfied outcome cannot override §38.6 evidence");
        assert!(error.to_string().contains("§38.6"));

        let mut mismatched =
            synthetic_evaluation(&resolution, ResolutionChecks::all_met(), Some(true));
        mismatched.gate_run_refs = vec!["different-run".to_owned()];
        let error = validate_resolution_evaluation(&resolution, &mismatched)
            .expect_err("different Gate Run references must fail");
        assert!(
            error
                .to_string()
                .contains("Gate Run references do not match")
        );
    }

    #[test]
    fn resolved_warrant_repository_path_accepts_exact_pinned_evidence() {
        let fixture = pinned_resolution_fixture();
        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect("exact repository-backed Resolution must verify");
    }

    #[test]
    fn resolved_warrant_ignores_an_unreceipted_gate_run() {
        let mut fixture = pinned_resolution_fixture();
        let orphan = GateRun {
            id: "GR-orphan".to_owned(),
            gate: "fixture.pass@1.0.0".to_owned(),
            askability: openwarrant_core::Askability::Askable,
            execution_status: openwarrant_core::ExecutionStatus::Completed,
            verdict: openwarrant_core::Verdict::Pass,
            reason_code: Some(openwarrant_core::ReasonCode::Passed),
        };
        fs::write(
            fixture.root.join("docs/receipts/GR-orphan.run.toml"),
            toml::to_string_pretty(&orphan).expect("serialize orphan Gate Run"),
        )
        .expect("write orphan Gate Run");
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-29T00:00:00Z"));

        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect("unreceipted Gate Run is not a Resolution evidence candidate");
    }

    #[test]
    fn resolved_warrant_ignores_a_malformed_bundle_for_an_uncited_gate() {
        let mut fixture = pinned_resolution_fixture();
        let unrelated = GateRun {
            id: "GR-unrelated".to_owned(),
            gate: "fixture.uncited@1.0.0".to_owned(),
            askability: openwarrant_core::Askability::Askable,
            execution_status: openwarrant_core::ExecutionStatus::Completed,
            verdict: openwarrant_core::Verdict::Pass,
            reason_code: Some(openwarrant_core::ReasonCode::Passed),
        };
        fs::write(
            fixture.root.join("docs/receipts/GR-unrelated.run.toml"),
            toml::to_string_pretty(&unrelated).expect("serialize unrelated Gate Run"),
        )
        .expect("write unrelated Gate Run");
        fs::write(
            fixture.root.join("docs/receipts/GR-unrelated.receipt.json"),
            b"{not valid JSON",
        )
        .expect("write malformed unrelated receipt");
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-29T00:00:00Z"));

        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect("uncited malformed Gate bundle is outside this Resolution evidence set");
    }

    #[test]
    fn resolved_warrant_binds_local_gate_identity_to_source_hash() {
        let mut fixture = pinned_resolution_fixture();
        let definition_path = fixture.root.join("docs/gates/fixture.pass@1.0.0.yaml");
        let definition = fs::read_to_string(&definition_path).expect("read Gate Definition");
        let declared_digest = format!("sha256:{}", "a".repeat(64));
        let definition = definition.replacen(
            "version: \"1.0.0\"\n",
            &format!("version: \"1.0.0\"\ndigest: \"{declared_digest}\"\n"),
            1,
        );
        let observed_digest = content_digest(definition.as_bytes());
        fs::write(&definition_path, definition).expect("write declared Gate digest");

        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.gate.digest = observed_digest.clone();
        receipt.gate_definition_digest = observed_digest.clone();
        let selection_path = fixture.root.join("docs/receipts/GR-1.selection.json");
        let mut selection: openwarrant_core::TestSelectionObservation =
            openwarrant_core::legacy_disposition::parse_strict_json(
                &fs::read(&selection_path).expect("read selection observation"),
            )
            .expect("parse selection observation");
        selection.gate_definition_digest = observed_digest;
        let selection_bytes =
            serde_json::to_vec(&selection).expect("serialize selection observation");
        fs::write(selection_path, &selection_bytes).expect("write selection observation");
        receipt.selection_observation_digest = content_digest(&selection_bytes);
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        authorize_fixture_gate_binding(&mut fixture, &binding);
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-26T01:00:00Z"));

        let evaluation =
            recompute_fixture_evaluation(&fixture, &fixture.snapshot, &fixture.resolution);
        let mut resolution = fixture.resolution.clone();
        resolution.assurance_case_snapshot_digest = evaluation.assurance_case_snapshot_digest;
        resolution.artifact_manifest_digest = evaluation.artifact_manifest_digest;
        resolution.gate_run_refs = evaluation.gate_run_refs;
        resolution.judgment_refs = evaluation.judgment_refs;
        resolution.residual_risk_refs = evaluation.residual_risk_refs;
        write_fixture_resolution(&mut fixture, resolution);

        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect("local Gate identity must bind exact source bytes");
    }

    #[test]
    fn resolved_warrant_ignores_an_unrelated_malformed_gate_definition() {
        let mut fixture = pinned_resolution_fixture();
        fs::write(
            fixture.root.join("docs/gates/unrelated.yaml"),
            b"this: [is: not: valid\n",
        )
        .expect("write unrelated malformed Gate Definition");
        commit_evidence_then_reseal_resolution(&mut fixture);

        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect("uncited malformed local candidates do not belong to Resolution evidence");
    }

    #[test]
    fn resolved_warrant_refuses_noncanonical_or_ineffective_gate_producers() {
        for plant in [
            "producer-whitespace",
            "blank-effective-time",
            "future-effective-time",
        ] {
            let mut fixture = pinned_resolution_fixture();
            let assignment = fixture
                .authority
                .assignments
                .iter_mut()
                .find(|assignment| assignment.actor == "gate_runner")
                .expect("fixture Gate producer assignment");
            match plant {
                "producer-whitespace" => {
                    assignment.actor.push(' ');
                    let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
                    binding.evidence_policy.producer.push(' ');
                    receipt.producer_actor.push(' ');
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "blank-effective-time" => assignment.effective_time.clear(),
                "future-effective-time" => {
                    assignment.effective_time = "2026-08-26T00:00:01Z".to_owned();
                }
                _ => unreachable!("closed producer plant inventory"),
            }
            write_fixture_authority(&fixture);

            let error = load_fixture_evidence_error(&mut fixture);
            assert!(
                error.to_string().contains("producer actor")
                    || error.to_string().contains("Verifier assignment"),
                "{plant} produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_requires_pinned_stdout_and_stderr_blobs() {
        for stream in ["stdout", "stderr"] {
            let mut fixture = pinned_resolution_fixture();
            fs::remove_file(
                fixture
                    .root
                    .join(format!("docs/receipts/GR-1.{stream}.txt")),
            )
            .expect("remove planted Gate output");

            let error = load_fixture_evidence_error(&mut fixture);
            assert!(
                error.to_string().contains(&format!("GR-1.{stream}.txt")),
                "missing {stream} produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_refuses_gate_output_digest_substitution() {
        for stream in ["stdout", "stderr"] {
            let mut fixture = pinned_resolution_fixture();
            fs::write(
                fixture
                    .root
                    .join(format!("docs/receipts/GR-1.{stream}.txt")),
                b"substituted output\n",
            )
            .expect("substitute Gate output");

            let error = load_fixture_evidence_error(&mut fixture);
            assert!(
                error.to_string().contains(&format!(
                    "{stream} \"docs/receipts/GR-1.{stream}.txt\" digest mismatch"
                )),
                "substituted {stream} produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_refuses_gate_output_outside_the_exact_bundle() {
        let mut fixture = pinned_resolution_fixture();
        let alternate = "docs/receipts/alternate.stdout.txt";
        fs::write(fixture.root.join(alternate), b"fixture gate stdout\n")
            .expect("write equal alternate stdout");
        let (binding, mut receipt) = fixture_gate_evidence(&fixture);
        receipt.stdout_ref = alternate.to_owned();
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);

        let error = load_fixture_evidence_error(&mut fixture);
        assert!(
            error.to_string().contains("exact sibling output"),
            "repointed output produced unexpected refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_a_mutating_gate_definition() {
        let mut fixture = pinned_resolution_fixture();
        let path = fixture.root.join("docs/gates/fixture.pass@1.0.0.yaml");
        let definition = fs::read_to_string(&path)
            .expect("read Gate Definition")
            .replacen("mutating: \"false\"", "mutating: \"true\"", 1);
        let definition_digest = content_digest(definition.as_bytes());
        fs::write(path, definition).expect("write mutating Gate Definition");
        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.gate.digest = definition_digest.clone();
        receipt.gate_definition_digest = definition_digest;
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);

        let error = load_fixture_evidence_error(&mut fixture);
        assert!(
            error.to_string().contains("mutating gate"),
            "mutating Gate produced unexpected refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_repository_path_refuses_stale_evidence_snapshot_and_refs() {
        let mut fixture = pinned_resolution_fixture();
        let changed_run = b"id = \"GR-1\"\ngate = \"fixture.pass@1.0.0\"\naskability = \"askable\"\nexecution_status = \"completed\"\nverdict = \"pass\"\nreason_code = \"passed\"\n# same semantics, different committed evidence object\n";
        fs::write(
            fixture.root.join("docs/receipts/GR-1.run.toml"),
            changed_run,
        )
        .expect("plant changed Gate Run source");
        let (binding, mut receipt) = fixture_gate_evidence(&fixture);
        receipt.gate_run_digest = content_digest(changed_run);
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("stale Resolution evidence binding must fail");
        assert!(
            error
                .to_string()
                .contains("assurance snapshot digest mismatch"),
            "unexpected refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_repository_path_refuses_one_actual_failed_check() {
        let mut fixture = pinned_resolution_fixture();
        fs::write(fixture.root.join("artifact.txt"), b"changed artifact\n")
            .expect("plant changed artifact");
        bind_fixture_gate_to_artifact(&mut fixture, b"changed artifact\n");
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-26T01:00:00Z"));
        let evaluation =
            recompute_fixture_evaluation(&fixture, &fixture.snapshot, &fixture.resolution);
        assert!(!evaluation.checks.artifact_digests_verify);

        let mut resolution = fixture.resolution.clone();
        resolution.assurance_case_snapshot_digest = evaluation.assurance_case_snapshot_digest;
        resolution.artifact_manifest_digest = evaluation.artifact_manifest_digest;
        resolution.gate_run_refs = evaluation.gate_run_refs;
        resolution.judgment_refs = evaluation.judgment_refs;
        resolution.residual_risk_refs = evaluation.residual_risk_refs;
        write_fixture_resolution(&mut fixture, resolution);

        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("one false actual §56.1 check must fail");
        assert!(
            error.to_string().contains("artifact digests verify"),
            "unexpected refusal: {error}"
        );
    }

    fn commit_evidence_then_reseal_resolution(fixture: &mut PinnedResolutionFixture) {
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-26T01:00:00Z"));
        let mut resolution = fixture.resolution.clone();
        resolution.profile_outcome.push_str("-resealed");
        write_fixture_resolution(fixture, resolution);
    }

    fn fixture_gate_evidence(fixture: &PinnedResolutionFixture) -> (GateBinding, GateReceipt) {
        let binding_path = fixture.root.join("docs/receipts/GR-1.binding.json");
        let binding: GateBinding = openwarrant_core::legacy_disposition::parse_strict_json(
            &fs::read(&binding_path).expect("read Gate Binding"),
        )
        .expect("parse Gate Binding");
        let receipt_path = fixture.root.join("docs/receipts/GR-1.receipt.json");
        let receipt: GateReceipt = openwarrant_core::legacy_disposition::parse_strict_json(
            &fs::read(&receipt_path).expect("read Gate receipt"),
        )
        .expect("parse Gate receipt");
        (binding, receipt)
    }

    fn write_fixture_authority(fixture: &PinnedResolutionFixture) {
        #[derive(serde::Serialize)]
        struct AuthorityFile<'a> {
            assignment: &'a [RoleAssignment],
        }

        fs::write(
            fixture.root.join("docs/authority/roles.toml"),
            toml::to_string_pretty(&AuthorityFile {
                assignment: &fixture.authority.assignments,
            })
            .expect("serialize fixture authority"),
        )
        .expect("write fixture authority");
    }

    fn authorize_fixture_gate_binding(
        fixture: &mut PinnedResolutionFixture,
        binding: &GateBinding,
    ) {
        let path = fixture
            .root
            .join("docs/warrants/OW-WAR-0099/authorization.toml");
        let mut authorization: AuthorizationRecord =
            toml::from_str(&fs::read_to_string(&path).expect("read fixture authorization"))
                .expect("parse fixture authorization");
        let digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, binding).expect("Gate Binding digest")
        );
        authorization.gate_bindings = Some(vec![crate::authorize::AuthorizedGateBinding {
            id: binding.id.clone(),
            gate: binding.gate.key(),
            digest,
            source_ref: "docs/receipts/GR-1.binding.json".to_owned(),
        }]);
        let bytes = toml::to_string_pretty(&authorization)
            .expect("serialize fixture authorization")
            .into_bytes();
        fs::write(path, &bytes).expect("write fixture authorization");
        fixture.warrant.authorization_sha256 = content_digest(&bytes);
    }

    fn load_fixture_evidence_error(fixture: &mut PinnedResolutionFixture) -> RepoError {
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-26T01:00:00Z"));
        let verified = verify_warrant(
            &fixture.repo,
            &fixture.warrant,
            "019c0000-0000-7000-8000-000000000099",
            &fixture.authority,
            &fixture.snapshot,
            "2026-08-30T00:00:00Z",
        )
        .expect("fixture Warrant verifies before evidence admission");
        load_pinned_resolution_evidence(
            &fixture.repo,
            &verified,
            &fixture.authority,
            &fixture.snapshot,
        )
        .expect_err("planted Gate evidence must be refused")
    }

    fn write_fixture_gate_evidence(
        fixture: &PinnedResolutionFixture,
        binding: &GateBinding,
        receipt: &mut GateReceipt,
    ) {
        write_named_gate_evidence(fixture, "GR-1", binding, receipt);
    }

    fn write_named_gate_evidence(
        fixture: &PinnedResolutionFixture,
        stem: &str,
        binding: &GateBinding,
        receipt: &mut GateReceipt,
    ) {
        let binding_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, binding).expect("Gate Binding digest")
        );
        fs::write(
            fixture
                .root
                .join(format!("docs/receipts/{stem}.binding.json")),
            serde_json::to_vec(binding).expect("serialize Gate Binding"),
        )
        .expect("write Gate Binding");
        receipt.gate_binding_digest = binding_digest;
        receipt.receipt_digest.clear();
        receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateReceipt, receipt).expect("Gate receipt digest")
        );
        fs::write(
            fixture
                .root
                .join(format!("docs/receipts/{stem}.receipt.json")),
            serde_json::to_vec(receipt).expect("serialize Gate receipt"),
        )
        .expect("write Gate receipt");
    }

    fn bind_fixture_gate_to_artifact(fixture: &mut PinnedResolutionFixture, artifact: &[u8]) {
        let (mut binding, mut receipt) = fixture_gate_evidence(fixture);
        let contract_hex = fixture
            .warrant
            .contract_digest
            .strip_prefix("contract:sha256:")
            .or_else(|| fixture.warrant.contract_digest.strip_prefix("sha256:"))
            .unwrap_or(&fixture.warrant.contract_digest);
        binding.subjects = vec![
            format!("contract:sha256:{contract_hex}"),
            content_digest(artifact),
        ];
        receipt.subject_digests = binding.subjects.clone();
        write_fixture_gate_evidence(fixture, &binding, &mut receipt);
        authorize_fixture_gate_binding(fixture, &binding);
    }

    fn verify_fixture_resolution_error(fixture: &PinnedResolutionFixture) -> RepoError {
        verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("planted invalid Resolution evidence must fail")
    }

    #[test]
    fn resolved_warrant_refuses_gate_fixture_inventory_mismatch() {
        let mut fixture = pinned_resolution_fixture();
        fs::write(fixture.root.join("gate-fixture.txt"), b"fixture bytes\n")
            .expect("write Gate fixture");
        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.fixtures.push(openwarrant_core::Fixture {
            reference: "gate-fixture.txt".to_owned(),
            digest: content_digest(b"fixture bytes\n"),
        });
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error
                .to_string()
                .contains("fixture inventory does not match"),
            "unexpected fixture refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_gate_producer_mismatch() {
        let mut fixture = pinned_resolution_fixture();
        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.evidence_policy.producer = "different-runner".to_owned();
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error
                .to_string()
                .contains("does not match Gate Binding producer"),
            "unexpected producer refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_forbidden_performer_authored_gate_report() {
        let mut fixture = pinned_resolution_fixture();
        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.evidence_policy.producer = fixture.repo.performer();
        receipt.producer_actor = fixture.repo.performer();
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error
                .to_string()
                .contains("forbids a performer-authored report"),
            "unexpected performer-report refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_forged_performer_and_unresolved_evidence() {
        for (label, expected) in [
            ("performer", "expected exact performer"),
            ("evidence", "unresolved or unsupported evidence"),
        ] {
            let mut fixture = pinned_resolution_fixture();
            let path = fixture
                .root
                .join("docs/warrants/OW-WAR-0099/verifications/OBL-001.toml");
            let mut verification: Verification =
                toml::from_str(&fs::read_to_string(&path).expect("read fixture verification"))
                    .expect("parse fixture verification");
            match label {
                "performer" => verification.performer = "forged".to_owned(),
                "evidence" => {
                    verification.evidence = "gate-run://DOES-NOT-EXIST".to_owned();
                }
                _ => unreachable!("closed test case inventory"),
            }
            fs::write(
                &path,
                toml::to_string_pretty(&verification).expect("serialize verification plant"),
            )
            .expect("write verification plant");
            commit_evidence_then_reseal_resolution(&mut fixture);

            let error = verify_resolved_warrant(
                ResolutionVerificationContext {
                    repo: &fixture.repo,
                    authority: &fixture.authority,
                    verification_as_of: "2026-08-30T00:00:00Z",
                },
                "019c0000-0000-7000-8000-000000000099",
                &fixture.warrant,
                &fixture.resolution_binding,
                &fixture.resolution_digest,
                &fixture.snapshot,
            )
            .expect_err("forged persisted verification must fail");
            assert!(
                error.to_string().contains(expected),
                "{label} plant produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_refuses_multiple_dispositions_for_one_obligation() {
        let mut fixture = pinned_resolution_fixture();
        let source = fixture
            .root
            .join("docs/warrants/OW-WAR-0099/verifications/OBL-001.toml");
        let mut conflicting: Verification =
            toml::from_str(&fs::read_to_string(source).expect("read verification"))
                .expect("parse verification");
        conflicting.disposition = Disposition::NotEstablished;
        fs::write(
            fixture
                .root
                .join("docs/warrants/OW-WAR-0099/verifications/OBL-001-conflict.toml"),
            toml::to_string_pretty(&conflicting).expect("serialize conflicting verification"),
        )
        .expect("write conflicting verification");
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("conflicting persisted dispositions must fail");
        assert!(
            error
                .to_string()
                .contains("2 admissible verification records"),
            "unexpected conflict refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_unbound_gate_run_subject() {
        let mut fixture = pinned_resolution_fixture();
        let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
        binding.subjects = vec!["different-contract".to_owned()];
        receipt.subject_digests = binding.subjects.clone();
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("Gate Run without exact Warrant subject binding must fail");
        assert!(
            error
                .to_string()
                .contains("subject inventory does not match Resolution basis"),
            "unexpected Gate Binding refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_undeclared_and_zero_admissible_obligations() {
        for (plant, expected) in [
            ("undeclared", "verification names undeclared obligation"),
            ("zero-admissible", "0 admissible verification records"),
        ] {
            let mut fixture = pinned_resolution_fixture();
            let path = fixture
                .root
                .join("docs/warrants/OW-WAR-0099/verifications/OBL-001.toml");
            let mut verification: Verification =
                toml::from_str(&fs::read_to_string(&path).expect("read fixture verification"))
                    .expect("parse fixture verification");
            match plant {
                "undeclared" => verification.obligation = "OBL-UNDECLARED".to_owned(),
                "zero-admissible" => verification.verifier.actor = fixture.repo.performer(),
                _ => unreachable!("closed obligation plant inventory"),
            }
            fs::write(
                &path,
                toml::to_string_pretty(&verification).expect("serialize verification plant"),
            )
            .expect("write verification plant");
            commit_evidence_then_reseal_resolution(&mut fixture);

            let error = verify_fixture_resolution_error(&fixture);
            assert!(
                error.to_string().contains(expected),
                "{plant} plant produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_refuses_each_malformed_gate_evidence_bundle() {
        for (plant, expected) in [
            ("invalid-receipt-digest", "invalid canonical receipt digest"),
            (
                "missing-receipt",
                "cites unresolved or unsupported evidence",
            ),
            ("selected-test-manifest", "declares 2 selected tests"),
            (
                "raw-evidence-inventory",
                "does not name a current Resolution artifact",
            ),
            (
                "blank-fixture-digest",
                "blank, duplicate, or noncanonical fixture_digests inventory",
            ),
            (
                "fixture-digest-mismatch",
                "fixture \"gate-fixture.txt\" digest mismatch",
            ),
            (
                "gate-definition-substitution",
                "does not bind the exact qualified Gate Definition",
            ),
            ("producer-unassigned", "has no authority assignment"),
            (
                "producer-not-verifier",
                "does not hold an effective Verifier assignment",
            ),
            ("unknown-receipt-field", "unknown field"),
            ("unknown-evidence-policy-field", "unknown field"),
            ("unknown-fixture-field", "unknown field"),
            (
                "unknown-gate-definition-field",
                "unknown Gate Definition field",
            ),
        ] {
            let mut fixture = pinned_resolution_fixture();
            let binding_path = fixture.root.join("docs/receipts/GR-1.binding.json");
            let receipt_path = fixture.root.join("docs/receipts/GR-1.receipt.json");
            match plant {
                "invalid-receipt-digest" => {
                    let (_, mut receipt) = fixture_gate_evidence(&fixture);
                    receipt.receipt_digest = format!("sha256:{}", "0".repeat(64));
                    fs::write(
                        &receipt_path,
                        serde_json::to_vec(&receipt).expect("serialize invalid receipt digest"),
                    )
                    .expect("write invalid receipt digest");
                }
                "missing-receipt" => {
                    fs::remove_file(&receipt_path).expect("remove Gate receipt");
                }
                "selected-test-manifest" => {
                    let (binding, mut receipt) = fixture_gate_evidence(&fixture);
                    receipt.selected_test_count += 1;
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "raw-evidence-inventory" => {
                    let (binding, mut receipt) = fixture_gate_evidence(&fixture);
                    receipt.raw_evidence_refs = vec!["artifact://DEL-OTHER".to_owned()];
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "blank-fixture-digest" => {
                    let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
                    binding.fixtures.push(openwarrant_core::Fixture {
                        reference: "gate-fixture.txt".to_owned(),
                        digest: String::new(),
                    });
                    receipt.fixture_digests.push(String::new());
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "fixture-digest-mismatch" => {
                    fs::write(fixture.root.join("gate-fixture.txt"), b"fixture bytes\n")
                        .expect("write Gate fixture");
                    let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
                    let wrong = content_digest(b"different fixture bytes\n");
                    binding.fixtures.push(openwarrant_core::Fixture {
                        reference: "gate-fixture.txt".to_owned(),
                        digest: wrong.clone(),
                    });
                    receipt.fixture_digests.push(wrong);
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "gate-definition-substitution" => {
                    let path = fixture.root.join("docs/gates/fixture.pass@1.0.0.yaml");
                    let mut definition = fs::read_to_string(&path).expect("read Gate Definition");
                    definition.push_str("# byte-level substitution\n");
                    fs::write(path, definition).expect("write substituted Gate Definition");
                }
                "producer-unassigned" | "producer-not-verifier" => {
                    let (mut binding, mut receipt) = fixture_gate_evidence(&fixture);
                    let producer = if plant == "producer-unassigned" {
                        "unassigned-gate-runner".to_owned()
                    } else {
                        "Resolver".to_owned()
                    };
                    binding.evidence_policy.producer = producer.clone();
                    receipt.producer_actor = producer;
                    write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
                }
                "unknown-receipt-field" => {
                    let mut value: serde_json::Value = serde_json::from_slice(
                        &fs::read(&receipt_path).expect("read Gate receipt JSON"),
                    )
                    .expect("parse Gate receipt JSON");
                    value
                        .as_object_mut()
                        .expect("Gate receipt object")
                        .insert("unexpected".to_owned(), serde_json::json!(true));
                    fs::write(
                        &receipt_path,
                        serde_json::to_vec(&value).expect("serialize unknown receipt field"),
                    )
                    .expect("write unknown receipt field");
                }
                "unknown-evidence-policy-field" => {
                    let mut value: serde_json::Value = serde_json::from_slice(
                        &fs::read(&binding_path).expect("read Gate Binding JSON"),
                    )
                    .expect("parse Gate Binding JSON");
                    value["evidence_policy"]
                        .as_object_mut()
                        .expect("evidence policy object")
                        .insert("unexpected".to_owned(), serde_json::json!(true));
                    fs::write(
                        &binding_path,
                        serde_json::to_vec(&value)
                            .expect("serialize unknown evidence-policy field"),
                    )
                    .expect("write unknown evidence-policy field");
                }
                "unknown-fixture-field" => {
                    let mut value: serde_json::Value = serde_json::from_slice(
                        &fs::read(&binding_path).expect("read Gate Binding JSON"),
                    )
                    .expect("parse Gate Binding JSON");
                    value["fixtures"] = serde_json::json!([{
                        "ref": "gate-fixture.txt",
                        "digest": "sha256:fixture",
                        "unexpected": true
                    }]);
                    fs::write(
                        &binding_path,
                        serde_json::to_vec(&value).expect("serialize unknown fixture field"),
                    )
                    .expect("write unknown fixture field");
                }
                "unknown-gate-definition-field" => {
                    let path = fixture.root.join("docs/gates/fixture.pass@1.0.0.yaml");
                    let mut definition = fs::read_to_string(&path).expect("read Gate Definition");
                    definition.push_str("arvg: [\"fixture\"]\n");
                    fs::write(path, definition).expect("write unknown Gate Definition field");
                }
                _ => unreachable!("closed Gate evidence plant inventory"),
            }
            commit_evidence_then_reseal_resolution(&mut fixture);

            let error = verify_fixture_resolution_error(&fixture);
            assert!(
                error.to_string().contains(expected),
                "{plant} plant produced unexpected refusal: {error}"
            );
        }
    }

    #[test]
    fn resolved_warrant_refuses_gate_receipt_completed_after_recording() {
        let mut fixture = pinned_resolution_fixture();
        let (binding, mut receipt) = fixture_gate_evidence(&fixture);
        receipt.completed_at = "2026-08-27T00:00:02Z".to_owned();
        write_fixture_gate_evidence(&fixture, &binding, &mut receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error
                .to_string()
                .contains("predates a bound Gate receipt completion"),
            "unexpected Gate receipt chronology refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_evidence_that_is_not_ancestral_to_resolution() {
        let mut fixture = pinned_resolution_fixture();
        let path = fixture
            .root
            .join("docs/warrants/OW-WAR-0099/verifications/OBL-001.toml");
        let mut verification = fs::read_to_string(&path).expect("read fixture verification");
        verification.push_str("\n# descendant evidence with an older claimed commit time\n");
        fs::write(path, verification).expect("write non-ancestral evidence");
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-26T02:00:00Z"));

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error
                .to_string()
                .contains("is not an ancestor of Resolution commit"),
            "unexpected strict-ancestry refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_refuses_fixture_evidence_without_a_fixture_aware_runner() {
        let mut fixture = pinned_resolution_fixture();
        fs::write(fixture.root.join("gate-fixture.txt"), b"fixture bytes\n")
            .expect("write Gate fixture");
        let correct = content_digest(b"fixture bytes\n");
        let wrong = content_digest(b"different fixture bytes\n");

        let (mut first_binding, mut first_receipt) = fixture_gate_evidence(&fixture);
        first_binding.fixtures.push(openwarrant_core::Fixture {
            reference: "gate-fixture.txt".to_owned(),
            digest: correct.clone(),
        });
        first_receipt.fixture_digests = vec![correct];
        write_fixture_gate_evidence(&fixture, &first_binding, &mut first_receipt);

        let second_run = b"id = \"GR-2\"\ngate = \"fixture.pass@1.0.0\"\naskability = \"askable\"\nexecution_status = \"completed\"\nverdict = \"pass\"\nreason_code = \"passed\"\n";
        fs::write(fixture.root.join("docs/receipts/GR-2.run.toml"), second_run)
            .expect("write second Gate Run");
        let mut second_binding = first_binding;
        second_binding.id = "GB-2".to_owned();
        second_binding.fixtures[0].digest = wrong.clone();
        let mut second_receipt = first_receipt;
        second_receipt.run_id = "GR-2".to_owned();
        second_receipt.gate_run_digest = content_digest(second_run);
        second_receipt.fixture_digests = vec![wrong];
        let selection_path = fixture.root.join("docs/receipts/GR-1.selection.json");
        let mut selection: openwarrant_core::TestSelectionObservation =
            openwarrant_core::legacy_disposition::parse_strict_json(
                &fs::read(selection_path).expect("read first selection observation"),
            )
            .expect("parse first selection observation");
        selection.run_id = "GR-2".to_owned();
        let selection_bytes =
            serde_json::to_vec(&selection).expect("serialize second selection observation");
        fs::write(
            fixture.root.join("docs/receipts/GR-2.selection.json"),
            &selection_bytes,
        )
        .expect("write second selection observation");
        second_receipt.selection_observation_ref = "docs/receipts/GR-2.selection.json".to_owned();
        second_receipt.selection_observation_digest = content_digest(&selection_bytes);
        second_receipt.stdout_ref = "docs/receipts/GR-2.stdout.txt".to_owned();
        second_receipt.stdout_digest = content_digest(b"second stdout\n");
        second_receipt.stderr_ref = "docs/receipts/GR-2.stderr.txt".to_owned();
        second_receipt.stderr_digest = content_digest(b"second stderr\n");
        fs::write(
            fixture.root.join("docs/receipts/GR-2.stdout.txt"),
            b"second stdout\n",
        )
        .expect("write second Gate stdout");
        fs::write(
            fixture.root.join("docs/receipts/GR-2.stderr.txt"),
            b"second stderr\n",
        )
        .expect("write second Gate stderr");
        write_named_gate_evidence(&fixture, "GR-2", &second_binding, &mut second_receipt);
        commit_evidence_then_reseal_resolution(&mut fixture);

        let error = verify_fixture_resolution_error(&fixture);
        assert!(
            error.to_string().contains("fixtures unsupported"),
            "unexpected fixture-adapter refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_repository_path_refuses_evidence_committed_after_recording() {
        let mut fixture = pinned_resolution_fixture();
        let verification_path = fixture
            .root
            .join("docs/warrants/OW-WAR-0099/verifications/OBL-001.toml");
        let mut verification =
            fs::read_to_string(&verification_path).expect("read fixture verification");
        verification.push_str("\n# committed after Resolution recording\n");
        fs::write(&verification_path, verification)
            .expect("plant evidence after Resolution recording");
        fixture.snapshot = commit_snapshot_at(&fixture.root, Some("2026-08-29T00:00:00Z"));

        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000099",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("evidence committed after Resolution recorded_at must fail");
        assert!(
            error.to_string().contains("predates pinned evidence"),
            "unexpected chronology refusal: {error}"
        );
    }

    #[test]
    fn resolved_warrant_repository_path_refuses_wrong_expected_uuid() {
        let fixture = pinned_resolution_fixture();
        let error = verify_resolved_warrant(
            ResolutionVerificationContext {
                repo: &fixture.repo,
                authority: &fixture.authority,
                verification_as_of: "2026-08-30T00:00:00Z",
            },
            "019c0000-0000-7000-8000-000000000098",
            &fixture.warrant,
            &fixture.resolution_binding,
            &fixture.resolution_digest,
            &fixture.snapshot,
        )
        .expect_err("different expected Warrant UUID must fail");
        assert!(error.to_string().contains("expected alias"));
    }

    #[test]
    fn resolved_warrant_record_requires_canonical_uuidv7_and_profile_outcome() {
        let mut resolution = accepted_resolution();
        resolution.id = "not-a-uuid".to_owned();
        let (binding, semantic, bytes, warrant) = bound_resolution(&resolution);
        let error = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("non-UUIDv7 Resolution id must fail");
        assert!(error.to_string().contains("UUIDv7"));

        resolution = accepted_resolution();
        resolution.profile_outcome.clear();
        let (binding, semantic, bytes, warrant) = bound_resolution(&resolution);
        let error = parse_bound_resolution(
            &binding,
            &semantic,
            &warrant,
            &bytes,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        )
        .expect_err("blank profile outcome must fail");
        assert!(error.to_string().contains("profile outcome"));

        resolution = accepted_resolution();
        resolution.id = "019c0000-0000-7000-c000-000000000001".to_owned();
        let (_, _, _, warrant) = bound_resolution(&accepted_resolution());
        let error = parse_mutated_resolution(
            &resolution,
            &warrant,
            &human_resolver_register("2026-08-26T00:00:00Z"),
            "2026-08-26T00:00:00Z",
            "2026-08-30T00:00:00Z",
        );
        assert!(error.to_string().contains("RFC 4122 UUIDv7"));
    }

    #[test]
    fn resolved_warrant_record_rejects_each_semantic_and_authority_substitution() {
        let base = accepted_resolution();
        let (_, _, _, warrant) = bound_resolution(&base);
        let register = human_resolver_register("2026-08-26T00:00:00Z");

        let mut changed = base.clone();
        changed.contract_digest = "f".repeat(64);
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("contract digest")
        );

        changed = base.clone();
        changed.common_outcome = CommonOutcome::NotSatisfied;
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("does not accept")
        );

        changed = base.clone();
        changed.standing = ResolutionStanding::Disputed;
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("cannot be relied upon")
        );

        changed = base.clone();
        changed.effective_at = "2026-08-28T00:00:00Z".to_owned();
        changed.recorded_at = "2026-08-27T00:00:01Z".to_owned();
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("chronology is invalid")
        );

        changed = base.clone();
        changed.acting_role_ref = "role-assignment://judge".to_owned();
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("role-assignment://resolver")
        );

        changed = base.clone();
        changed.resolved_by_ref = "agent://Resolver".to_owned();
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &register,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("must use person:// or policy-service://")
        );

        changed = base.clone();
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &AuthorityRegister::new(vec![]),
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("no authority assignment")
        );

        let wrong_kind = AuthorityRegister::new(vec![RoleAssignment {
            actor: "Resolver".to_owned(),
            actor_kind: ActorKind::PolicyService,
            roles: BTreeSet::from([ActorRole::Resolver]),
            assigned_by: "human:owner".to_owned(),
            effective_time: "2026-08-26T00:00:00Z".to_owned(),
            note: None,
        }]);
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &wrong_kind,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("lacks an effective Resolver assignment")
        );

        let wrong_role = AuthorityRegister::new(vec![RoleAssignment {
            actor: "Resolver".to_owned(),
            actor_kind: ActorKind::Human,
            roles: BTreeSet::from([ActorRole::Judge]),
            assigned_by: "human:owner".to_owned(),
            effective_time: "2026-08-26T00:00:00Z".to_owned(),
            note: None,
        }]);
        assert!(
            parse_mutated_resolution(
                &changed,
                &warrant,
                &wrong_role,
                "2026-08-26T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .to_string()
            .contains("lacks an effective Resolver assignment")
        );
    }

    fn write_response_and_snapshot(
        root: &Utf8Path,
        receipt: &mut LegacyAdrDispositionReceipt,
        response: &LegacyAdrDispositionReviewResponse,
    ) -> RepositorySnapshot {
        let path = root.join(&receipt.payload.review.response.reference);
        fs::create_dir_all(path.parent().expect("response parent")).expect("response directory");
        let bytes = serde_json::to_vec(response).expect("response JSON");
        fs::write(&path, &bytes).expect("write response");
        receipt.payload.review.response.sha256 = content_digest(&bytes);
        commit_snapshot(root)
    }

    #[test]
    fn receipt_digest_is_over_typed_unsigned_envelope() {
        let mut value = receipt();
        value.receipt_digest = artifact_digest(&value.digest_preimage()).expect("digest");
        assert_eq!(
            artifact_digest(&value.digest_preimage()).expect("digest"),
            value.receipt_digest
        );
    }

    #[test]
    fn required_gate_pass_needs_authority_context() {
        let binding = |reference: &str| ContentBinding {
            reference: reference.to_owned(),
            sha256: format!("sha256:{}", "a".repeat(64)),
            extensions: Default::default(),
        };
        let mut one = receipt();
        one.payload.evidence = vec![LegacyAdrSupport::RequiredGatePass {
            run: binding("ow:docs/receipts/run-1.run.toml"),
            definition: binding("ow:docs/gates/gate-1.yaml"),
            binding: binding("ow:docs/receipts/run-1.binding.json"),
            selection: Box::new(binding("ow:docs/receipts/run-1.selection.json")),
            stdout: Box::new(binding("ow:docs/receipts/run-1.stdout.txt")),
            stderr: Box::new(binding("ow:docs/receipts/run-1.stderr.txt")),
            fixtures: vec![],
            receipt: Box::new(binding("ow:docs/receipts/run-1.receipt.json")),
            subject_source_sha256: one.payload.source.source_sha256.clone(),
            extensions: Default::default(),
        }];
        let error = verify_support(
            Utf8Path::new("/not-used"),
            Utf8Path::new("/not-used"),
            &one,
            &mut BTreeSet::new(),
            &mut CheckedRefs::new(),
            &fake_snapshot(Utf8Path::new("/not-used")),
            &mut SupportState::default(),
            &InternalArtifacts::default(),
            None,
        )
        .expect_err("Gate evidence cannot be admitted without authority context");
        assert!(
            error
                .to_string()
                .contains("requires repository and authority context")
        );
    }

    #[test]
    fn required_gate_pass_admits_one_complete_bound_bundle() {
        let root = scratch("legacy-required-gate-pass");
        let receipts = root.join("docs/receipts");
        let gates = root.join("docs/gates");
        fs::create_dir_all(&receipts).expect("Gate receipt directory");
        fs::create_dir_all(&gates).expect("Gate Definition directory");

        let repo = Repository {
            root: root.clone(),
            config: RepositoryConfig::new(
                "legacy-required-gate-pass-fixture",
                Namespace::parse("OW").expect("fixture namespace"),
            ),
        };
        let run = GateRun {
            id: "GR-LEGACY-1".to_owned(),
            gate: "fixture.legacy-pass@1.0.0".to_owned(),
            askability: openwarrant_core::Askability::Askable,
            execution_status: openwarrant_core::ExecutionStatus::Completed,
            verdict: openwarrant_core::Verdict::Pass,
            reason_code: Some(openwarrant_core::ReasonCode::Passed),
        };
        let run_bytes = toml::to_string_pretty(&run)
            .expect("serialize Gate Run")
            .into_bytes();
        fs::write(receipts.join("GR-LEGACY-1.run.toml"), &run_bytes).expect("write Gate Run");

        let definition_bytes = br#"gate_id: "fixture.legacy-pass"
version: "1.0.0"
lifecycle: "qualified"
implementation_ref: "artifact://fixture-runner"
output_schema_ref: "schema://fixture-gate/v1"
provenance: "local_candidate"
input_kinds: ["fixture"]
argv: ["fixture-pass"]
selection_manifest: ["check://fixture-pass"]
mutating: "false"
timeout_secs: "5"
fault_model: ["fixture-failure"]
known_blind_spots: ["Fixture-only control."]
qualification_qualifier: "fixture verifier"
qualification_digest: ""
qualification_positive_controls: ["Known failure is rejected."]
qualification_negative_controls: ["Known pass is accepted."]
qualification_mutation_classes: ["fixture substitution"]
qualification_environments: ["test"]
qualification_limitations: ["Fixture-only."]
detection_results:
  - fault_class: "fixture-failure"
    mutation: "replace known pass"
    detected: "true"
"#;
        fs::write(
            gates.join("fixture.legacy-pass@1.0.0.yaml"),
            definition_bytes,
        )
        .expect("write Gate Definition");
        let definition_digest = content_digest(definition_bytes);

        let subject_digest = format!("sha256:{}", "a".repeat(64));
        let gate_binding = GateBinding {
            id: "GB-LEGACY-1".to_owned(),
            gate: openwarrant_core::GateRef {
                id: "fixture.legacy-pass".to_owned(),
                version: "1.0.0".to_owned(),
                digest: definition_digest.clone(),
            },
            subjects: vec![subject_digest.clone()],
            fixtures: vec![],
            parameters: BTreeMap::new(),
            pass_predicate: BTreeMap::new(),
            evidence_policy: openwarrant_core::EvidencePolicy {
                producer: "gate_runner".to_owned(),
                performer_authored_report_admissible: false,
            },
        };
        let binding_bytes = serde_json::to_vec(&gate_binding).expect("serialize Gate Binding");
        fs::write(receipts.join("GR-LEGACY-1.binding.json"), &binding_bytes)
            .expect("write Gate Binding");
        let binding_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateBinding, &gate_binding).expect("Gate Binding digest")
        );

        let stdout_bytes = b"fixture gate stdout\n";
        let stderr_bytes = b"fixture gate stderr\n";
        fs::write(receipts.join("GR-LEGACY-1.stdout.txt"), stdout_bytes)
            .expect("write Gate stdout");
        fs::write(receipts.join("GR-LEGACY-1.stderr.txt"), stderr_bytes)
            .expect("write Gate stderr");
        let selection_observation = openwarrant_core::TestSelectionObservation {
            schema: openwarrant_core::TEST_SELECTION_OBSERVATION_SCHEMA.to_owned(),
            kind: openwarrant_core::TEST_SELECTION_OBSERVATION_KIND.to_owned(),
            run_id: run.id.clone(),
            gate_definition_digest: definition_digest.clone(),
            adapter: "adapter://openwarrant/fixture-legacy-pass@1.0.0".to_owned(),
            selected_test_count: 1,
            selected_test_manifest: vec!["check://fixture-pass".to_owned()],
        };
        let selection_bytes =
            serde_json::to_vec(&selection_observation).expect("serialize selection observation");
        fs::write(
            receipts.join("GR-LEGACY-1.selection.json"),
            &selection_bytes,
        )
        .expect("write selection observation");
        let mut gate_receipt = GateReceipt {
            schema: openwarrant_core::GATE_RECEIPT_SCHEMA.to_owned(),
            kind: openwarrant_core::GATE_RECEIPT_KIND.to_owned(),
            run_id: run.id.clone(),
            gate_run_digest: content_digest(&run_bytes),
            gate_definition_digest: definition_digest,
            gate_binding_digest: binding_digest,
            subject_digests: vec![subject_digest.clone()],
            fixture_digests: vec![],
            runner: selection_observation.adapter.clone(),
            producer_actor: "gate_runner".to_owned(),
            runtime_environment: "fixture".to_owned(),
            arguments: vec!["fixture-pass".to_owned()],
            working_directory: ".".to_owned(),
            started_at: "2026-08-26T00:00:00Z".to_owned(),
            completed_at: "2026-08-26T00:00:01Z".to_owned(),
            exit_result: openwarrant_core::GateExitResult::ExitCode { code: 0 },
            selected_test_count: 1,
            selected_test_manifest: vec!["check://fixture-pass".to_owned()],
            selection_observation_ref: "docs/receipts/GR-LEGACY-1.selection.json".to_owned(),
            selection_observation_digest: content_digest(&selection_bytes),
            raw_evidence_refs: vec![],
            stdout_ref: "docs/receipts/GR-LEGACY-1.stdout.txt".to_owned(),
            stdout_digest: content_digest(stdout_bytes),
            stderr_ref: "docs/receipts/GR-LEGACY-1.stderr.txt".to_owned(),
            stderr_digest: content_digest(stderr_bytes),
            resource_usage: "fixture".to_owned(),
            verdict: openwarrant_core::Verdict::Pass,
            receipt_digest: String::new(),
            extensions: openwarrant_core::GateReceiptExtensions::default(),
        };
        gate_receipt.receipt_digest = format!(
            "sha256:{}",
            sha256_digest(DigestDomain::GateReceipt, &gate_receipt).expect("Gate receipt digest")
        );
        let receipt_bytes = serde_json::to_vec(&gate_receipt).expect("serialize Gate receipt");
        fs::write(receipts.join("GR-LEGACY-1.receipt.json"), &receipt_bytes)
            .expect("write Gate receipt");

        let snapshot = commit_snapshot_at(&root, Some("2026-08-26T01:00:00Z"));
        let bound = |path: &str, bytes: &[u8]| ContentBinding {
            reference: format!("ow:{path}"),
            sha256: content_digest(bytes),
            extensions: Default::default(),
        };
        let mut one = receipt();
        one.payload.source.source_sha256 = subject_digest.clone();
        one.receipt_digest = format!("sha256:{}", "f".repeat(64));
        one.payload.evidence = vec![LegacyAdrSupport::RequiredGatePass {
            run: bound("docs/receipts/GR-LEGACY-1.run.toml", &run_bytes),
            definition: ContentBinding {
                reference: format!(
                    "git:{}:docs/gates/fixture.legacy-pass@1.0.0.yaml",
                    snapshot.revision
                ),
                sha256: content_digest(definition_bytes),
                extensions: Default::default(),
            },
            binding: bound("docs/receipts/GR-LEGACY-1.binding.json", &binding_bytes),
            selection: Box::new(bound(
                "docs/receipts/GR-LEGACY-1.selection.json",
                &selection_bytes,
            )),
            stdout: Box::new(bound("docs/receipts/GR-LEGACY-1.stdout.txt", stdout_bytes)),
            stderr: Box::new(bound("docs/receipts/GR-LEGACY-1.stderr.txt", stderr_bytes)),
            fixtures: vec![],
            receipt: Box::new(bound(
                "docs/receipts/GR-LEGACY-1.receipt.json",
                &receipt_bytes,
            )),
            subject_source_sha256: subject_digest,
            extensions: Default::default(),
        }];
        one.validate_structure()
            .expect("complete RequiredGatePass support shape");

        verify_support(
            &root,
            &root,
            &one,
            &mut BTreeSet::new(),
            &mut CheckedRefs::new(),
            &snapshot,
            &mut SupportState::default(),
            &InternalArtifacts::default(),
            Some(ResolutionVerificationContext {
                repo: &repo,
                authority: &fixture_authority(),
                verification_as_of: "2026-08-30T00:00:00Z",
            }),
        )
        .expect("complete Gate receipt bundle admits through legacy support seam");
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[test]
    fn source_digest_is_plain_sha256_of_exact_bytes() {
        assert_eq!(
            content_digest(b"abc"),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn payload_mutation_invalidates_receipt_and_manifest_digests() {
        let mut one = receipt();
        one.receipt_digest = artifact_digest(&one.digest_preimage()).expect("receipt digest");
        let mut set = manifest(vec![one]);
        set.manifest_digest = artifact_digest(&set.digest_preimage()).expect("manifest digest");
        verify_receipt_digests(&set).expect("unaltered receipt");
        verify_manifest_digest(&set).expect("unaltered manifest");

        let mut receipt_mutation = set.clone();
        receipt_mutation.payload.receipts[0]
            .payload
            .reason
            .push_str(" changed");
        assert!(verify_receipt_digests(&receipt_mutation).is_err());

        let mut manifest_mutation = set;
        manifest_mutation.payload.sha_f_predecessor_ids[0] = "0002".to_owned();
        assert!(verify_manifest_digest(&manifest_mutation).is_err());
    }

    #[test]
    fn only_effective_human_judge_assignment_can_back_review() {
        let one = receipt();
        let assignment = |kind, effective: &str| RoleAssignment {
            actor: "QuiteTall".to_owned(),
            actor_kind: kind,
            roles: BTreeSet::from([ActorRole::Judge]),
            assigned_by: "human:owner".to_owned(),
            effective_time: effective.to_owned(),
            note: None,
        };

        let agent =
            AuthorityRegister::new(vec![assignment(ActorKind::Agent, "2026-08-25T00:00:00Z")]);
        assert!(
            verify_reviewer_authority(&agent, &one, "2026-08-25T00:00:00Z", "2026-08-30T00:00:00Z")
                .is_err()
        );

        let future =
            AuthorityRegister::new(vec![assignment(ActorKind::Human, "2026-08-27T00:00:00Z")]);
        assert!(
            verify_reviewer_authority(
                &future,
                &one,
                "2026-08-25T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .is_err()
        );

        let judge =
            AuthorityRegister::new(vec![assignment(ActorKind::Human, "2026-08-25T00:00:00Z")]);
        verify_reviewer_authority(&judge, &one, "2026-08-25T00:00:00Z", "2026-08-30T00:00:00Z")
            .expect("effective human Judge");
        let cutoff_error =
            verify_reviewer_authority(&judge, &one, "2026-08-25T00:00:00Z", "2026-08-26T00:00:00Z")
                .expect_err("review after verification cutoff must fail");
        assert!(
            cutoff_error
                .to_string()
                .contains("after verification cutoff"),
            "unexpected cutoff diagnostic: {cutoff_error}"
        );
        assert!(
            verify_reviewer_authority(
                &judge,
                &one,
                "2026-08-30T00:00:00Z",
                "2026-08-30T00:00:00Z",
            )
            .is_err(),
            "review cannot predate governing authorization"
        );
    }

    #[test]
    fn human_response_binds_exact_review_subject() {
        let root = scratch("review-response");
        let mut one = receipt();
        let response = review_response(&one);
        let snapshot = write_response_and_snapshot(&root, &mut one, &response);
        let mut state = ReviewState::default();
        verify_human_review(
            &human_judge_register(),
            &one,
            "2026-08-25T00:00:00Z",
            "2026-08-30T00:00:00Z",
            &root,
            &snapshot,
            &mut state,
        )
        .expect("bound human review");
        verify_review_coverage(&state).expect("every response judgment consumed");

        one.payload.reason.push_str(" changed after review");
        assert!(
            verify_human_review(
                &human_judge_register(),
                &one,
                "2026-08-25T00:00:00Z",
                "2026-08-30T00:00:00Z",
                &root,
                &snapshot,
                &mut ReviewState::default(),
            )
            .is_err(),
            "post-review subject mutation must fail"
        );
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[test]
    fn human_response_actor_and_judgment_are_single_use() {
        let root = scratch("review-single-use");
        let mut one = receipt();
        let mut response = review_response(&one);
        response.judgments[0].actor = "somebody-else".to_owned();
        let snapshot = write_response_and_snapshot(&root, &mut one, &response);
        assert!(
            verify_human_review(
                &human_judge_register(),
                &one,
                "2026-08-25T00:00:00Z",
                "2026-08-30T00:00:00Z",
                &root,
                &snapshot,
                &mut ReviewState::default(),
            )
            .is_err(),
            "response actor must match receipt reviewer"
        );
        fs::remove_dir_all(&root).expect("remove first scratch directory");

        let root = scratch("review-reuse");
        let mut one = receipt();
        let response = review_response(&one);
        let snapshot = write_response_and_snapshot(&root, &mut one, &response);
        let mut state = ReviewState::default();
        verify_human_review(
            &human_judge_register(),
            &one,
            "2026-08-25T00:00:00Z",
            "2026-08-30T00:00:00Z",
            &root,
            &snapshot,
            &mut state,
        )
        .expect("first use");
        assert!(
            verify_human_review(
                &human_judge_register(),
                &one,
                "2026-08-25T00:00:00Z",
                "2026-08-30T00:00:00Z",
                &root,
                &snapshot,
                &mut state,
            )
            .is_err(),
            "same judgment cannot back two receipts"
        );
        fs::remove_dir_all(root).expect("remove second scratch directory");
    }

    #[test]
    fn unused_response_judgment_fails_coverage() {
        let root = scratch("review-unused");
        let mut one = receipt();
        let mut response = review_response(&one);
        let mut extra = response.judgments[0].clone();
        extra.id = "judgment-0002".to_owned();
        extra.adr_id = "0002".to_owned();
        extra.review_subject_digest = format!("sha256:{}", "8".repeat(64));
        response.judgments.push(extra);
        let snapshot = write_response_and_snapshot(&root, &mut one, &response);
        let mut state = ReviewState::default();
        verify_human_review(
            &human_judge_register(),
            &one,
            "2026-08-25T00:00:00Z",
            "2026-08-30T00:00:00Z",
            &root,
            &snapshot,
            &mut state,
        )
        .expect("selected judgment is valid");
        assert!(verify_review_coverage(&state).is_err());
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[test]
    fn complete_receipt_cannot_use_its_source_as_only_evidence() {
        let mut one = receipt();
        one.payload.evidence = vec![LegacyAdrSupport::ImplementationArtifact {
            artifact: ContentBinding {
                reference: format!(
                    "git:{}:{}",
                    one.payload.source.commit_sha, one.payload.source.path
                ),
                sha256: one.payload.source.source_sha256.clone(),
                extensions: Default::default(),
            },
            extensions: Default::default(),
        }];
        assert!(
            verify_support(
                Utf8Path::new("/not-used"),
                Utf8Path::new("/not-used"),
                &one,
                &mut BTreeSet::new(),
                &mut CheckedRefs::new(),
                &fake_snapshot(Utf8Path::new("/not-used")),
                &mut SupportState::default(),
                &InternalArtifacts::default(),
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn administrative_artifacts_and_lineage_cannot_support_completion() {
        let mut one = receipt();
        let migration = match &one.payload.migration {
            ImportBinding::Migrated { artifact, .. } => artifact.clone(),
            ImportBinding::DeliberatelyExcluded { .. } => unreachable!("fixture is migrated"),
        };
        one.payload.evidence = vec![LegacyAdrSupport::ImplementationArtifact {
            artifact: migration.clone(),
            extensions: Default::default(),
        }];
        let mut internal = InternalArtifacts::default();
        internal.insert(&migration);
        assert!(
            verify_support(
                Utf8Path::new("/not-used"),
                Utf8Path::new("/not-used"),
                &one,
                &mut BTreeSet::new(),
                &mut CheckedRefs::new(),
                &fake_snapshot(Utf8Path::new("/not-used")),
                &mut SupportState::default(),
                &internal,
                None,
            )
            .is_err(),
            "migration artifact cannot prove its own disposition"
        );

        one.payload.evidence = vec![LegacyAdrSupport::HistoricalContext {
            artifact: ContentBinding {
                reference:
                    "git:1111111111111111111111111111111111111111:docs/decisions/0001-old.md"
                        .to_owned(),
                sha256: format!("sha256:{}", "7".repeat(64)),
                extensions: Default::default(),
            },
            extensions: Default::default(),
        }];
        assert!(
            one.validate_structure().is_err(),
            "lineage cannot be relabeled as disposition evidence"
        );
    }

    #[test]
    fn dot_path_alias_cannot_hide_source_as_evidence() {
        let mut one = receipt();
        one.payload.evidence = vec![LegacyAdrSupport::ImplementationArtifact {
            artifact: ContentBinding {
                reference: format!(
                    "git:{}:./{}",
                    one.payload.source.commit_sha, one.payload.source.path
                ),
                sha256: one.payload.source.source_sha256.clone(),
                extensions: Default::default(),
            },
            extensions: Default::default(),
        }];
        let error = verify_support(
            Utf8Path::new("/not-used"),
            Utf8Path::new("/not-used"),
            &one,
            &mut BTreeSet::new(),
            &mut CheckedRefs::new(),
            &fake_snapshot(Utf8Path::new("/not-used")),
            &mut SupportState::default(),
            &InternalArtifacts::default(),
            None,
        )
        .expect_err("dot path must fail before Git access");
        assert!(error.to_string().contains("not canonical"));
    }

    #[test]
    fn copied_source_bytes_are_not_independent_evidence() {
        let root = scratch("copied-evidence");
        let source = b"same source bytes\n";
        fs::write(root.join("copy.md"), source).expect("write copied evidence");
        let snapshot = commit_snapshot(&root);
        let mut one = receipt();
        one.payload.source.source_sha256 = content_digest(source);
        one.payload.evidence = vec![LegacyAdrSupport::ImplementationArtifact {
            artifact: ContentBinding {
                reference: format!("git:{}:copy.md", snapshot.revision),
                sha256: content_digest(source),
                extensions: Default::default(),
            },
            extensions: Default::default(),
        }];
        let error = verify_support(
            &root,
            &root,
            &one,
            &mut BTreeSet::new(),
            &mut CheckedRefs::new(),
            &snapshot,
            &mut SupportState::default(),
            &InternalArtifacts::default(),
            None,
        )
        .expect_err("copied source bytes are self-evidence");
        assert!(
            error
                .to_string()
                .contains("support is its own source or copied source bytes")
        );
        fs::remove_dir_all(root).expect("remove scratch directory");
    }

    #[test]
    fn source_binding_must_match_frozen_census() {
        let mut source = receipt().payload.source;
        source.commit_sha = SHA_F.to_owned();
        source.exact_ref = CONSOLIDATE_REF.to_owned();
        source.ref_tip_at_review = SHA_F.to_owned();
        verify_frozen_source_binding(&source, true).expect("frozen base source");

        let candidate = &OFF_BRANCH_SOURCES[0];
        source.adr_id = candidate.adr_id.to_owned();
        source.commit_sha = candidate.commit.to_owned();
        source.path = candidate.path.to_owned();
        source.source_sha256 = candidate.source_sha256.to_owned();
        source.exact_ref = candidate.refs[0].0.to_owned();
        source.ref_tip_at_review = candidate.refs[0].1.to_owned();
        verify_frozen_source_binding(&source, false).expect("frozen off-branch source");

        source.commit_sha = "0".repeat(40);
        assert!(verify_frozen_source_binding(&source, false).is_err());

        let adr_0187 = OFF_BRANCH_SOURCES
            .iter()
            .find(|candidate| candidate.adr_id == "0187")
            .expect("0187 census entry");
        source.adr_id = adr_0187.adr_id.to_owned();
        source.commit_sha = adr_0187.commit.to_owned();
        source.path = adr_0187.path.to_owned();
        source.source_sha256 = adr_0187.source_sha256.to_owned();
        source.exact_ref = CASCADE_REF.to_owned();
        source.ref_tip_at_review = CASCADE_TIP.to_owned();
        assert!(verify_frozen_source_binding(&source, false).is_err());
    }

    fn imported(id: &str) -> MigratedAdr {
        MigratedAdr {
            source: format!("{id}-fixture.md"),
            preserved_body: "body\n".to_owned(),
            preserved_body_digest: content_digest(b"body\n"),
            mapped_elements: BTreeMap::new(),
            unmapped_elements: vec![],
            legacy_gates: vec![],
            historical_claims: vec![],
        }
    }

    #[test]
    fn base_inventory_is_derived_from_git_not_import_self_report() {
        let mut one = receipt();
        one.receipt_digest = artifact_digest(&one.digest_preimage()).expect("receipt digest");
        let mut set = manifest(vec![one]);
        set.payload.sha_f_predecessor_ids = (1..=172).map(|n| format!("{n:04}")).collect();

        let mut imported_ids: Vec<String> = (1..=171).map(|n| format!("{n:04}")).collect();
        imported_ids.push("0185".to_owned());
        imported_ids.push(FINAL_ADR_ID.to_owned());
        let import = ImportArtifact {
            commit_sha: SHA_F.to_owned(),
            corpus_relative_root: "docs/decisions".to_owned(),
            adr_count: imported_ids.len(),
            promoted_resolutions: 0,
            historical_claims: 0,
            legacy_declared_unqualified_gates: 0,
            preservation_failures: vec![],
            unmapped_elements: BTreeMap::new(),
            adrs: imported_ids.iter().map(|id| imported(id)).collect(),
        };
        let mut committed_ids: Vec<String> = (1..=172).map(|n| format!("{n:04}")).collect();
        committed_ids.push(FINAL_ADR_ID.to_owned());
        let committed: Vec<(String, String)> = committed_ids
            .iter()
            .map(|id| {
                let name = format!("{id}-fixture.md");
                (name.clone(), format!("{SOURCE_CORPUS_ROOT}/{name}"))
            })
            .collect();

        assert_eq!(set.payload.sha_f_predecessor_ids.len(), 172);
        assert_eq!(import.adr_count, 173);
        assert!(verify_base_inventory(&set, &import, &committed).is_err());
    }

    #[test]
    fn imported_body_must_equal_exact_git_source_body() {
        let root = scratch("import-body");
        let mut one = receipt();
        let artifact_path = root.join("import.json");
        let record = MigratedAdr {
            source: "0001-x.md".to_owned(),
            preserved_body: "tampered\n".to_owned(),
            preserved_body_digest: content_digest(b"tampered\n"),
            mapped_elements: BTreeMap::new(),
            unmapped_elements: vec![],
            legacy_gates: vec![],
            historical_claims: vec![],
        };
        let import = ImportArtifact {
            commit_sha: one.payload.source.commit_sha.clone(),
            corpus_relative_root: SOURCE_CORPUS_ROOT.to_owned(),
            adr_count: 1,
            promoted_resolutions: 0,
            historical_claims: 0,
            legacy_declared_unqualified_gates: 0,
            preservation_failures: vec![],
            unmapped_elements: BTreeMap::new(),
            adrs: vec![record],
        };
        let bytes = serde_json::to_vec(&import).expect("serialize import");
        fs::write(&artifact_path, &bytes).expect("write import");
        one.payload.migration = ImportBinding::Migrated {
            artifact: ContentBinding {
                reference: "import.json".to_owned(),
                sha256: content_digest(&bytes),
                extensions: Default::default(),
            },
            imported_body_sha256: content_digest(b"tampered\n"),
            extensions: Default::default(),
        };

        let error = verify_migrated_body(
            "0001",
            &import.adrs[0],
            b"---\ntitle: x\n---\nbody\n",
            "fixture import",
        )
        .expect_err("self-consistent import must not override exact Git body");
        assert!(error.to_string().contains("exact Git source body"));
        fs::remove_dir_all(root).expect("remove scratch directory");
    }
}
