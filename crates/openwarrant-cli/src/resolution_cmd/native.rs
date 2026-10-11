// SPDX-License-Identifier: Apache-2.0
//! Native acceptance: exact human-signed decision and fresh protected assessment.
//! No legacy supplementation, saved-verdict callback or register fallback.
use super::*;
use crate::runtime_capture::collector_signature::OpenSshSignatureCheck;
use crate::runtime_capture::{Fault, Selection, native_host::NativeAssessment};
use openwarrant_core::runtime_collector::Fault as CryptoFault;
use std::time::Duration;

pub mod signing;

pub const RESPONSE_SCHEMA: &str = "oh.war/native-resolution-response/v2-draft.1";
pub const RECORD_SCHEMA: &str = "oh.war/resolution/v2-draft.1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub schema: String,
    pub warrant: String,
    pub warrant_uuid: String,
    pub resolution_id: String,
    pub resolved_by: String,
    pub acting_role: String,
    pub common_outcome: CommonOutcome,
    pub profile_outcome: String,
    pub meaning: String,
    pub effective_time: String,
    pub contract_revision: u32,
    pub contract_digest: String,
    pub assurance_case_snapshot_digest: String,
    pub artifact_manifest_digest: String,
    pub gate_run_refs: Vec<String>,
    pub judgment_refs: Vec<String>,
    pub residual_risk_refs: Vec<String>,
    pub selections: Vec<Selection>,
    pub policy: crate::runtime_capture::collector_loading::NativePolicyBinding,
    pub require_user_presence: bool,
    #[serde(deserialize_with = "strict_locator")]
    pub locator: Locator,
}

fn strict_locator<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Locator, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Exact {
        commit_sha: String,
        worktree_clean: bool,
        #[serde(default)]
        paths_dirty: Vec<String>,
    }
    let exact = Exact::deserialize(decoder)?;
    Ok(Locator {
        commit_sha: exact.commit_sha,
        worktree_clean: exact.worktree_clean,
        paths_dirty: exact.paths_dirty,
    })
}

fn fault(code: &'static str, message: impl ToString, unknown: bool) -> Fault {
    Fault {
        code,
        message: message.to_string(),
        unknown,
    }
}
fn source(error: RepoError) -> Fault {
    fault("native-resolution.context-unavailable", error, true)
}
fn crypto(error: CryptoFault) -> Fault {
    match error {
        CryptoFault::Unavailable(why) => {
            fault("native-resolution.signature-unavailable", why, true)
        }
        CryptoFault::Rejected(why) => fault("native-resolution.signature-refused", why, false),
    }
}

pub fn parse(bytes: &[u8]) -> Result<Response, Fault> {
    if bytes.len() > 65_536 {
        return Err(fault(
            "native-resolution.budget",
            "response exceeds 64 KiB",
            false,
        ));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|e| fault("native-resolution.syntax", e, false))?;
    // Discriminate before decoding: a future shape is not a malformed v2.
    let value: toml::Value =
        toml::from_str(text).map_err(|e| fault("native-resolution.syntax", e, false))?;
    if value.get("schema").and_then(toml::Value::as_str) != Some(RESPONSE_SCHEMA) {
        return Err(fault(
            "native-resolution.schema-unavailable",
            "native response schema unsupported",
            true,
        ));
    }
    let response: Response =
        toml::from_str(text).map_err(|e| fault("native-resolution.syntax", e, false))?;
    if response.acting_role != "resolver" || response.selections.len() > 256 {
        return Err(fault(
            "native-resolution.shape",
            "resolver role and at most 256 selections required",
            false,
        ));
    }
    if response.policy.schema != "oh.war/runtime-native-policy-binding/v1-draft.1" {
        return Err(fault(
            "native-resolution.policy-schema-unavailable",
            "native policy binding schema unsupported",
            true,
        ));
    }
    response
        .resolution_id
        .parse::<WarUuid>()
        .map_err(|e| fault("native-resolution.id", e, false))?;
    response
        .warrant_uuid
        .parse::<WarUuid>()
        .map_err(|e| fault("native-resolution.warrant-id", e, false))?;
    openwarrant_core::timestamp::validate_rfc3339_utc(&response.effective_time)
        .map_err(|e| fault("native-resolution.effective-time", e, false))?;
    if response.meaning.trim().is_empty()
        || !response.locator.worktree_clean
        || !response.locator.paths_dirty.is_empty()
        || response.locator.commit_sha.len() != 40
        || !response
            .locator
            .commit_sha
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(fault(
            "native-resolution.shape",
            "meaning and clean exact Git locator required",
            false,
        ));
    }
    Ok(response)
}

