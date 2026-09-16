// SPDX-License-Identifier: Apache-2.0
//! Pure record checks over explicit, caller-authenticated facts. No signing or I/O.
use super::validate::identity;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub warrant: String,
    pub contract_digest: String,
    pub result_digest: Option<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActorKind {
    Human,
    Agent,
    Service,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub id: String,
    pub kind: ActorKind,
    pub role: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source: String,
    pub authenticity: Authenticity,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Authenticity {
    Unverified,
    ExternallyVerified,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRecord {
    schema: String,
    id: String,
    kind: String,
    subject: Subject,
    actor: Actor,
    #[serde(deserialize_with = "required_option")]
    policy_ref: Option<String>,
    evidence_refs: Vec<String>,
    #[serde(deserialize_with = "unique_json")]
    payload: Value,
    provenance: Provenance,
}
#[derive(Clone, Debug)]
pub struct Record {
    pub id: String,
    pub subject: Subject,
    pub actor: Actor,
    pub policy_ref: Option<String>,
    pub evidence_refs: Vec<String>,
    pub payload: Payload,
    pub provenance: Provenance,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "kebab-case")]
pub enum Payload {
    Claim(Statement),
    Observation(Observation),
    Inference(Inference),
    Judgment(Judgment),
    Verification(Verification),
    Permission(Permission),
    PolicyDisposition(PolicyDisposition),
    GovernanceAdoption(GovernanceAdoption),
    HumanAcceptance(HumanAcceptance),
    Qualification(Qualification),
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    pub statement: String,
    pub scope: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inference {
    pub statement: String,
    pub scope: String,
    pub limitations: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judgment {
    pub statement: String,
    pub scope: String,
    pub limitations: Vec<String>,
    pub decision: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Execution {
    Completed,
    Unavailable,
    Error,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    PASS,
    FAIL,
    UNKNOWN,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub check_id: String,
    pub check_digest: String,
    pub execution: Execution,
    pub verdict: Verdict,
    pub artifact_refs: Vec<String>,
    pub reason: Option<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Disposition {
    Established,
    NotEstablished,
    Refuted,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obligation {
    pub id: String,
    pub scope: String,
    pub disposition: Disposition,
    pub evidence_refs: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub performer_id: String,
    pub candidate_digest: String,
    pub obligations: Vec<Obligation>,
    pub isolation_refs: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Act {
    Execute,
    AmendContract,
    AdoptSas,
    Merge,
    Deploy,
    EditPolicy,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Permission {
    pub acts: Vec<Act>,
    pub constraints: BTreeMap<String, Value>,
    pub effective_policy_digest: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDisposition {
    pub decision: String,
    pub limitations: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernanceAdoption {
    pub document_digest: String,
    pub decision: String,
    pub delegated: bool,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanAcceptance {
    pub decision: String,
    pub reviewed: Vec<String>,
    pub meaning: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Criterion {
    pub id: String,
    pub disposition: Disposition,
    pub evidence_refs: Vec<String>,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Qualification {
    pub profile: String,
    pub scope: String,
    pub verification_ref: String,
    pub acceptance_ref: String,
    pub limitations: Vec<String>,
    pub history_claims: Vec<String>,
    pub criteria: Vec<Criterion>,
}

/// Out-of-band authentication established by the caller, bound to exact raw bytes.
/// `human_signed` means a secure human signing act, not a service acting for a human.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedRecord {
    pub id: String,
    pub raw_digest: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub human_signed: bool,
    pub observation_established: bool,
    pub assurance_criteria: Vec<String>,
    pub observed_at: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
pub struct RecordLimits {
    pub bytes: usize,
    pub records: usize,
    pub references: usize,
}
impl Default for RecordLimits {
    fn default() -> Self {
        Self {
            bytes: 4 * 1024 * 1024,
            records: 4096,
            references: 16384,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordError {
    pub code: &'static str,
    pub message: String,
}
fn error(code: &'static str, message: impl Into<String>) -> RecordError {
    RecordError {
        code,
        message: message.into(),
    }
}
#[derive(Clone, Debug)]
pub struct CheckedRecord {
    record: Record,
    raw_digest: String,
    supported: bool,
    authenticated: bool,
    human_signed: bool,
    observation_established: bool,
    assurance_criteria: Vec<String>,
    observed_at: Option<u64>,
}
impl CheckedRecord {
    pub fn record(&self) -> &Record {
        &self.record
    }
    pub fn raw_digest(&self) -> &str {
        &self.raw_digest
    }
    pub fn authenticated(&self) -> bool {
        self.authenticated
    }
    pub fn human_signed(&self) -> bool {
        self.human_signed
    }
}
#[derive(Debug)]
pub struct CheckedRecords {
    records: Vec<CheckedRecord>,
    index: BTreeMap<String, usize>,
    subject: Subject,
    missing_refs: Vec<String>,
}
impl CheckedRecords {
    pub fn records(&self) -> &[CheckedRecord] {
        &self.records
    }
    pub fn missing_refs(&self) -> &[String] {
        &self.missing_refs
    }
    fn get(&self, id: &str) -> Option<&CheckedRecord> {
        self.index.get(id).map(|i| &self.records[*i])
    }
}
pub fn raw_digest(bytes: &[u8]) -> String {
    super::source::raw_digest(bytes)
}
fn digest(s: &str) -> bool {
    s.len() == 71
        && s.starts_with("sha256:")
        && s.as_bytes()[7..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}
fn nonempty(s: &str) -> bool {
    !s.trim().is_empty()
}
fn unique_refs(v: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    v.iter().all(|s| identity(s) && seen.insert(s))
}
fn valid_subject(s: &Subject) -> bool {
    identity(&s.warrant)
        && digest(&s.contract_digest)
        && s.result_digest.as_ref().is_none_or(|d| digest(d))
}

pub fn check_records(
    inputs: &[&[u8]],
    subject: &Subject,
    trust: &[TrustedRecord],
    limits: RecordLimits,
) -> Result<CheckedRecords, RecordError> {
    if limits.bytes == 0
        || limits.records == 0
        || limits.references == 0
        || inputs.len() > limits.records
        || trust.len() > limits.records
    {
        return Err(error("resource-limit", "Invalid limits or record count"));
    }
    let mut remaining = limits.bytes;
    for input in inputs {
        remaining = remaining
            .checked_sub(input.len())
            .ok_or_else(|| error("resource-limit", "Record bytes exceed limit"))?;
    }
    for s in [&subject.warrant, &subject.contract_digest]
        .into_iter()
        .chain(subject.result_digest.iter())
    {
        remaining = remaining
            .checked_sub(s.len())
            .ok_or_else(|| error("resource-limit", "Subject exceeds limit"))?;
    }
    let mut trust_refs = limits.references;
    for fact in trust {
        trust_refs = trust_refs
            .checked_sub(fact.assurance_criteria.len())
            .ok_or_else(|| error("resource-limit", "Trust criteria exceed limit"))?;
        for s in [&fact.id, &fact.raw_digest, &fact.actor_id]
            .into_iter()
            .chain(fact.assurance_criteria.iter())
        {
            remaining = remaining
                .checked_sub(s.len())
                .ok_or_else(|| error("resource-limit", "Trust input exceeds limit"))?;
        }
    }
    if !valid_subject(subject) {
        return Err(error("record.subject", "Malformed expected subject"));
    }
    let mut trusted = BTreeMap::new();
    let mut actor_kinds = BTreeMap::new();
    for fact in trust {
        if actor_kinds
            .insert(fact.actor_id.as_str(), fact.actor_kind)
            .is_some_and(|kind| kind != fact.actor_kind)
        {
            return Err(error("record.trust", "Conflicting actor kind facts"));
        }
        let mut criterion_ids = BTreeSet::new();
        if fact
            .assurance_criteria
            .iter()
            .any(|id| !nonempty(id) || !criterion_ids.insert(id))
        {
            return Err(error(
                "record.trust",
                "Invalid or duplicate criterion attestation",
            ));
        }
        if !identity(&fact.id)
            || !identity(&fact.actor_id)
            || !digest(&fact.raw_digest)
            || (fact.human_signed && fact.actor_kind != ActorKind::Human)
            || trusted.insert(fact.id.as_str(), fact).is_some()
        {
            return Err(error(
                "record.trust",
                "Malformed or duplicate trusted identity",
            ));
        }
    }
    let mut ids = BTreeSet::new();
    let mut records = Vec::new();
    let mut refs = limits.references;
    for input in inputs {
        let unique: UniqueJson =
            serde_json::from_slice(input).map_err(|e| error("record.syntax", e.to_string()))?;
        let raw: RawRecord = strict_object_decode(unique.0, "record.syntax")?;
        if raw.schema != "oh.war/record/1.0.0-rc.2"
            || !identity(&raw.id)
            || !ids.insert(raw.id.clone())
        {
            return Err(error(
                "record.identity",
                "Unsupported schema, invalid or duplicate ID",
            ));
        }
        if !valid_subject(&raw.subject)
            || raw.subject.warrant != subject.warrant
            || raw.subject.contract_digest != subject.contract_digest
            || raw
                .subject
                .result_digest
                .as_ref()
                .is_some_and(|d| Some(d) != subject.result_digest.as_ref())
        {
            return Err(error(
                "record.subject",
                "Record belongs to another exact subject",
            ));
        }
        if !identity(&raw.actor.id)
            || !nonempty(&raw.actor.role)
            || !nonempty(&raw.provenance.source)
            || !unique_refs(&raw.evidence_refs)
            || raw.policy_ref.as_ref().is_some_and(|r| !identity(r))
        {
            return Err(error(
                "record.fields",
                "Malformed actor, provenance or references",
            ));
        }
        let payload: Payload = strict_object_decode(
            serde_json::json!({"kind":raw.kind,"payload":raw.payload}),
            "record.payload",
        )?;
        if !matches!(
            payload,
            Payload::Permission(_) | Payload::GovernanceAdoption(_)
        ) && raw.subject.result_digest.is_none()
        {
            return Err(error(
                "record.subject",
                "Result-related record needs result digest",
            ));
        }
        if actor_kinds
            .get(raw.actor.id.as_str())
            .is_some_and(|kind| *kind != raw.actor.kind)
        {
            return Err(error(
                "record.trust-mismatch",
                "Actor kind contradicts supplied identity facts",
            ));
        }
        validate_payload(&payload, &raw.actor, &raw.subject)?;
        let record = Record {
            id: raw.id,
            subject: raw.subject,
            actor: raw.actor,
            policy_ref: raw.policy_ref,
            evidence_refs: raw.evidence_refs,
            payload,
            provenance: raw.provenance,
        };
        let all_refs = references(&record);
        refs = refs
            .checked_sub(all_refs.len())
            .ok_or_else(|| error("resource-limit", "Too many references"))?;
        let hash = raw_digest(input);
        let mut authenticated = false;
        let mut human_signed = false;
        let mut observation_established = false;
        let mut assurance_criteria = Vec::new();
        let mut observed_at = None;
        if let Some(fact) = trusted.get(record.id.as_str()) {
            if fact.raw_digest != hash
                || fact.actor_id != record.actor.id
                || fact.actor_kind != record.actor.kind
            {
                return Err(error(
                    "record.trust-mismatch",
                    "Authentication does not bind these bytes and actor",
                ));
            }
            authenticated = true;
            human_signed = fact.human_signed;
            observation_established = fact.observation_established;
            assurance_criteria = fact.assurance_criteria.clone();
            observed_at = fact.observed_at;
        }
        records.push(CheckedRecord {
            record,
            raw_digest: hash,
            supported: false,
            authenticated,
            human_signed,
            observation_established,
            assurance_criteria,
            observed_at,
        });
    }
    let missing_refs: BTreeSet<_> = records
        .iter()
        .flat_map(|r| references(&r.record))
        .filter(|r| !ids.contains(*r))
        .cloned()
        .collect();
    let index: BTreeMap<_, _> = records
        .iter()
        .enumerate()
        .map(|(i, r)| (r.record.id.clone(), i))
        .collect();
    let mut parents = vec![Vec::new(); records.len()];
    let mut pending = vec![0usize; records.len()];
    let mut support: Vec<_> = records.iter().map(|r| r.authenticated).collect();
    for (i, r) in records.iter().enumerate() {
        for id in references(&r.record) {
            if let Some(&child) = index.get(id) {
                parents[child].push(i);
                pending[i] += 1;
            } else {
                support[i] = false;
            }
        }
    }
    let mut ready: std::collections::VecDeque<_> = pending
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    while let Some(child) = ready.pop_front() {
        records[child].supported = support[child];
        for &parent in &parents[child] {
            support[parent] &= support[child];
            pending[parent] -= 1;
            if pending[parent] == 0 {
                ready.push_back(parent);
            }
        }
    }
    // Cycles and their dependents never drain; their support stays unknown.
    Ok(CheckedRecords {
        records,
        index,
        subject: subject.clone(),
        missing_refs: missing_refs.into_iter().collect(),
    })
}
fn references(record: &Record) -> Vec<&String> {
    let mut refs: Vec<_> = record
        .evidence_refs
        .iter()
        .chain(record.policy_ref.iter())
        .collect();
    match &record.payload {
        Payload::Observation(o) => refs.extend(o.artifact_refs.iter()),
        Payload::Verification(v) => {
            refs.extend(v.isolation_refs.iter());
            for o in &v.obligations {
                refs.extend(o.evidence_refs.iter());
            }
        }
        Payload::Qualification(q) => {
            refs.extend([&q.verification_ref, &q.acceptance_ref]);
            for c in &q.criteria {
                refs.extend(c.evidence_refs.iter());
            }
        }
        _ => {}
    }
    refs
}
fn validate_payload(p: &Payload, actor: &Actor, subject: &Subject) -> Result<(), RecordError> {
    let valid = match p {
        Payload::Claim(s) => nonempty(&s.statement) && nonempty(&s.scope),
        Payload::Inference(s) => nonempty(&s.statement) && nonempty(&s.scope),
        Payload::Judgment(s) => {
            nonempty(&s.statement) && nonempty(&s.scope) && nonempty(&s.decision)
        }
        Payload::Observation(o) => {
            identity(&o.check_id)
                && digest(&o.check_digest)
                && unique_refs(&o.artifact_refs)
                && (o.execution == Execution::Completed || o.verdict == Verdict::UNKNOWN)
                && (o.verdict != Verdict::UNKNOWN || o.reason.as_ref().is_some_and(|s| nonempty(s)))
        }
        Payload::Verification(v) => {
            identity(&v.performer_id)
                && digest(&v.candidate_digest)
                && subject.result_digest.as_ref() == Some(&v.candidate_digest)
                && unique_refs(&v.isolation_refs)
                && {
                    let mut ids = BTreeSet::new();
                    v.obligations.iter().all(|o| {
                        nonempty(&o.id)
                            && ids.insert(&o.id)
                            && nonempty(&o.scope)
                            && unique_refs(&o.evidence_refs)
                    })
                }
        }
        Payload::Permission(p) => {
            !p.acts.is_empty()
                && p.acts
                    .iter()
                    .enumerate()
                    .all(|(i, a)| !p.acts[..i].contains(a))
                && digest(&p.effective_policy_digest)
        }
        Payload::PolicyDisposition(p) => nonempty(&p.decision),
        Payload::GovernanceAdoption(g) => digest(&g.document_digest) && nonempty(&g.decision),
        Payload::HumanAcceptance(a) => {
            actor.kind == ActorKind::Human
                && a.decision == "accept"
                && nonempty(&a.meaning)
                && ["outcome", "verification", "risks"]
                    .iter()
                    .all(|s| a.reviewed.iter().any(|v| v == s))
                && {
                    let mut seen = BTreeSet::new();
                    a.reviewed.iter().all(|v| {
                        ["outcome", "verification", "risks", "code"].contains(&v.as_str())
                            && seen.insert(v)
                    })
                }
        }
        Payload::Qualification(q) => {
            nonempty(&q.profile)
                && nonempty(&q.scope)
                && BASELINE
                    .iter()
                    .all(|id| q.criteria.iter().any(|c| c.id == *id))
                && identity(&q.verification_ref)
                && identity(&q.acceptance_ref)
                && {
                    let mut ids = BTreeSet::new();
                    q.criteria.iter().all(|c| {
                        nonempty(&c.id) && ids.insert(&c.id) && unique_refs(&c.evidence_refs)
                    })
                }
        }
    };
    if valid {
        Ok(())
    } else {
        Err(error("record.payload", "Invalid record payload semantics"))
    }
}

// serde_json::Value normally accepts duplicate object keys. Preserve refusal at
// every payload depth before converting to a typed payload.
struct UniqueJson(Value);
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Bool(v)))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson(v.into()))
            }
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| UniqueJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("nonfinite number"))
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(v.into())))
            }
            fn visit_string<E: serde::de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::String(v)))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson(Value::Null))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut v = Vec::new();
                while let Some(x) = a.next_element::<UniqueJson>()? {
                    v.push(x.0);
                }
                Ok(UniqueJson(Value::Array(v)))
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<Self::Value, A::Error> {
                let mut m = serde_json::Map::new();
                while let Some(k) = a.next_key::<String>()? {
                    if m.contains_key(&k) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    m.insert(k, a.next_value::<UniqueJson>()?.0);
                }
                Ok(UniqueJson(Value::Object(m)))
            }
        }
        d.deserialize_any(Visitor)
    }
}
fn unique_json<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
    UniqueJson::deserialize(d).map(|v| v.0)
}
fn required_option<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingState {
    Established,
    Unmet,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub id: String,
    pub state: FindingState,
    pub reason: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Purpose {
    ActionGate,
    Qualification,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Timing {
    Any,
    Before(u64),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum Requirement {
    PassingCheck {
        record_id: String,
        check_digest: String,
    },
    HumanAcceptance {
        record_id: String,
    },
    Permission {
        record_id: String,
        act: Act,
    },
}
impl Requirement {
    fn record_id(&self) -> &str {
        match self {
            Self::PassingCheck { record_id, .. }
            | Self::HumanAcceptance { record_id }
            | Self::Permission { record_id, .. } => record_id,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub id: String,
    pub action: String,
    pub stage: String,
    pub purpose: Purpose,
    pub timing: Timing,
    pub requirement: Requirement,
}
/// Caller-established human-approved policy, not a record's self-declared grant.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyAuthority {
    pub record_id: String,
    pub digest: String,
    pub acts: Vec<Act>,
}
#[derive(Debug, Serialize)]
pub struct ReadinessReport {
    /// Inline profile: exact conditions are retained below; no ambient lookup.
    pub profile: &'static str,
    pub subject: Subject,
    pub action: String,
    pub stage: String,
    pub conditions: Vec<Condition>,
    pub action_ready: bool,
    pub action_gates: Vec<Finding>,
    pub qualification: Vec<Finding>,
}
fn finding(id: &str, state: FindingState, reason: &str) -> Finding {
    Finding {
        id: id.into(),
        state,
        reason: reason.into(),
    }
}
fn merge_state(a: FindingState, b: FindingState) -> FindingState {
    match (a, b) {
        (FindingState::Unmet, _) | (_, FindingState::Unmet) => FindingState::Unmet,
        (FindingState::Unknown, _) | (_, FindingState::Unknown) => FindingState::Unknown,
        _ => FindingState::Established,
    }
}
fn supporting_records(_checked: &CheckedRecords, record: &CheckedRecord) -> bool {
    record.supported
}
fn check_requirement(
    checked: &CheckedRecords,
    requirement: &Requirement,
    timing: Timing,
    policies: &[PolicyAuthority],
) -> (FindingState, &'static str) {
    let Some(record) = checked.get(requirement.record_id()) else {
        return (FindingState::Unknown, "Required record missing");
    };
    if !supporting_records(checked, record) {
        return (
            FindingState::Unknown,
            "Authentication or supporting records missing",
        );
    }
    if let Timing::Before(bound) = timing {
        match record.observed_at {
            None => return (FindingState::Unknown, "Trusted event time missing"),
            Some(t) if t >= bound => {
                return (
                    FindingState::Unmet,
                    "Event did not occur before required time",
                );
            }
            _ => {}
        }
    }
    match (requirement, &record.record.payload) {
        (Requirement::PassingCheck { check_digest, .. }, Payload::Observation(o)) => {
            if !record.observation_established {
                return (
                    FindingState::Unknown,
                    "Observed execution not established by caller",
                );
            }
            if o.check_digest != *check_digest {
                return (FindingState::Unmet, "Check revision differs");
            }
            match o.verdict {
                Verdict::PASS => (
                    FindingState::Established,
                    "Exact authenticated passing observation",
                ),
                Verdict::FAIL => (FindingState::Unmet, "Observed failure"),
                Verdict::UNKNOWN => (FindingState::Unknown, "Check was not established"),
            }
        }
        (Requirement::HumanAcceptance { .. }, Payload::HumanAcceptance(_)) => {
            if record.human_signed {
                (FindingState::Established, "Exact secure human acceptance")
            } else {
                (
                    FindingState::Unknown,
                    "Secure human signing act not established",
                )
            }
        }
        (Requirement::Permission { act, .. }, Payload::Permission(p)) => {
            if !p.constraints.is_empty() {
                return (
                    FindingState::Unknown,
                    "Permission constraints require caller applicability evaluation",
                );
            }
            if !p.acts.contains(act) {
                return (FindingState::Unmet, "Requested act not granted");
            }
            if record.human_signed {
                return (FindingState::Established, "Human-signed permission");
            }
            if *act == Act::EditPolicy {
                return (
                    FindingState::Unmet,
                    "Automation cannot grant effective policy edits",
                );
            }
            if policies.iter().any(|f| {
                Some(&f.record_id) == record.record.policy_ref.as_ref()
                    && f.digest == p.effective_policy_digest
                    && f.acts.contains(act)
                    && checked
                        .get(&f.record_id)
                        .is_some_and(|approval| approval.human_signed && approval.supported)
            }) {
                (
                    FindingState::Established,
                    "Explicit caller-established human policy delegation",
                )
            } else {
                (
                    FindingState::Unknown,
                    "Applicable human-approved policy not established",
                )
            }
        }
        _ => (
            FindingState::Unmet,
            "Record kind cannot establish this requirement",
        ),
    }
}
fn validate_conditions(
    conditions: &[Condition],
    policies: &[PolicyAuthority],
    limits: RecordLimits,
) -> Result<(), RecordError> {
    if limits.bytes == 0
        || limits.records == 0
        || limits.references == 0
        || conditions.len() > limits.records
        || policies.len() > limits.records
    {
        return Err(error("resource-limit", "Invalid condition limits"));
    }
    let mut remaining = limits.bytes;
    let mut names = BTreeSet::new();
    for c in conditions {
        for s in [&c.id, &c.action, &c.stage, c.requirement.record_id()] {
            remaining = remaining
                .checked_sub(s.len())
                .ok_or_else(|| error("resource-limit", "Conditions exceed byte limit"))?;
        }
        if !nonempty(&c.id)
            || !nonempty(&c.action)
            || !nonempty(&c.stage)
            || !identity(c.requirement.record_id())
            || !names.insert(&c.id)
        {
            return Err(error("condition.invalid", "Invalid or duplicate condition"));
        }
        if let Requirement::PassingCheck { check_digest, .. } = &c.requirement {
            remaining = remaining
                .checked_sub(check_digest.len())
                .ok_or_else(|| error("resource-limit", "Conditions exceed byte limit"))?;
            if !digest(check_digest) {
                return Err(error("condition.invalid", "Malformed check digest"));
            }
        }
    }
    let mut names = BTreeSet::new();
    let mut refs = limits.references;
    for p in policies {
        remaining = remaining
            .checked_sub(p.record_id.len())
            .and_then(|n| n.checked_sub(p.digest.len()))
            .ok_or_else(|| error("resource-limit", "Policy input exceeds limit"))?;
        refs = refs
            .checked_sub(p.acts.len())
            .ok_or_else(|| error("resource-limit", "Policy acts exceed limit"))?;
        if !identity(&p.record_id)
            || !digest(&p.digest)
            || !names.insert(&p.record_id)
            || p.acts.contains(&Act::EditPolicy)
        {
            return Err(error(
                "policy.invalid",
                "Malformed/duplicate policy or delegated policy edit",
            ));
        }
    }
    Ok(())
}
pub fn evaluate_readiness(
    checked: &CheckedRecords,
    conditions: &[Condition],
    action: &str,
    stage: &str,
    policies: &[PolicyAuthority],
    limits: RecordLimits,
) -> Result<ReadinessReport, RecordError> {
    validate_conditions(conditions, policies, limits)?;
    if !nonempty(action)
        || !nonempty(stage)
        || action.len().saturating_add(stage.len()) > limits.bytes
    {
        return Err(error("condition.invalid", "Invalid action scope"));
    }
    let mut report = ReadinessReport {
        profile: "explicit supplied conditions",
        subject: checked.subject.clone(),
        action: action.into(),
        stage: stage.into(),
        conditions: conditions
            .iter()
            .filter(|c| c.action == action && c.stage == stage)
            .cloned()
            .collect(),
        action_ready: true,
        action_gates: Vec::new(),
        qualification: Vec::new(),
    };
    for c in conditions
        .iter()
        .filter(|c| c.action == action && c.stage == stage)
    {
        let (state, reason) = check_requirement(checked, &c.requirement, c.timing, policies);
        let f = finding(&c.id, state, reason);
        match c.purpose {
            Purpose::ActionGate => {
                report.action_ready &= state == FindingState::Established;
                report.action_gates.push(f);
            }
            Purpose::Qualification => report.qualification.push(f),
        }
    }
    Ok(report)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expectation {
    pub id: String,
    pub scope: String,
    pub check_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedContract {
    pub warrant_source_digest: String,
    pub sources: Vec<super::source::SourceDescriptor>,
    pub constraints: Vec<super::source::BoundReference>,
    pub expectations: Vec<Expectation>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AssuranceStanding {
    Eligible,
    Ineligible,
    Unknown,
}
#[derive(Debug, Serialize)]
pub struct AssuranceReport {
    pub subject: Subject,
    pub baseline: &'static str,
    pub conditions: Vec<Condition>,
    pub standing: AssuranceStanding,
    pub findings: Vec<Finding>,
    pub issues_mark: bool,
}
const BASELINE: [&str; 8] = [
    "scope-permission",
    "obligations",
    "protected-expectations",
    "independence",
    "passing-checks",
    "adequacy",
    "human-review",
    "preservation",
];
/// Evaluate the fixed baseline plus caller-supplied stronger conditions. Conditions
/// supply the baseline's external observations; they cannot remove technical checks.
/// Returns eligibility only. No result issues a mark or authenticates a signature.
pub fn evaluate_assurance(
    checked: &CheckedRecords,
    contract_json: Option<&[u8]>,
    verification_ref: &str,
    acceptance_ref: &str,
    conditions: &[Condition],
    policies: &[PolicyAuthority],
    limits: RecordLimits,
) -> Result<AssuranceReport, RecordError> {
    validate_conditions(conditions, policies, limits)?;
    if !identity(verification_ref)
        || !identity(acceptance_ref)
        || verification_ref.len().saturating_add(acceptance_ref.len()) > limits.bytes
        || conditions
            .iter()
            .any(|c| c.purpose != Purpose::Qualification)
    {
        return Err(error(
            "assurance.input",
            "Invalid assessment references or condition purpose",
        ));
    }
    let mut findings = Vec::new();
    let mut expectations = None;
    match contract_json {
        None => findings.push(finding(
            "contract",
            FindingState::Unknown,
            "Normalized reviewed contract missing",
        )),
        Some(bytes) => {
            if bytes.len() > limits.bytes {
                return Err(error("resource-limit", "Contract exceeds byte limit"));
            }
            let unique: UniqueJson = serde_json::from_slice(bytes)
                .map_err(|e| error("contract.invalid", e.to_string()))?;
            let value = unique.0;
            let contract: NormalizedContract =
                strict_object_decode(value.clone(), "contract.invalid")?;
            if contract.sources.len() > limits.records
                || contract.constraints.len() > limits.references
                || contract.expectations.len() > limits.records
            {
                return Err(error("resource-limit", "Contract collection limit"));
            }
            let mut ids = BTreeSet::new();
            if !digest(&contract.warrant_source_digest)
                || !contract.sources.iter().any(|s| {
                    s.source_digest == contract.warrant_source_digest
                        && s.document
                            .as_ref()
                            .is_some_and(|d| d.kind == "warrant" && d.id == checked.subject.warrant)
                })
                || !contract.sources.windows(2).all(|s| s[0].path < s[1].path)
                || !contract.expectations.iter().all(|e| {
                    nonempty(&e.id)
                        && nonempty(&e.scope)
                        && digest(&e.check_digest)
                        && ids.insert(&e.id)
                })
            {
                return Err(error("contract.invalid", "Malformed normalized contract"));
            }
            let source_limits = super::source::SourceLimits {
                source_bytes: limits.bytes,
                total_bytes: limits.bytes,
                sources: limits.records,
                units: limits.references,
                metadata_bytes: limits.bytes,
                output_bytes: limits.bytes,
            };
            for source in &contract.sources {
                super::source::encode_source_descriptor(source, source_limits)
                    .map_err(|e| error("contract.invalid", e.message))?;
            }
            for reference in &contract.constraints {
                super::source::encode_bound_reference(reference, source_limits)
                    .map_err(|e| error("contract.invalid", e.message))?;
                if !contract.sources.iter().any(|s| {
                    s.source_digest == reference.source_digest && reference.end <= s.byte_length
                }) {
                    return Err(error(
                        "contract.invalid",
                        "Constraint not in supplied source inventory",
                    ));
                }
            }
            let hash = super::packet::structured_digest(
                super::packet::DigestDomain::Contract,
                &value,
                limits.bytes,
            )
            .map_err(|e| error(e.code, e.message))?;
            if hash != checked.subject.contract_digest {
                findings.push(finding(
                    "contract",
                    FindingState::Unmet,
                    "Reviewed contract digest differs",
                ));
            } else {
                findings.push(finding(
                    "contract",
                    FindingState::Established,
                    "Exact normalized contract",
                ));
                expectations = Some(contract.expectations);
            }
        }
    }
    let verifier = checked.get(verification_ref);
    let verification = verifier.and_then(|r| {
        if let Payload::Verification(v) = &r.record.payload {
            Some(v)
        } else {
            None
        }
    });
    let mut independent = FindingState::Unknown;
    let mut obligations = FindingState::Unknown;
    let mut checks = FindingState::Unknown;
    if let (Some(record), Some(v)) = (verifier, verification) {
        if v.performer_id == record.record.actor.id {
            independent = FindingState::Unmet;
        } else if supporting_records(checked, record) && !v.isolation_refs.is_empty() {
            independent = FindingState::Established;
            for id in &v.isolation_refs {
                let meaning = checked
                    .get(id)
                    .is_some_and(|r| r.assurance_criteria.iter().any(|c| c == "independence"));
                independent = merge_state(
                    independent,
                    if meaning {
                        observed_state(checked, id, None)
                    } else {
                        FindingState::Unknown
                    },
                );
            }
        }
        if record.authenticated
            && let Some(expected) = expectations.as_ref()
        {
            obligations = FindingState::Established;
            checks = FindingState::Established;
            for e in expected {
                match v
                    .obligations
                    .iter()
                    .find(|o| o.id == e.id && o.scope == e.scope)
                {
                    None => {
                        obligations = merge_state(obligations, FindingState::Unknown);
                        checks = merge_state(checks, FindingState::Unknown);
                    }
                    Some(o) => {
                        obligations = merge_state(
                            obligations,
                            match o.disposition {
                                Disposition::Established => FindingState::Established,
                                Disposition::NotEstablished => FindingState::Unknown,
                                Disposition::Refuted => FindingState::Unmet,
                            },
                        );
                        let mut state = FindingState::Unknown;
                        let mut found = false;
                        for id in &o.evidence_refs {
                            if let Some(r) = checked.get(id)
                                && let Payload::Observation(obs) = &r.record.payload
                                && obs.check_digest == e.check_digest
                            {
                                let current = if r.record.actor.id == v.performer_id {
                                    FindingState::Unmet
                                } else {
                                    observed_state(checked, id, Some(&e.check_digest))
                                };
                                state = if found {
                                    merge_state(state, current)
                                } else {
                                    current
                                };
                                found = true;
                            }
                        }
                        checks = merge_state(checks, state);
                    }
                }
            }
            if !supporting_records(checked, record) {
                obligations = merge_state(obligations, FindingState::Unknown);
                checks = merge_state(checks, FindingState::Unknown);
            }
        }
    }
    findings.push(finding(
        "independence",
        independent,
        "Requires a different verifier and authenticated passing isolation evidence",
    ));
    findings.push(finding(
        "obligations",
        obligations,
        "Verification must cover every exact contract expectation",
    ));
    findings.push(finding(
        "passing-checks",
        checks,
        "Claims cannot substitute for authenticated exact-check observations",
    ));
    let (human, reason) = check_requirement(
        checked,
        &Requirement::HumanAcceptance {
            record_id: acceptance_ref.into(),
        },
        Timing::Any,
        policies,
    );
    findings.push(finding("human-review", human, reason));
    // Remaining criteria have semantics owned by the supplied profile/trust adapter;
    // require explicit passing requirements, never the qualification record's claims.
    for id in BASELINE {
        if [
            "independence",
            "obligations",
            "passing-checks",
            "human-review",
        ]
        .contains(&id)
        {
            continue;
        }
        if !conditions.iter().any(|c| c.id == id) {
            findings.push(finding(
                id,
                FindingState::Unknown,
                "Required baseline condition missing",
            ));
        }
    }
    for c in conditions {
        let (mut state, mut reason) =
            check_requirement(checked, &c.requirement, c.timing, policies);
        if [
            "scope-permission",
            "protected-expectations",
            "adequacy",
            "preservation",
        ]
        .contains(&c.id.as_str())
            && !checked
                .get(c.requirement.record_id())
                .is_some_and(|r| r.assurance_criteria.contains(&c.id))
        {
            state = merge_state(state, FindingState::Unknown);
            reason = "Caller has not established this evidence's baseline meaning";
        }
        findings.push(finding(&format!("profile:{}", c.id), state, reason));
    }
    let state = findings.iter().fold(FindingState::Established, |state, f| {
        merge_state(state, f.state)
    });
    Ok(AssuranceReport {
        subject: checked.subject.clone(),
        baseline: "OpenWarrant RC.3 section 12 baseline",
        conditions: conditions.to_vec(),
        standing: match state {
            FindingState::Established => AssuranceStanding::Eligible,
            FindingState::Unmet => AssuranceStanding::Ineligible,
            FindingState::Unknown => AssuranceStanding::Unknown,
        },
        findings,
        issues_mark: false,
    })
}
fn observed_state(checked: &CheckedRecords, id: &str, expected: Option<&str>) -> FindingState {
    let Some(record) = checked.get(id) else {
        return FindingState::Unknown;
    };
    if !supporting_records(checked, record) || !record.observation_established {
        return FindingState::Unknown;
    }
    match &record.record.payload {
        Payload::Observation(o) if expected.is_none_or(|d| d == o.check_digest) => {
            match o.verdict {
                Verdict::PASS => FindingState::Established,
                Verdict::FAIL => FindingState::Unmet,
                Verdict::UNKNOWN => FindingState::Unknown,
            }
        }
        _ => FindingState::Unmet,
    }
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExactReference {
    pub id: String,
    pub digest: String,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentAction {
    Authorize,
    Complete,
}
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentAct {
    pub schema: String,
    pub id: String,
    pub act: AgentAction,
    pub subject: Subject,
    pub actor: Actor,
    #[serde(deserialize_with = "present_option")]
    pub policy_ref: Option<ExactReference>,
    pub meaning: String,
    pub evidence_refs: Vec<String>,
    #[serde(deserialize_with = "present_option")]
    pub signature_ref: Option<ExactReference>,
}
fn present_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}
#[derive(Debug, Serialize)]
pub struct AgentActReport {
    pub act: AgentAct,
    pub unsigned_digest: String,
    pub authenticated: bool,
    pub human_acceptance: bool,
}
/// Decode an unsigned or signed claim. Signature references are not authentication.
pub fn decode_agent_act(bytes: &[u8], limits: RecordLimits) -> Result<AgentActReport, RecordError> {
    if limits.bytes == 0
        || limits.records == 0
        || limits.references == 0
        || bytes.len() > limits.bytes
    {
        return Err(error("resource-limit", "Agent act limit"));
    }
    let mut payload = serde_json::from_slice::<UniqueJson>(bytes)
        .map_err(|e| error("agent-act.syntax", e.to_string()))?
        .0;
    let act: AgentAct = strict_object_decode(payload.clone(), "agent-act.syntax")?;
    if act.schema != "oh.war/agent-act/1.0.0-rc.3"
        || !identity(&act.id)
        || !valid_subject(&act.subject)
        || act.actor.kind != ActorKind::Agent
        || !identity(&act.actor.id)
        || !nonempty(&act.actor.role)
        || !nonempty(&act.meaning)
        || !unique_refs(&act.evidence_refs)
        || act
            .evidence_refs
            .len()
            .saturating_add(usize::from(act.policy_ref.is_some()))
            .saturating_add(usize::from(act.signature_ref.is_some()))
            > limits.references
        || act
            .policy_ref
            .iter()
            .chain(act.signature_ref.iter())
            .any(|r| !identity(&r.id) || !digest(&r.digest))
        || (act.act == AgentAction::Complete && act.subject.result_digest.is_none())
        || (act.act == AgentAction::Authorize && act.policy_ref.is_none())
    {
        return Err(error(
            "agent-act.invalid",
            "Invalid agent act; cannot assert a human act",
        ));
    }
    // Hash the original unsigned JSON payload; do not invent absent optional fields.
    payload
        .as_object_mut()
        .ok_or_else(|| error("agent-act.syntax", "Agent act must be an object"))?
        .remove("signature_ref");
    let wrapper =
        serde_json::json!({"digest_domain":"oh.war/agent-act/1.0.0-rc.3","payload":payload});
    let canonical =
        serde_jcs::to_vec(&wrapper).map_err(|e| error("agent-act.invalid", e.to_string()))?;
    if canonical.len() > limits.bytes {
        return Err(error("resource-limit", "Canonical agent act exceeds limit"));
    }
    Ok(AgentActReport {
        act,
        unsigned_digest: raw_digest(&canonical),
        authenticated: false,
        human_acceptance: false,
    })
}

pub mod workflow;

/// Structural JSON schemas. Semantic and trust checks still require the SDK.
#[cfg(feature = "schema")]
pub fn record_schemas() -> Value {
    #[allow(dead_code)]
    #[derive(schemars::JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct RecordShape {
        schema: String,
        id: String,
        subject: Subject,
        actor: Actor,
        policy_ref: Option<String>,
        evidence_refs: Vec<String>,
        provenance: Provenance,
        #[serde(flatten)]
        body: Payload,
    }
    let mut record =
        serde_json::to_value(schemars::schema_for!(RecordShape)).expect("schema serializes");
    record["required"]
        .as_array_mut()
        .expect("schema required fields")
        .push(Value::String("policy_ref".into()));
    record["properties"]["schema"]["const"] = Value::String("oh.war/record/1.0.0-rc.2".into());
    let mut agent =
        serde_json::to_value(schemars::schema_for!(AgentAct)).expect("schema serializes");
    agent["properties"]["schema"]["const"] = Value::String("oh.war/agent-act/1.0.0-rc.3".into());
    let mut workflow = serde_json::to_value(schemars::schema_for!(workflow::WorkflowEnvelope))
        .expect("schema serializes");
    workflow["properties"]["schema"]["const"] = Value::String(workflow::SCHEMA.into());
    serde_json::json!({"record":record,"agent_act":agent,"workflow":workflow})
}

// Serde structs also accept positional arrays. The wire schemas require objects,
// including nested structs; compare container shapes after typed decoding.
fn strict_object_decode<T: serde::de::DeserializeOwned + Serialize>(
    value: Value,
    code: &'static str,
) -> Result<T, RecordError> {
    fn same_shape(input: &Value, output: &Value) -> bool {
        match (input, output) {
            (Value::Object(a), Value::Object(b)) => a.iter().all(|(k, v)| {
                b.get(k)
                    .map_or_else(|| v.is_null(), |other| same_shape(v, other))
            }),
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same_shape(a, b))
            }
            (Value::Object(_) | Value::Array(_), _) | (_, Value::Object(_) | Value::Array(_)) => {
                false
            }
            _ => true,
        }
    }
    if !value.is_object() {
        return Err(error(code, "Expected JSON object"));
    }
    let decoded: T =
        serde_json::from_value(value.clone()).map_err(|e| error(code, e.to_string()))?;
    let encoded = serde_json::to_value(&decoded).map_err(|e| error(code, e.to_string()))?;
    if !same_shape(&value, &encoded) {
        return Err(error(code, "Object/array shape differs from wire schema"));
    }
    Ok(decoded)
}
