// SPDX-License-Identifier: Apache-2.0
//! Candidate RC.3 codecs and pure checks for supplied workflow facts.
//! Decoding preserves claims. It does not establish tracker origin or authority.
use super::*;

pub const SCHEMA: &str = "oh.war/workflow-record/1.0.0-rc.3";
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowEnvelope {
    pub schema: String,
    pub id: String,
    pub payload: WorkflowPayload,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "kebab-case",
    deny_unknown_fields
)]
pub enum WorkflowPayload {
    ContextView(ContextView),
    SharedContract(SharedContract),
    EvidenceAvailability(EvidenceAvailability),
    Stop(Stop),
    WorkChange(WorkChange),
    Overview(Overview),
    Handoff(Handoff),
    ReviewManifest(ReviewManifest),
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkState {
    Pending,
    InProgress,
    Complete,
    Blocked,
    Failed,
    Cancelled,
    Unknown,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QualificationClaim {
    Unverified,
    Unknown,
    Verified,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Maturity {
    Draft,
    Proposed,
    Accepted,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContractOrigin {
    CodeDerived,
    ApprovedDocument,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContextView {
    pub source_digest: String,
    pub document_maturity: Maturity,
    pub work_state: WorkState,
    pub qualification: QualificationClaim,
    pub contract_origin: ContractOrigin,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Participant {
    pub project: String,
    pub scope: String,
    pub basis_digest: String,
    pub contract_digest: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedContract {
    pub contract_digest: String,
    pub participants: Vec<Participant>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Present,
    Absent,
    Deleted,
    Restored,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceAvailability {
    pub evidence: ExactReference,
    pub state: Availability,
    pub retained_digests: Vec<String>,
    pub history_refs: Vec<String>,
    pub affected_assurance_refs: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopClass {
    Work,
    Harness,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopScope {
    Feature,
    Integration,
    Warrant,
    Program,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkerState {
    Running,
    StopRequested,
    Stopped,
    Unknown,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StopCause {
    Completed,
    TurnEnd,
    UserInterruption,
    TokenBudget,
    Timeout,
    ToolFailure,
    ProcessExit,
    PermissionRevoked,
    DependencyBlocked,
    ResourceLimit,
    DecisionNeeded,
    ConnectionLost,
    Cancelled,
    Unknown,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stop {
    pub subject: Subject,
    pub class: StopClass,
    pub scope: StopScope,
    pub scope_id: String,
    pub parent_scope: Option<String>,
    pub cause: StopCause,
    pub work_state: WorkState,
    pub worker_state: WorkerState,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeClass {
    Work,
    Harness,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApplyAt {
    StopNow,
    NextWorkStop,
    NextToolCall,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkChange {
    pub class: ChangeClass,
    pub old_basis: String,
    pub new_basis: String,
    pub scope_id: String,
    pub apply_at: ApplyAt,
    pub choice_ref: String,
    pub fence_ref: String,
    pub context_ref: String,
    pub limits: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletionRef {
    pub event_id: String,
    pub subject: Subject,
    pub scope_id: String,
    pub notes: String,
    pub document_trail: Vec<String>,
    pub next_steps: Vec<String>,
    pub qualification: QualificationClaim,
    pub assurance_refs: Vec<ExactReference>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overview {
    pub tracker: String,
    pub revision: String,
    pub as_of_unix_ms: u64,
    pub completions: Vec<CompletionRef>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handoff {
    pub event_id: String,
    pub overview: ExactReference,
    pub overview_url: String,
    pub safeword: String,
    pub notes: String,
    pub document_trail: Vec<String>,
    pub next_steps: Vec<String>,
    pub qualification: QualificationClaim,
    pub assurance_refs: Vec<ExactReference>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewMember {
    pub subject: Subject,
    pub profile: ExactReference,
    pub evidence: Vec<ExactReference>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewManifest {
    pub members: Vec<ReviewMember>,
}
#[derive(Debug)]
pub struct CheckedWorkflowRecord {
    envelope: WorkflowEnvelope,
    raw_digest: String,
}
impl CheckedWorkflowRecord {
    pub fn envelope(&self) -> &WorkflowEnvelope {
        &self.envelope
    }
    pub fn raw_digest(&self) -> &str {
        &self.raw_digest
    }
}
pub fn decode_workflow_record(
    bytes: &[u8],
    limits: RecordLimits,
) -> Result<CheckedWorkflowRecord, RecordError> {
    if limits.bytes == 0
        || limits.records == 0
        || limits.references == 0
        || bytes.len() > limits.bytes
    {
        return Err(error("resource-limit", "Workflow input limit"));
    }
    let unique: UniqueJson =
        serde_json::from_slice(bytes).map_err(|e| error("workflow.syntax", e.to_string()))?;
    let envelope: WorkflowEnvelope = strict_object_decode(unique.0, "workflow.syntax")?;
    if envelope.schema != SCHEMA || !identity(&envelope.id) {
        return Err(error(
            "workflow.identity",
            "Unsupported schema or invalid ID",
        ));
    }
    let mut remaining = limits.references;
    let mut charge = |n: usize| -> Result<(), RecordError> {
        remaining = remaining
            .checked_sub(n)
            .ok_or_else(|| error("resource-limit", "Workflow reference count"))?;
        Ok(())
    };
    let valid = match &envelope.payload {
        WorkflowPayload::ContextView(v) => {
            charge(1)?;
            digest(&v.source_digest)
        }
        WorkflowPayload::SharedContract(v) => {
            charge(v.participants.len())?;
            let mut ids = BTreeSet::new();
            digest(&v.contract_digest)
                && !v.participants.is_empty()
                && v.participants.iter().all(|p| {
                    identity(&p.project)
                        && nonempty(&p.scope)
                        && digest(&p.basis_digest)
                        && p.contract_digest == v.contract_digest
                        && ids.insert(&p.project)
                })
        }
        WorkflowPayload::EvidenceAvailability(v) => {
            charge(
                1 + v.retained_digests.len()
                    + v.history_refs.len()
                    + v.affected_assurance_refs.len(),
            )?;
            exact(&v.evidence)
                && v.retained_digests.iter().all(|s| digest(s))
                && unique_refs(&v.history_refs)
                && unique_refs(&v.affected_assurance_refs)
        }
        WorkflowPayload::Stop(v) => {
            charge(usize::from(v.parent_scope.is_some()))?;
            valid_subject(&v.subject)
                && (v.class != StopClass::Work || v.subject.result_digest.is_some())
                && identity(&v.scope_id)
                && v.parent_scope
                    .as_ref()
                    .is_none_or(|s| identity(s) && s != &v.scope_id)
                && (if v.class == StopClass::Work {
                    v.work_state == WorkState::Complete && v.cause == StopCause::Completed
                } else {
                    v.cause != StopCause::Completed
                })
        }
        WorkflowPayload::WorkChange(v) => {
            charge(5 + v.limits.len())?;
            digest(&v.old_basis)
                && digest(&v.new_basis)
                && v.old_basis != v.new_basis
                && identity(&v.scope_id)
                && identity(&v.choice_ref)
                && identity(&v.fence_ref)
                && identity(&v.context_ref)
                && v.limits.iter().all(|s| nonempty(s))
                && match v.class {
                    ChangeClass::Harness => v.apply_at == ApplyAt::NextToolCall,
                    ChangeClass::Work => v.apply_at != ApplyAt::NextToolCall,
                }
        }
        WorkflowPayload::Overview(v) => {
            charge(v.completions.len())?;
            for c in &v.completions {
                charge(1 + c.document_trail.len() + c.next_steps.len() + c.assurance_refs.len())?;
            }
            let mut ids = BTreeSet::new();
            identity(&v.tracker)
                && digest(&v.revision)
                && v.completions.iter().all(|c| {
                    identity(&c.event_id)
                        && valid_subject(&c.subject)
                        && c.subject.result_digest.is_some()
                        && identity(&c.scope_id)
                        && nonempty(&c.notes)
                        && !c.document_trail.is_empty()
                        && c.document_trail
                            .iter()
                            .chain(&c.next_steps)
                            .all(|s| nonempty(s))
                        && qualification_refs(c.qualification, &c.assurance_refs)
                        && ids.insert(&c.event_id)
                })
        }
        WorkflowPayload::Handoff(v) => {
            charge(3 + v.document_trail.len() + v.next_steps.len() + v.assurance_refs.len())?;
            identity(&v.event_id)
                && exact(&v.overview)
                && qualification_refs(v.qualification, &v.assurance_refs)
                && nonempty(&v.overview_url)
                && nonempty(&v.safeword)
                && !v.safeword.contains(['\n', '\r'])
                && nonempty(&v.notes)
                && !v.document_trail.is_empty()
                && v.document_trail
                    .iter()
                    .chain(&v.next_steps)
                    .all(|s| nonempty(s))
        }
        WorkflowPayload::ReviewManifest(v) => {
            charge(v.members.len())?;
            let mut ids = BTreeSet::new();
            let mut valid = !v.members.is_empty();
            for m in &v.members {
                charge(1 + m.evidence.len())?;
                let mut evidence = BTreeSet::new();
                valid &= valid_subject(&m.subject)
                    && m.subject.result_digest.is_some()
                    && exact(&m.profile)
                    && ids.insert((
                        &m.subject.warrant,
                        &m.subject.contract_digest,
                        &m.subject.result_digest,
                    ))
                    && m.evidence
                        .iter()
                        .all(|r| exact(r) && evidence.insert(&r.id));
            }
            valid
        }
    };
    if !valid {
        return Err(error(
            "workflow.invalid",
            "Malformed workflow record or mismatched contract",
        ));
    }
    Ok(CheckedWorkflowRecord {
        envelope,
        raw_digest: raw_digest(bytes),
    })
}
fn exact(r: &ExactReference) -> bool {
    identity(&r.id) && digest(&r.digest)
}

/// Explicit tracker-adapter attestation over the exact supplied overview bytes.
#[derive(Clone, Debug)]
pub struct TrackerTrust {
    pub overview_url: String,
    pub raw_digest: String,
    pub tracker: String,
    pub revision: String,
}
#[derive(Clone, Copy, Debug)]
pub enum ResponseStyle {
    Minimal,
    WithNotes,
}
#[derive(Debug)]
pub struct HandoffResult {
    pub work_complete: bool,
    pub state: FindingState,
    pub reason: &'static str,
    pub response: Option<String>,
}
pub fn render_handoff(
    stop: &CheckedWorkflowRecord,
    overview: &CheckedWorkflowRecord,
    handoff: &CheckedWorkflowRecord,
    trust: &TrackerTrust,
    configured_word: &str,
    style: ResponseStyle,
    max_bytes: usize,
) -> Result<HandoffResult, RecordError> {
    let (WorkflowPayload::Stop(s), WorkflowPayload::Overview(v), WorkflowPayload::Handoff(h)) = (
        &stop.envelope.payload,
        &overview.envelope.payload,
        &handoff.envelope.payload,
    ) else {
        return Err(error("handoff.kind", "Expected stop, overview and handoff"));
    };
    let complete = s.work_state == WorkState::Complete;
    let pending = |reason| HandoffResult {
        work_complete: complete,
        state: FindingState::Unknown,
        reason,
        response: None,
    };
    if s.class != StopClass::Work || !complete {
        return Ok(pending(
            "Harness stop or incomplete scope cannot emit completion signal",
        ));
    }
    if trust.raw_digest != overview.raw_digest
        || trust.tracker != v.tracker
        || trust.revision != v.revision
        || trust.overview_url != h.overview_url
    {
        return Ok(pending("Tracker origin not established for exact overview"));
    }
    if h.event_id != stop.envelope.id
        || h.overview.id != overview.envelope.id
        || h.overview.digest != overview.raw_digest
        || h.safeword != configured_word
    {
        return Ok(pending(
            "Handoff does not bind exact completion and configuration",
        ));
    }
    if !v.completions.iter().any(|c| {
        c.event_id == stop.envelope.id
            && c.subject == s.subject
            && c.scope_id == s.scope_id
            && c.notes == h.notes
            && c.document_trail == h.document_trail
            && c.next_steps == h.next_steps
            && c.qualification == h.qualification
            && c.assurance_refs.len() == h.assurance_refs.len()
            && c.assurance_refs
                .iter()
                .zip(&h.assurance_refs)
                .all(|(a, b)| a.id == b.id && a.digest == b.digest)
    }) {
        return Ok(pending(
            "Overview does not acknowledge exact completion and required document trail",
        ));
    }
    let notes = if matches!(style, ResponseStyle::WithNotes) {
        h.notes.as_str()
    } else {
        ""
    };
    let count = h
        .safeword
        .len()
        .checked_add(h.overview_url.len())
        .and_then(|n| n.checked_add(notes.len()))
        .and_then(|n| n.checked_add(if notes.is_empty() { 1 } else { 2 }))
        .ok_or_else(|| error("resource-limit", "Response overflow"))?;
    if max_bytes == 0 || count > max_bytes {
        return Err(error(
            "resource-limit",
            "Response would exceed configured bound",
        ));
    }
    let response = if notes.is_empty() {
        format!("{}\n{}", h.safeword, h.overview_url)
    } else {
        format!("{}\n{}\n{}", h.safeword, h.overview_url, notes)
    };
    Ok(HandoffResult {
        work_complete: true,
        state: FindingState::Established,
        reason: "Exact scoped completion acknowledged by trusted overview",
        response: Some(response),
    })
}
/// Same event identity must retain exact bytes. A changed event is not a retry.
pub fn check_replay(
    previous: &CheckedWorkflowRecord,
    current: &CheckedWorkflowRecord,
) -> Result<bool, RecordError> {
    if previous.envelope.id != current.envelope.id {
        return Ok(false);
    }
    if previous.raw_digest != current.raw_digest {
        return Err(error(
            "event.conflict",
            "Same event identity with different content",
        ));
    }
    Ok(true)
}
#[derive(Clone, Debug)]
pub struct ResumeFacts {
    pub change_digest: String,
    pub choice_established: bool,
    pub writers_fenced: bool,
    pub context_available: bool,
    pub within_limits: bool,
    pub at_work_stop: bool,
    pub before_next_tool: bool,
}
pub fn evaluate_resume(
    change: &CheckedWorkflowRecord,
    facts: &ResumeFacts,
) -> Result<Finding, RecordError> {
    let WorkflowPayload::WorkChange(c) = &change.envelope.payload else {
        return Err(error("resume.kind", "Expected work change"));
    };
    let state = if facts.change_digest != change.raw_digest
        || !facts.choice_established
        || !facts.context_available
    {
        FindingState::Unknown
    } else if !facts.within_limits {
        FindingState::Unmet
    } else {
        match c.class {
            ChangeClass::Work if !facts.writers_fenced => FindingState::Unknown,
            ChangeClass::Work if c.apply_at == ApplyAt::NextWorkStop && !facts.at_work_stop => {
                FindingState::Unmet
            }
            ChangeClass::Harness if !facts.before_next_tool => FindingState::Unmet,
            _ => FindingState::Established,
        }
    };
    Ok(finding(
        &change.envelope.id,
        state,
        "Exact change, choice, context, limits and required stop boundary must be established",
    ))
}
#[derive(Clone, Debug)]
pub struct AvailabilityFacts {
    pub record_digest: String,
    pub retained_exact_bytes: bool,
    pub expected_history: Vec<String>,
}
#[derive(Debug)]
pub struct AvailabilityReport {
    pub declared_state: Availability,
    pub support: FindingState,
    pub history_preserved: bool,
}
pub fn evaluate_availability(
    record: &CheckedWorkflowRecord,
    facts: &AvailabilityFacts,
) -> Result<AvailabilityReport, RecordError> {
    let WorkflowPayload::EvidenceAvailability(a) = &record.envelope.payload else {
        return Err(error(
            "availability.kind",
            "Expected evidence availability record",
        ));
    };
    // Borrow supplied history; do not clone or delete the authority records.
    let history_preserved = a.history_refs == facts.expected_history;
    let support = if facts.record_digest != record.raw_digest {
        FindingState::Unknown
    } else if !history_preserved {
        FindingState::Unmet
    } else if facts.retained_exact_bytes && a.retained_digests.contains(&a.evidence.digest) {
        FindingState::Established
    } else {
        FindingState::Unknown
    };
    Ok(AvailabilityReport {
        declared_state: a.state,
        support,
        history_preserved,
    })
}
#[derive(Clone, Debug)]
pub struct HumanReviewTrust {
    pub manifest_digest: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub secure_human_act: bool,
    pub reviewed_outcome_evidence_risks: bool,
}
/// Establish only human acceptance coverage. Each member still needs its own
/// independent assurance evaluation; this function never returns eligibility.
pub fn batch_acceptance(
    manifest: &CheckedWorkflowRecord,
    member: &ReviewMember,
    trust: &HumanReviewTrust,
) -> Result<Finding, RecordError> {
    let WorkflowPayload::ReviewManifest(m) = &manifest.envelope.payload else {
        return Err(error("batch.kind", "Expected review manifest"));
    };
    if trust.manifest_digest != manifest.raw_digest
        || trust.actor_kind != ActorKind::Human
        || trust.actor_id.len() > 4096
        || !identity(&trust.actor_id)
        || !trust.secure_human_act
        || !trust.reviewed_outcome_evidence_risks
    {
        return Ok(finding(
            &manifest.envelope.id,
            FindingState::Unknown,
            "Exact secure human review not established",
        ));
    }
    let covered = m.members.iter().any(|v| {
        v.subject == member.subject
            && v.profile.id == member.profile.id
            && v.profile.digest == member.profile.digest
            && v.evidence.len() == member.evidence.len()
            && v.evidence
                .iter()
                .zip(&member.evidence)
                .all(|(a, b)| a.id == b.id && a.digest == b.digest)
    });
    Ok(finding(
        &manifest.envelope.id,
        if covered {
            FindingState::Established
        } else {
            FindingState::Unmet
        },
        "Coverage binds exact member, profile and evidence; no sibling assurance inherited",
    ))
}

fn qualification_refs(claim: QualificationClaim, refs: &[ExactReference]) -> bool {
    let mut ids = BTreeSet::new();
    (claim != QualificationClaim::Verified || !refs.is_empty())
        && refs.iter().all(|r| exact(r) && ids.insert(&r.id))
}