fn store(repo: &Repository, actor: &str) -> Result<crate::authority_check::StoreBinding, Fault> {
    match crate::authority_check::binding(repo) {
        crate::authority_check::Binding::Store(store) if !store.test_mode => {
            store
                .signer(actor, &[crate::authority_check::Act::Resolve])
                .map_err(|v| {
                    fault(
                        v.rule(),
                        v.why(),
                        matches!(v, crate::authority_check::Verdict::Unavailable { .. }),
                    )
                })?;
            if actor == repo.performer() {
                return Err(fault(
                    "native-resolution.self-completion",
                    "performer cannot accept its own work",
                    false,
                ));
            }
            Ok(store)
        }
        crate::authority_check::Binding::Failed { why, .. } => {
            Err(fault("native-resolution.authority-unavailable", why, true))
        }
        _ => Err(fault(
            "native-resolution.protected-authority-required",
            "normal protected authority required; no register or test-store fallback",
            false,
        )),
    }
}

/// Reuse outcome/meaning choices only; every evidence/policy field is regenerated.
pub fn prepare(
    repo: &Repository,
    alias: &str,
    intent: &ResolutionResponse,
    selections: &[Selection],
    native: &NativeAssessment,
) -> Result<Response, Fault> {
    if intent.warrant != alias || intent.acting_role != "resolver" {
        return Err(fault(
            "native-resolution.subject",
            "exact subject and resolver role required",
            false,
        ));
    }
    let policy = native.policy_binding()?;
    let authority = store(repo, &intent.resolved_by)?;
    if authority.head != policy.authority_digest {
        return Err(fault(
            "native-resolution.authority-mismatch",
            "signer and native collector must use the same authority revision",
            false,
        ));
    }
    let dir = repo.warrant_dir(alias).map_err(source)?;
    let one = repo.load_warrant(&dir).map_err(source)?;
    one.require(&repo.profiles, openwarrant_core::Capability::Resolution)
        .map_err(source)?;
    let bound = bind(alias, &one, None).map_err(source)?;
    let evidence = crate::evidence::load(repo, &dir).map_err(source)?;
    let provider =
        |dispatch: &openwarrant_core::execution::StageDispatch,
         interface: &openwarrant_core::document::runtime::ProviderInterface| {
            native.resolve(dispatch, interface)
        };
    let assessment = crate::resolve::assess_with_runtime(
        repo,
        &one,
        &evidence,
        crate::resolve::runtime::Input {
            selections,
            native: &provider,
        },
    )
    .map_err(source)?;
    let checks = assessment.checks;
    let unmet = checks.unmet();
    if !unmet.is_empty() {
        return Err(fault(
            "native-resolution.requirements-unmet",
            unmet.join("; "),
            true,
        ));
    }
    let req =
        request_from_assessment(repo, alias, &one, bound.clone(), assessment).map_err(source)?;
    if !req.eligible_resolvers.contains(&intent.resolved_by) {
        return Err(fault(
            "native-resolution.resolver-not-eligible",
            "resolver is not currently eligible",
            false,
        ));
    }
    match crate::authorize::assignment_standing(repo, &dir)
        .map_err(source)?
        .for_act(crate::authorize::assignment::Act::Resolve)
    {
        Ok(None) => {}
        Ok(Some(assigned)) if assigned.contains(&intent.resolved_by) => {}
        _ => {
            return Err(fault(
                "native-resolution.assignment",
                "signed resolver assignment unavailable or does not permit this actor",
                false,
            ));
        }
    }
    if intent.common_outcome == CommonOutcome::Satisfied
        && req.would_resolve_satisfied != Some(true)
    {
        return Err(fault(
            "native-resolution.outcome-unsupported",
            "independently established obligations required for satisfied",
            false,
        ));
    }
    let paths = repo
        .load_deliverables(&dir)
        .map_err(source)?
        .records
        .iter()
        .map(|d| d.target_ref.clone())
        .collect::<Vec<_>>();
    let locator = locate(&repo.root, &paths)
        .filter(|l| l.worktree_clean)
        .ok_or_else(|| {
            fault(
                "native-resolution.locator-unavailable",
                "exact clean committed deliverables required",
                true,
            )
        })?;
    let response = Response {
        schema: RESPONSE_SCHEMA.into(),
        warrant: alias.into(),
        warrant_uuid: one
            .validated
            .as_ref()
            .expect("bound validated manifest")
            .uuid
            .to_string(),
        resolution_id: WarUuid::mint().to_string(),
        resolved_by: intent.resolved_by.clone(),
        acting_role: "resolver".into(),
        common_outcome: intent.common_outcome,
        profile_outcome: intent.profile_outcome.clone(),
        meaning: intent.meaning.clone(),
        effective_time: intent.effective_time.clone(),
        contract_revision: bound.contract_revision,
        contract_digest: bound.contract_digest.clone(),
        assurance_case_snapshot_digest: bound.assurance_case_snapshot_digest.clone(),
        artifact_manifest_digest: bound.artifact_manifest_digest.clone(),
        gate_run_refs: req.gate_run_refs,
        judgment_refs: req.judgment_refs,
        residual_risk_refs: req.residual_risk_refs,
        selections: selections.to_vec(),
        policy,
        require_user_presence: authority
            .revision
            .policy
            .as_ref()
            .is_none_or(|p| p.require_user_presence),
        locator,
    };
    let rendered =
        toml::to_string(&response).map_err(|e| fault("native-resolution.render", e, false))?;
    parse(rendered.as_bytes())?;
    let falsifiable = one
        .validated
        .as_ref()
        .is_some_and(|v| repo.profiles.kind(&v.profile).falsifiable_claims);
    response
        .resolution(&crate::gate_cmd::receipt::now_rfc3339_public())
        .validate(checks, falsifiable)
        .map_err(|e| fault("native-resolution.invalid", e, false))?;
    let current = repo.load_warrant(&dir).map_err(source)?;
    let current = bind(alias, &current, None).map_err(source)?;
    if bound.contract_digest != current.contract_digest
        || bound.assurance_case_snapshot_digest != current.assurance_case_snapshot_digest
        || bound.artifact_manifest_digest != current.artifact_manifest_digest
        || native.policy_binding()? != response.policy
        || store(repo, &intent.resolved_by)?.head != authority.head
    {
        return Err(fault(
            "native-resolution.context-moved",
            "sources or policy changed during assessment",
            false,
        ));
    }
    Ok(response)
}

impl Response {
    fn intent(&self) -> ResolutionResponse {
        ResolutionResponse {
            schema: super::RESPONSE_SCHEMA.into(),
            warrant: self.warrant.clone(),
            contract_digest: self.contract_digest.clone(),
            resolved_by: self.resolved_by.clone(),
            acting_role: self.acting_role.clone(),
            common_outcome: self.common_outcome,
            profile_outcome: self.profile_outcome.clone(),
            meaning: self.meaning.clone(),
            effective_time: self.effective_time.clone(),
        }
    }
    fn resolution(&self, recorded_at: &str) -> Resolution {
        Resolution {
            id: self.resolution_id.clone(),
            common_outcome: self.common_outcome,
            profile_outcome: self.profile_outcome.clone(),
            contract_revision: self.contract_revision,
            contract_digest: self.contract_digest.clone(),
            assurance_case_snapshot_digest: self.assurance_case_snapshot_digest.clone(),
            artifact_manifest_digest: self.artifact_manifest_digest.clone(),
            gate_run_refs: self.gate_run_refs.clone(),
            judgment_refs: self.judgment_refs.clone(),
            residual_risk_refs: self.residual_risk_refs.clone(),
            resolved_by_ref: format!("person://{}", self.resolved_by),
            acting_role_ref: format!(
                "role-assignment://{}/{}",
                self.resolved_by, self.acting_role
            ),
            meaning: self.meaning.clone(),
            effective_at: self.effective_time.clone(),
            recorded_at: recorded_at.into(),
            standing: openwarrant_core::ResolutionStanding::Valid,
        }
    }
}

/// The signature verifies the exact bytes parsed, never a path reopened after parsing.
pub fn verify_signature(
    repo: &Repository,
    bytes: &[u8],
    signature: &str,
) -> Result<Response, Fault> {
    verify_signature_details(repo, bytes, signature).map(|(response, _)| response)
}

fn verify_signature_details(
    repo: &Repository,
    bytes: &[u8],
    signature: &str,
) -> Result<(Response, String), Fault> {
    let response = parse(bytes)?;
    let authority = store(repo, &response.resolved_by)?;
    let (principal_id, principal) = authority
        .revision
        .principal_for_actor(&response.resolved_by)
        .ok_or_else(|| {
            fault(
                "native-resolution.actor",
                "protected actor unavailable",
                false,
            )
        })?;
    let check = OpenSshSignatureCheck::new(repo.root.as_std_path().into(), Duration::from_secs(10))
        .map_err(crypto)?;
    let observed = check
        .verify_response(&principal.public_key, bytes, signature)
        .map_err(crypto)?;
    if !observed.valid || (response.require_user_presence && observed.user_present != Some(true)) {
        return Err(fault(
            "native-resolution.signature-refused",
            "cryptographic signature or required authenticated presence not established",
            false,
        ));
    }
    if store(repo, &response.resolved_by)?.head != authority.head {
        return Err(fault(
            "native-resolution.authority-moved",
            "authority changed during signature verification",
            false,
        ));
    }
    Ok((response, principal_id.to_string()))
}

pub fn matches_record(bytes: &[u8], record: &ResolutionRecord) -> bool {
    let Ok(response) = parse(bytes) else {
        return false;
    };
    record.schema == RECORD_SCHEMA
        && record.warrant == response.warrant
        && record
            .native_response
            .as_deref()
            .is_some_and(|raw| raw.as_bytes() == bytes)
        && matches!((serde_json::to_value(&record.resolution), serde_json::to_value(response.resolution(&record.resolution.recorded_at))), (Ok(a), Ok(b)) if a == b)
        && matches!((serde_json::to_value(&record.locator), serde_json::to_value(Some(&response.locator))), (Ok(a), Ok(b)) if a == b)
        && openwarrant_core::timestamp::validate_rfc3339_utc(&record.resolution.recorded_at).is_ok()
}

/// Historical signed decision, not a fresh execution verdict or policy grant.
pub fn signature_for_record(
    repo: &Repository,
    record: &ResolutionRecord,
) -> crate::authority_check::Verdict {
    use crate::authority_check::Verdict;
    let (Some(raw), Some(signature)) = (&record.native_response, &record.native_signature) else {
        return Verdict::Unsigned {
            why: "native response and signature required".into(),
        };
    };
    if !matches_record(raw.as_bytes(), record) {
        return Verdict::Invalid {
            why: "native signed decision does not match record".into(),
        };
    }
    match verify_signature_details(repo, raw.as_bytes(), signature) {
        Ok((_, principal)) => Verdict::Signed {
            principal,
            response: format!("{}: embedded exact native response", record.warrant),
        },
        Err(f) if f.unknown => Verdict::Unavailable { why: f.message },
        Err(f) => Verdict::Invalid { why: f.message },
    }
}

fn recheck(repo: &Repository, response: &Response, native: &NativeAssessment) -> Result<(), Fault> {
    let mut fresh = prepare(
        repo,
        &response.warrant,
        &response.intent(),
        &response.selections,
        native,
    )?;
    fresh.resolution_id = response.resolution_id.clone();
    if !matches!((serde_json::to_value(&fresh), serde_json::to_value(response)), (Ok(a), Ok(b)) if a == b)
    {
        return Err(fault(
            "native-resolution.stale-response",
            "fresh evidence, snapshots, policy, actor or locator differs from signed decision",
            false,
        ));
    }
    Ok(())
}

/// Bounded CLI transport. The explicit selection must match the signed decision;
/// signature verification uses the original bytes, never a reserialized document.
pub fn ingest_selected_from_path(
    repo: &Repository,
    alias: &str,
    path: &Utf8Path,
    selections: &[Selection],
    native: &NativeAssessment,
    mode: crate::sign::IngestMode,
) -> Result<Report, Fault> {
    let bytes = crate::runtime_capture::read(repo, path, 65_536)?;
    let response = parse(&bytes)?;
    if !matches!((serde_json::to_value(&response.selections), serde_json::to_value(selections)), (Ok(a), Ok(b)) if a == b)
    {
        return Err(fault(
            "native-resolution.selection-mismatch",
            "explicit capture selection differs from signed decision",
            false,
        ));
    }
    let signature_path = camino::Utf8PathBuf::from(format!("{path}.sig"));
    let signature = crate::runtime_capture::read(repo, &signature_path, 16_384)?;
    let signature = std::str::from_utf8(&signature)
        .map_err(|e| fault("native-resolution.signature-syntax", e, false))?;
    ingest(repo, alias, &bytes, signature, native, mode)
}

/// Exact signed ingestion. No existing-record or signature-supplement exception.
pub fn ingest(
    repo: &Repository,
    alias: &str,
    bytes: &[u8],
    signature: &str,
    native: &NativeAssessment,
    mode: crate::sign::IngestMode,
) -> Result<Report, Fault> {
    let response = verify_signature(repo, bytes, signature)?;
    if response.warrant != alias {
        return Err(fault(
            "native-resolution.subject",
            "signed response names another Warrant",
            false,
        ));
    }
    let dir = repo.warrant_dir(alias).map_err(source)?;
    let out = dir.join("resolution.toml");
    let before = crate::compile::atomic::prestate(&out).map_err(|e| {
        fault(
            "native-resolution.record-prestate",
            e.diagnostic().message,
            false,
        )
    })?;
    if repo.load_resolution(&dir).map_err(source)?.is_some() {
        return Err(fault(
            "native-resolution.exists",
            "resolution already exists; native ingestion cannot replace or supplement it",
            false,
        ));
    }
    recheck(repo, &response, native)?;
    let record = ResolutionRecord {
        schema: RECORD_SCHEMA.into(),
        warrant: alias.into(),
        resolution: response.resolution(&crate::gate_cmd::receipt::now_rfc3339_public()),
        locator: Some(response.locator.clone()),
        native_response: Some(std::str::from_utf8(bytes).expect("parsed UTF8").into()),
        native_signature: Some(signature.into()),
    };
    let body =
        toml::to_string_pretty(&record).map_err(|e| fault("native-resolution.render", e, false))?;
    // Neither source checks nor target CAS are an atomic fence over all inputs.
    recheck(repo, &response, native)?;
    let mut report = Report::default();
    if mode == crate::sign::IngestMode::DryRun {
        report.push(Diagnostic::pass(
            "native-resolution.would-record",
            format!("{alias}: fresh checks and exact human signature hold; no write"),
        ));
        return Ok(report);
    }
    crate::compile::atomic::write_if(&out, body, &before).map_err(|e| {
        fault(
            "native-resolution.record-prestate",
            e.diagnostic().message,
            false,
        )
    })?;
    let details = serde_json::json!({"common_outcome": response.common_outcome,
        "contract_digest": response.contract_digest, "native_response_digest": format!("sha256:{}", sha256_hex(bytes))});
    crate::journal_cmd::record(
        &dir,
        &response.warrant_uuid,
        crate::journal_cmd::RESOLUTION_RECORDED,
        &format!("person://{}", response.resolved_by),
        &details.to_string(),
    )
    .map_err(source)?;
    report.push(Diagnostic::pass(
        "native-resolution.recorded",
        format!(
            "{alias}: exact native human acceptance → {}",
            repo.relative(&out)
        ),
    ));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Response {
        // Shape-only fixture, not an observation, authorization or acceptance.
        let digest = format!("sha256:{}", "a".repeat(64));
        Response {
            schema: RESPONSE_SCHEMA.into(),
            warrant: "FX-WAR-0001".into(),
            warrant_uuid: "01a12897-783e-71c5-a729-22bc7d21adfa".into(),
            resolution_id: "01a12897-783e-71c5-a729-22bc7d21adfb".into(),
            resolved_by: "fixture-actor".into(),
            acting_role: "resolver".into(),
            common_outcome: CommonOutcome::Cancelled,
            profile_outcome: "cancelled".into(),
            meaning: "Synthetic cancellation".into(),
            effective_time: "2026-10-10T00:00:00Z".into(),
            contract_revision: 1,
            contract_digest: digest.clone(),
            assurance_case_snapshot_digest: digest.clone(),
            artifact_manifest_digest: digest.clone(),
            gate_run_refs: vec!["gate/fixture.toml".into()],
            judgment_refs: vec!["judgments/fixture.toml".into()],
            residual_risk_refs: vec![],
            selections: vec![Selection {
                stage_id: "ST-001".into(),
                capture_digest: digest.clone(),
            }],
            policy: crate::runtime_capture::collector_loading::NativePolicyBinding {
                schema: "oh.war/runtime-native-policy-binding/v1-draft.1".into(),
                repository: "fixture-repository".into(),
                collector: "fixture-collector".into(),
                authority_digest: digest.clone(),
                active_enrollment_digest: digest.clone(),
                host_configuration_digest: digest.clone(),
                native_configuration_digest: digest,
            },
            require_user_presence: true,
            locator: Locator {
                commit_sha: "a".repeat(40),
                worktree_clean: true,
                paths_dirty: vec![],
            },
        }
    }
    fn raw(response: &Response) -> String {
        toml::to_string(response).unwrap()
    }
    fn record(response: &Response) -> ResolutionRecord {
        ResolutionRecord {
            schema: RECORD_SCHEMA.into(),
            warrant: response.warrant.clone(),
            resolution: response.resolution("2026-10-10T01:00:00Z"),
            locator: Some(response.locator.clone()),
            native_response: Some(raw(response)),
            native_signature: Some("not a signature: shape-only fixture".into()),
        }
    }
    #[test]
    fn known_shapes_are_strict_and_future_shapes_are_unknown() {
        let baseline = fixture();
        let text = raw(&baseline);
        assert!(parse(text.as_bytes()).is_ok());
        let mut future = baseline.clone();
        future.schema = "oh.war/native-resolution-response/v999".into();
        assert!(parse(raw(&future).as_bytes()).unwrap_err().unknown);
        future = baseline.clone();
        future.policy.schema = "future-policy".into();
        assert!(parse(raw(&future).as_bytes()).unwrap_err().unknown);
        let extra = format!("invented = true\n{text}");
        assert!(!parse(extra.as_bytes()).unwrap_err().unknown);
        let mut extra: toml::Value = toml::from_str(&text).unwrap();
        extra["locator"]
            .as_table_mut()
            .unwrap()
            .insert("invented".into(), true.into());
        assert!(
            !parse(toml::to_string(&extra).unwrap().as_bytes())
                .unwrap_err()
                .unknown
        );
        assert!(!parse(&vec![b' '; 65_537]).unwrap_err().unknown);
        for text in [b"\xff".as_slice(), b"not toml".as_slice()] {
            assert!(!parse(text).unwrap_err().unknown);
        }
        for changed in [
            Response {
                warrant_uuid: "not a UUID".into(),
                ..baseline.clone()
            },
            Response {
                resolution_id: "not a UUID".into(),
                ..baseline.clone()
            },
            Response {
                effective_time: "yesterday".into(),
                ..baseline.clone()
            },
            Response {
                acting_role: "performer".into(),
                ..baseline.clone()
            },
            Response {
                meaning: " ".into(),
                ..baseline.clone()
            },
            Response {
                selections: vec![baseline.selections[0].clone(); 257],
                ..baseline.clone()
            },
            Response {
                locator: Locator {
                    commit_sha: "A".repeat(40),
                    ..baseline.locator.clone()
                },
                ..baseline.clone()
            },
            Response {
                locator: Locator {
                    worktree_clean: false,
                    ..baseline.locator.clone()
                },
                ..baseline.clone()
            },
            Response {
                locator: Locator {
                    paths_dirty: vec!["dirty".into()],
                    ..baseline.locator.clone()
                },
                ..baseline.clone()
            },
        ] {
            assert!(!parse(raw(&changed).as_bytes()).unwrap_err().unknown);
        }
    }
    #[test]
    fn record_cannot_substitute_any_signed_decision_field() {
        let response = fixture();
        let bytes = raw(&response);
        let baseline = record(&response);
        assert!(matches_record(bytes.as_bytes(), &baseline));
        let core = serde_json::to_value(&baseline.resolution).unwrap();
        for (field, value) in core.as_object().unwrap() {
            if field == "recorded_at" {
                continue;
            } // Service metadata, explicitly unsigned.
            let mut modified = core.clone();
            modified[field] = match value {
                serde_json::Value::String(_) => serde_json::json!("substituted"),
                serde_json::Value::Number(_) => serde_json::json!(99),
                serde_json::Value::Array(_) => serde_json::json!(["substituted"]),
                other => panic!("uncovered field {field}: {other}"),
            };
            if let Ok(resolution) = serde_json::from_value(modified) {
                let changed = ResolutionRecord {
                    resolution,
                    ..baseline.clone()
                };
                assert!(!matches_record(bytes.as_bytes(), &changed), "{field}");
            }
        }
        let changed = ResolutionRecord {
            locator: Some(Locator {
                commit_sha: "b".repeat(40),
                ..response.locator.clone()
            }),
            ..baseline.clone()
        };
        assert!(!matches_record(bytes.as_bytes(), &changed));
        let changed = ResolutionRecord {
            warrant: "FX-WAR-0002".into(),
            ..baseline.clone()
        };
        assert!(!matches_record(bytes.as_bytes(), &changed));
        let changed = ResolutionRecord {
            schema: super::super::RECORD_SCHEMA.into(),
            ..baseline.clone()
        };
        assert!(!matches_record(bytes.as_bytes(), &changed));
        let changed = ResolutionRecord {
            native_response: Some(format!("{bytes}\n")),
            ..baseline.clone()
        };
        assert!(!matches_record(bytes.as_bytes(), &changed));
        let mut changed = baseline;
        changed.resolution.recorded_at = "2026-10-11T00:00:00Z".into();
        assert!(matches_record(bytes.as_bytes(), &changed));
        changed.resolution.recorded_at = "invented time".into();
        assert!(!matches_record(bytes.as_bytes(), &changed));
    }

    #[test]
    fn record_loader_refuses_hybrids_and_unsigned_register_cannot_grant_native_acceptance() {
        let name = format!(
            "ow-native-record-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir().join(name)).unwrap();
        std::fs::create_dir(&root).unwrap();
        struct Cleanup(camino::Utf8PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(root.clone());
        std::fs::write(root.join("openwarrant.toml"), "schema = \"oh.war/repository-config/v1\"\n[project]\nname = \"native-record-fixture\"\nnamespace = \"FX\"\n[paths]\n").unwrap();
        let repo = Repository::open(root.clone()).unwrap();
        let directory = root.join("docs/warrants/FX-WAR-0001");
        std::fs::create_dir_all(&directory).unwrap();
        let response = fixture();
        let baseline = record(&response);
        let path = directory.join("resolution.toml");
        std::fs::write(&path, toml::to_string(&baseline).unwrap()).unwrap();
        // Structural loading is not signature validation or qualification.
        assert!(repo.load_resolution(&directory).unwrap().is_some());
        let refused =
            verify_signature(&repo, raw(&response).as_bytes(), "not a signature").unwrap_err();
        assert_eq!(
            refused.code,
            "native-resolution.protected-authority-required"
        );
        let changed = ResolutionRecord {
            schema: super::super::RECORD_SCHEMA.into(),
            ..baseline.clone()
        };
        std::fs::write(&path, toml::to_string(&changed).unwrap()).unwrap();
        assert!(repo.load_resolution(&directory).is_err());
        let changed = ResolutionRecord {
            native_signature: None,
            ..baseline.clone()
        };
        std::fs::write(&path, toml::to_string(&changed).unwrap()).unwrap();
        assert!(repo.load_resolution(&directory).is_err());
        std::fs::write(&path, "schema = \"future-resolution\"\ninvented = true\n").unwrap();
        assert!(matches!(
            repo.load_resolution(&directory),
            Err(RepoError::ObservationUnavailable { .. })
        ));
        let mut report = Report::default();
        super::super::check(
            &repo,
            &directory,
            "FX-WAR-0001",
            Some(&response.contract_digest),
            &mut report,
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.rule == "resolution.schema-unavailable")
        );
    }
}
