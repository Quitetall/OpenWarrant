// SPDX-License-Identifier: AGPL-3.0-or-later
//! Gate Definitions, qualification, and Gate Bindings (SAS §43, RQ-056).
//!
//! # Why a gate is an object and not a string
//!
//! §43.1 splits ownership: Knowledge Fabric owns the authoritative institutional
//! registry; OpenWarrant owns schemas, local candidate authoring, CLI inspection
//! and binding, and cached projections. §43.1 then states the distinction this
//! module exists to keep legible:
//!
//! > Repositories may hold local candidates. A candidate is not a qualified
//! > institutional gate.
//!
//! OW-WAR-0019's Intent records the cautionary case: in the parent project's
//! corpus, 23 of 94 declared gates invoked a tool, script, or crate that was not
//! in the tree. Those were strings someone typed, carried in prose, never
//! resolved. §43.4's qualification is what separates a gate shown to work from
//! one written down.
//!
//! # What qualification has to mean
//!
//! §43.4: "Qualification establishes that the gate detects declared fault
//! classes." A gate qualified with positive controls alone can be one that flags
//! everything; a gate qualified with negative controls alone can be one that
//! flags nothing. Both directions are required here, and every declared fault
//! class must have a recorded detection result — an undeclared blind spot is the
//! failure mode §43.2's `known_blind_spots` exists to make sayable.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum GateError {
    #[error(
        "gate {gate_id:?} declares no version; §43.2 requires one and §43.3 makes definitions immutable, so a fix is a new version"
    )]
    MissingVersion { gate_id: String },
    #[error("gate {gate_id:?} declares an empty gate_id")]
    MissingId { gate_id: String },
    #[error("unknown gate lifecycle {found:?}; §43.3 defines {known}")]
    UnknownLifecycle { found: String, known: String },
    #[error(
        "gate binding {binding:?} binds {gate_id}@{version}, whose lifecycle is \
         {lifecycle}. §43.4 permits binding only a qualified gate — an unqualified \
         gate has not been shown to detect anything"
    )]
    BindingUnqualified {
        binding: String,
        gate_id: String,
        version: String,
        lifecycle: GateLifecycle,
    },
    #[error(
        "gate binding {binding:?} cites {gate_id}@{version}, which is not in the \
         registry. A gate that cannot be resolved is a string, not a gate"
    )]
    BindingUnresolved {
        binding: String,
        gate_id: String,
        version: String,
    },
    #[error(
        "gate binding {binding:?} pins digest {pinned} for {gate_id}@{version}, but \
         the registered definition digests to {actual}"
    )]
    BindingDigestMismatch {
        binding: String,
        gate_id: String,
        version: String,
        pinned: String,
        actual: String,
    },
    #[error("gate binding {binding:?} binds no subject; §43.5 requires at least one")]
    BindingNoSubject { binding: String },
    #[error(
        "obligation {obligation:?} cites {uri}, which is not in the registry. This is \
         the defect the parent project shipped 23 times: a declared gate naming a \
         tool that is not in the tree"
    )]
    ObligationUnresolvedGate { obligation: String, uri: String },
    #[error("malformed gate URI {uri:?}; expected gate://<id>@<version>")]
    MalformedGateUri { uri: String },
    #[error(
        "gate {gate_id}@{version} claims qualification with no positive controls. \
         §43.4 requires the gate to be shown DETECTING a fault; without one it may \
         be a gate that never fires"
    )]
    QualificationNoPositiveControls { gate_id: String, version: String },
    #[error(
        "gate {gate_id}@{version} claims qualification with no negative controls. \
         Without a known-good input that the gate accepts, a gate that fails \
         everything qualifies identically to one that works"
    )]
    QualificationNoNegativeControls { gate_id: String, version: String },
    #[error(
        "gate {gate_id}@{version} declares fault class {fault_class:?} in its \
         fault_model but records no detection result for it. §43.4 qualification \
         establishes that the gate detects its DECLARED fault classes; an \
         undeclared gap belongs in known_blind_spots, stated"
    )]
    QualificationUndetectedFaultClass {
        gate_id: String,
        version: String,
        fault_class: String,
    },
    #[error("gate {gate_id}@{version} records a qualification with no qualifier (§43.4)")]
    QualificationNoQualifier { gate_id: String, version: String },
    #[error(
        "gate {gate_id}@{version} records qualification_digest {found:?}, which is not \
         a digest. A placeholder in a digest field renders as though it were real; \
         leave it empty until it can be computed"
    )]
    QualificationDigestNotADigest {
        gate_id: String,
        version: String,
        found: String,
    },
    #[error(
        "gate {gate_id}@{version} is lifecycle {lifecycle} but carries no \
         qualification record; §43.3 places `qualified` after `draft` for a reason"
    )]
    LifecycleWithoutQualification {
        gate_id: String,
        version: String,
        lifecycle: GateLifecycle,
    },
    #[error("duplicate gate definition {gate_id}@{version}; §43.3 definitions are immutable")]
    DuplicateDefinition { gate_id: String, version: String },
}

/// §43.3's lifecycle, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateLifecycle {
    Draft,
    Qualified,
    Active,
    Deprecated,
    Invalidated,
}

impl GateLifecycle {
    pub const ALL: [Self; 5] = [
        Self::Draft,
        Self::Qualified,
        Self::Active,
        Self::Deprecated,
        Self::Invalidated,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Qualified => "qualified",
            Self::Active => "active",
            Self::Deprecated => "deprecated",
            Self::Invalidated => "invalidated",
        }
    }

    /// Whether §43.4 permits a binding to this definition.
    ///
    /// `deprecated` is deliberately still bindable: §43.3 marks a gate on its way
    /// out, and refusing existing bindings outright would invalidate historical
    /// records that were correct when made. `invalidated` is not bindable — that
    /// state means the gate was found not to detect what it claimed.
    #[must_use]
    pub const fn is_bindable(self) -> bool {
        match self {
            Self::Qualified | Self::Active | Self::Deprecated => true,
            Self::Draft | Self::Invalidated => false,
        }
    }

    /// Whether §43.3 places this state at or past `qualified`.
    #[must_use]
    pub const fn implies_qualification(self) -> bool {
        match self {
            Self::Qualified | Self::Active | Self::Deprecated | Self::Invalidated => true,
            Self::Draft => false,
        }
    }
}

impl FromStr for GateLifecycle {
    type Err = GateError;
    fn from_str(s: &str) -> Result<Self, GateError> {
        Self::ALL
            .into_iter()
            .find(|l| l.as_str() == s)
            .ok_or_else(|| GateError::UnknownLifecycle {
                found: s.to_owned(),
                known: Self::ALL
                    .iter()
                    .map(|l| l.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            })
    }
}

impl fmt::Display for GateLifecycle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// §43.1 — whether this repository authored the gate locally or is projecting a
/// gate the institutional registry qualified.
///
/// This is the whole of OW-ADR-0005 in one field. A local candidate is usable and
/// is not institutional, and the record must not blur the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateProvenance {
    /// Authored here. §43.1: "A candidate is not a qualified institutional gate."
    #[default]
    LocalCandidate,
    /// Projected from the Knowledge Fabric registry (§43.1). Not reachable until
    /// OW-WAR-0028 delivers federation; representable now so that the local case
    /// is never the only case the schema can express.
    InstitutionalProjection,
}

impl fmt::Display for GateProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::LocalCandidate => "local candidate",
            Self::InstitutionalProjection => "institutional projection",
        })
    }
}

/// A recorded detection result from qualification (§43.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectionResult {
    /// The fault class this exercised, matching an entry in `fault_model`.
    pub fault_class: String,
    /// What was done to the input.
    pub mutation: String,
    /// Whether the gate flagged it.
    pub detected: bool,
}

/// §43.4's qualification record. Every field the SAS lists as SHALL is present
/// and validated; none of them is optional.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Qualification {
    /// Known-bad inputs the gate is expected to flag.
    #[serde(default)]
    pub positive_controls: Vec<String>,
    /// Known-good inputs the gate is expected to accept. Without these, a gate
    /// that rejects everything qualifies identically to one that works.
    #[serde(default)]
    pub negative_controls: Vec<String>,
    #[serde(default)]
    pub mutation_classes: Vec<String>,
    #[serde(default)]
    pub environments: Vec<String>,
    #[serde(default)]
    pub detection_results: Vec<DetectionResult>,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default)]
    pub qualifier: String,
    #[serde(default)]
    pub qualification_digest: String,
}

impl Qualification {
    /// Fault classes this qualification actually demonstrated detection for.
    #[must_use]
    pub fn detected_fault_classes(&self) -> BTreeSet<&str> {
        self.detection_results
            .iter()
            .filter(|r| r.detected)
            .map(|r| r.fault_class.as_str())
            .collect()
    }

    fn validate(
        &self,
        gate_id: &str,
        version: &str,
        fault_model: &[String],
    ) -> Result<(), GateError> {
        if self.positive_controls.is_empty() {
            return Err(GateError::QualificationNoPositiveControls {
                gate_id: gate_id.to_owned(),
                version: version.to_owned(),
            });
        }
        if self.negative_controls.is_empty() {
            return Err(GateError::QualificationNoNegativeControls {
                gate_id: gate_id.to_owned(),
                version: version.to_owned(),
            });
        }
        if self.qualifier.trim().is_empty() {
            return Err(GateError::QualificationNoQualifier {
                gate_id: gate_id.to_owned(),
                version: version.to_owned(),
            });
        }
        // Empty is honest — the digest is not computed yet. A non-digest that
        // LOOKS like one is not: `sha256:pending` renders in a report exactly
        // where a reader expects an integrity value.
        let d = self.qualification_digest.trim();
        if !d.is_empty() && !is_sha256(d) {
            return Err(GateError::QualificationDigestNotADigest {
                gate_id: gate_id.to_owned(),
                version: version.to_owned(),
                found: d.to_owned(),
            });
        }
        let detected = self.detected_fault_classes();
        for class in fault_model {
            if !detected.contains(class.as_str()) {
                return Err(GateError::QualificationUndetectedFaultClass {
                    gate_id: gate_id.to_owned(),
                    version: version.to_owned(),
                    fault_class: class.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Whether a string is a `sha256:` digest and not something shaped like one.
fn is_sha256(s: &str) -> bool {
    s.strip_prefix("sha256:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// §43.2's Gate Definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateDefinition {
    pub gate_id: String,
    pub version: String,
    #[serde(default)]
    pub digest: String,
    pub lifecycle: GateLifecycle,
    #[serde(default)]
    pub implementation_ref: String,
    #[serde(default)]
    pub input_kinds: Vec<String>,
    #[serde(default)]
    pub output_schema_ref: String,
    /// The fault classes this gate claims to detect. Qualification must show a
    /// detection result for every one.
    #[serde(default)]
    pub fault_model: Vec<String>,
    /// §43.2 — what it is known NOT to catch. Stating this is what keeps
    /// `fault_model` honest instead of aspirational.
    #[serde(default)]
    pub known_blind_spots: Vec<String>,
    #[serde(default)]
    pub qualification: Option<Qualification>,
    #[serde(default)]
    pub provenance: GateProvenance,
    /// §44.7 — the structured argument vector this gate runs. Preferred over a
    /// raw shell string, which §44.7 permits only through a gate that explicitly
    /// owns shell parsing.
    #[serde(default)]
    pub argv: Vec<String>,
    /// §44.8 — whether this gate changes state while measuring it. A mutating
    /// gate is quarantined from routine runs however completely it is declared.
    #[serde(default)]
    pub mutating: bool,
    /// Seconds before §44.2's `timeout`. A gate knows its own runtime better
    /// than the runner does, and a single global deadline is either too short
    /// for the slowest gate or too long to be a deadline at all.
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

impl GateDefinition {
    /// `<gate_id>@<version>` — the key a binding resolves against.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}@{}", self.gate_id, self.version)
    }

    pub fn validate(&self) -> Result<(), GateError> {
        if self.gate_id.trim().is_empty() {
            return Err(GateError::MissingId {
                gate_id: self.gate_id.clone(),
            });
        }
        if self.version.trim().is_empty() {
            return Err(GateError::MissingVersion {
                gate_id: self.gate_id.clone(),
            });
        }
        match (&self.qualification, self.lifecycle.implies_qualification()) {
            (Some(q), _) => q.validate(&self.gate_id, &self.version, &self.fault_model)?,
            (None, true) => {
                return Err(GateError::LifecycleWithoutQualification {
                    gate_id: self.gate_id.clone(),
                    version: self.version.clone(),
                    lifecycle: self.lifecycle,
                });
            }
            (None, false) => {}
        }
        Ok(())
    }
}

/// A gate reference as pinned by a binding (§43.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateRef {
    pub id: String,
    pub version: String,
    #[serde(default)]
    pub digest: String,
}

impl GateRef {
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}@{}", self.id, self.version)
    }

    /// Parse a `gate://<id>@<version>` URI as cited from obligation evidence.
    pub fn parse_uri(uri: &str) -> Result<Self, GateError> {
        let rest = uri
            .strip_prefix("gate://")
            .ok_or_else(|| GateError::MalformedGateUri {
                uri: uri.to_owned(),
            })?;
        let (id, version) = rest
            .split_once('@')
            .ok_or_else(|| GateError::MalformedGateUri {
                uri: uri.to_owned(),
            })?;
        if id.is_empty() || version.is_empty() {
            return Err(GateError::MalformedGateUri {
                uri: uri.to_owned(),
            });
        }
        Ok(Self {
            id: id.to_owned(),
            version: version.to_owned(),
            digest: String::new(),
        })
    }
}

/// §43.5's Gate Binding, digested under `oh.war/gate-binding/v1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateBinding {
    pub id: String,
    pub gate: GateRef,
    #[serde(default)]
    pub subjects: Vec<String>,
    #[serde(default)]
    pub fixtures: Vec<Fixture>,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
    #[serde(default)]
    pub pass_predicate: BTreeMap<String, String>,
    #[serde(default)]
    pub evidence_policy: EvidencePolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fixture {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub digest: String,
}

/// §43.5's evidence policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidencePolicy {
    #[serde(default)]
    pub producer: String,
    /// §43.5 shows this `false`. A performer grading its own work is the
    /// substitution §40.7 forbids, in gate form.
    #[serde(default)]
    pub performer_authored_report_admissible: bool,
}

impl Default for EvidencePolicy {
    fn default() -> Self {
        Self {
            producer: "gate_runner".to_owned(),
            performer_authored_report_admissible: false,
        }
    }
}

/// A local gate registry — §43.1's "local gate-candidate authoring" and "cached
/// registry projections", never the authoritative institutional registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GateRegistry {
    #[serde(default)]
    pub definitions: Vec<GateDefinition>,
}

impl GateRegistry {
    pub fn insert(&mut self, def: GateDefinition) -> Result<(), GateError> {
        def.validate()?;
        if self.definitions.iter().any(|d| d.key() == def.key()) {
            return Err(GateError::DuplicateDefinition {
                gate_id: def.gate_id,
                version: def.version,
            });
        }
        self.definitions.push(def);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&GateDefinition> {
        self.definitions.iter().find(|d| d.key() == key)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// §43.4 and §43.5: resolve the binding, then refuse it if the definition is
    /// not bindable.
    pub fn validate_binding(&self, binding: &GateBinding) -> Result<&GateDefinition, GateError> {
        if binding.subjects.is_empty() {
            return Err(GateError::BindingNoSubject {
                binding: binding.id.clone(),
            });
        }
        let key = binding.gate.key();
        let def = self.get(&key).ok_or_else(|| GateError::BindingUnresolved {
            binding: binding.id.clone(),
            gate_id: binding.gate.id.clone(),
            version: binding.gate.version.clone(),
        })?;
        if !binding.gate.digest.is_empty()
            && !def.digest.is_empty()
            && binding.gate.digest != def.digest
        {
            return Err(GateError::BindingDigestMismatch {
                binding: binding.id.clone(),
                gate_id: binding.gate.id.clone(),
                version: binding.gate.version.clone(),
                pinned: binding.gate.digest.clone(),
                actual: def.digest.clone(),
            });
        }
        if !def.lifecycle.is_bindable() {
            return Err(GateError::BindingUnqualified {
                binding: binding.id.clone(),
                gate_id: binding.gate.id.clone(),
                version: binding.gate.version.clone(),
                lifecycle: def.lifecycle,
            });
        }
        Ok(def)
    }

    /// OW-WAR-0019 OBL-003: an obligation citing a gate that does not exist is
    /// refused, by name.
    pub fn resolve_citation(
        &self,
        obligation: &str,
        uri: &str,
    ) -> Result<&GateDefinition, GateError> {
        let r = GateRef::parse_uri(uri)?;
        self.get(&r.key())
            .ok_or_else(|| GateError::ObligationUnresolvedGate {
                obligation: obligation.to_owned(),
                uri: uri.to_owned(),
            })
    }
}

/// Parse one Gate Definition from a restricted-reader document.
///
/// # Why the keys are flat
///
/// OW-ADR-0003 chose a restricted reader over a YAML dependency. It reads
/// scalars, lists, and records — a list of flat maps — and nothing nested inside
/// a record. §43.4's qualification is therefore spelled with `qualification_`
/// prefixes rather than as a nested block. That is a cost of the ADR, paid
/// visibly here rather than by quietly adding a YAML parser to satisfy one file
/// format.
pub fn definition_from_structured(
    doc: &crate::structured::StructuredDoc,
) -> Result<GateDefinition, GateError> {
    let scalar = |k: &str| doc.scalar(k).unwrap_or_default().to_owned();
    let list = |k: &str| {
        doc.get(k)
            .and_then(StructuredValueExt::list)
            .unwrap_or_default()
    };

    let gate_id = scalar("gate_id");
    let version = scalar("version");

    let lifecycle = match doc.scalar("lifecycle") {
        Some(s) => GateLifecycle::from_str(s)?,
        None => GateLifecycle::Draft,
    };

    let detection_results = doc
        .records("detection_results")
        .unwrap_or_default()
        .iter()
        .map(|r| DetectionResult {
            fault_class: r
                .get("fault_class")
                .and_then(|v| v.as_scalar())
                .unwrap_or_default()
                .to_owned(),
            mutation: r
                .get("mutation")
                .and_then(|v| v.as_scalar())
                .unwrap_or_default()
                .to_owned(),
            // Anything that is not literally `true` is not a detection. A typo
            // must not read as success.
            detected: r
                .get("detected")
                .and_then(|v| v.as_scalar())
                .is_some_and(|s| s.eq_ignore_ascii_case("true")),
        })
        .collect::<Vec<_>>();

    let qualifier = scalar("qualification_qualifier");

    // A qualification exists if the author wrote ANY part of one. Keying this on
    // qualifier-plus-detection-results alone meant a gate carrying real positive
    // and negative controls, but no qualifier yet, parsed as `qualification:
    // None` — and a draft lifecycle then skipped validation entirely, silently
    // discarding the controls the author did provide. Absence has to mean the
    // author wrote nothing, not that they wrote the wrong two fields.
    let declared_any_qualification = QUALIFICATION_KEYS.iter().any(|k| doc.get(k).is_some());
    let qualification = if !declared_any_qualification {
        None
    } else {
        Some(Qualification {
            positive_controls: list("qualification_positive_controls"),
            negative_controls: list("qualification_negative_controls"),
            mutation_classes: list("qualification_mutation_classes"),
            environments: list("qualification_environments"),
            detection_results,
            limitations: list("qualification_limitations"),
            qualifier,
            qualification_digest: scalar("qualification_digest"),
        })
    };

    let provenance = match doc.scalar("provenance") {
        Some("institutional_projection") => GateProvenance::InstitutionalProjection,
        _ => GateProvenance::LocalCandidate,
    };

    let def = GateDefinition {
        gate_id,
        version,
        digest: scalar("digest"),
        lifecycle,
        implementation_ref: scalar("implementation_ref"),
        input_kinds: list("input_kinds"),
        output_schema_ref: scalar("output_schema_ref"),
        fault_model: list("fault_model"),
        known_blind_spots: list("known_blind_spots"),
        qualification,
        provenance,
        argv: list("argv"),
        mutating: doc
            .scalar("mutating")
            .is_some_and(|s| s.eq_ignore_ascii_case("true")),
        timeout_secs: doc
            .scalar("timeout_secs")
            .and_then(|s| s.trim().parse().ok()),
    };
    def.validate()?;
    Ok(def)
}

/// Every key that contributes to a §43.4 qualification record. If any one of
/// them is present the author is declaring a qualification, however partial, and
/// it gets validated rather than dropped.
const QUALIFICATION_KEYS: [&str; 8] = [
    "qualification_qualifier",
    "qualification_positive_controls",
    "qualification_negative_controls",
    "qualification_mutation_classes",
    "qualification_environments",
    "qualification_limitations",
    "qualification_digest",
    "detection_results",
];

/// Small helper so `definition_from_structured` can ask a value for a list
/// without importing the reader's whole surface.
trait StructuredValueExt {
    fn list(&self) -> Option<Vec<String>>;
}

impl StructuredValueExt for crate::structured::StructuredValue {
    fn list(&self) -> Option<Vec<String>> {
        self.as_list().map(<[String]>::to_vec)
    }
}

/// Extract an obligation's gate citations from `- **gate:**` bullets.
///
/// # Why not scan the prose
///
/// The first version of this searched the whole assurance atom for `gate://`.
/// It immediately flagged OW-WAR-0019's own sentence — "evidence: a plant citing
/// `gate://does-not-exist`, refused by name" — which describes a plant rather
/// than citing a gate.
///
/// That is not a tuning problem. A gate identified by pattern-matching prose IS
/// the "string, not a gate" failure §43 exists to end; §43.5 makes a binding an
/// object with a subject and a pinned digest, not a phrase someone wrote. So a
/// citation is a declared field, in the same `- **key:**` form the surrounding
/// obligations already use for scope and evidence, and prose that merely
/// mentions a URI is prose.
#[must_use]
pub fn cited_gate_uris(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed
            .strip_prefix("- **gate:**")
            .or_else(|| trimmed.strip_prefix("- **gates:**"))
        else {
            continue;
        };
        // Split on commas AND whitespace. A URI cannot contain either, so
        // accepting both costs nothing and avoids reading
        // `- **gate:** gate://a@1 gate://b@1` as one unresolvable token.
        for token in rest.split([',', ' ', '\t']) {
            let uri = token.trim().trim_matches('`').trim_end_matches('.').trim();
            if uri.is_empty() {
                continue;
            }
            if !out.iter().any(|u| u == uri) {
                out.push(uri.to_owned());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_qualification() -> Qualification {
        Qualification {
            positive_controls: vec!["a byte-flipped payload".into()],
            negative_controls: vec!["the unmodified golden vector".into()],
            mutation_classes: vec!["bitflip".into()],
            environments: vec!["linux-x86_64, rustc 1.97.1".into()],
            detection_results: vec![DetectionResult {
                fault_class: "byte-inequality".into(),
                mutation: "flipped one byte of the encoded stream".into(),
                detected: true,
            }],
            limitations: vec!["single-byte mutations only".into()],
            qualifier: "QuiteTall".into(),
            // A real 64-hex digest. The first version of this fixture said
            // "sha256:0", and the placeholder rule caught it the moment it landed.
            qualification_digest: format!("sha256:{}", "0".repeat(64)),
        }
    }

    fn good_definition() -> GateDefinition {
        GateDefinition {
            gate_id: "software.codec.byte-identity".into(),
            version: "4.0.0".into(),
            digest: "sha256:abc".into(),
            lifecycle: GateLifecycle::Active,
            implementation_ref: "artifact://byte-identity".into(),
            input_kinds: vec!["encoded-stream".into()],
            output_schema_ref: "schema://gate-result/v1".into(),
            fault_model: vec!["byte-inequality".into()],
            known_blind_spots: vec!["does not detect timing differences".into()],
            qualification: Some(good_qualification()),
            provenance: GateProvenance::LocalCandidate,
            argv: vec!["true".into()],
            mutating: false,
            timeout_secs: None,
        }
    }

    fn binding_to(def: &GateDefinition) -> GateBinding {
        GateBinding {
            id: "GB-001".into(),
            gate: GateRef {
                id: def.gate_id.clone(),
                version: def.version.clone(),
                digest: def.digest.clone(),
            },
            subjects: vec!["deliverable://DEL-001".into()],
            fixtures: vec![],
            parameters: BTreeMap::new(),
            pass_predicate: BTreeMap::from([("byte_equal".into(), "true".into())]),
            evidence_policy: EvidencePolicy::default(),
        }
    }

    #[test]
    fn a_well_formed_definition_validates_and_binds() {
        let def = good_definition();
        assert_eq!(def.validate(), Ok(()));
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("insert");
        assert!(reg.validate_binding(&binding_to(&def)).is_ok());
    }

    /// OBL-001 — §43.2 requires a version, and §43.3 makes definitions immutable
    /// so an unversioned one cannot be superseded.
    #[test]
    fn a_definition_without_a_version_is_refused() {
        let mut def = good_definition();
        def.version = String::new();
        assert!(matches!(
            def.validate(),
            Err(GateError::MissingVersion { .. })
        ));
    }

    /// OBL-002 — §43.4: unqualified cannot be bound.
    #[test]
    fn a_draft_gate_cannot_be_bound() {
        let mut def = good_definition();
        def.lifecycle = GateLifecycle::Draft;
        def.qualification = None;
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("a draft may exist");
        let err = reg.validate_binding(&binding_to(&def)).unwrap_err();
        assert!(matches!(err, GateError::BindingUnqualified { .. }), "{err}");
    }

    /// An invalidated gate was found NOT to detect what it claimed. Binding it
    /// would be worse than binding a draft.
    #[test]
    fn an_invalidated_gate_cannot_be_bound() {
        let mut def = good_definition();
        def.lifecycle = GateLifecycle::Invalidated;
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("insert");
        assert!(matches!(
            reg.validate_binding(&binding_to(&def)),
            Err(GateError::BindingUnqualified { .. })
        ));
    }

    /// Deprecated stays bindable so historical records that were correct when
    /// made do not retroactively break.
    #[test]
    fn a_deprecated_gate_remains_bindable() {
        let mut def = good_definition();
        def.lifecycle = GateLifecycle::Deprecated;
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("insert");
        assert!(reg.validate_binding(&binding_to(&def)).is_ok());
    }

    /// OBL-003 — the failure the parent project shipped 23 times.
    #[test]
    fn an_obligation_citing_a_nonexistent_gate_is_refused_by_name() {
        let reg = GateRegistry::default();
        let err = reg
            .resolve_citation("OBL-001", "gate://does-not-exist@1.0.0")
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("does-not-exist"), "{msg}");
        assert!(
            msg.contains("OBL-001") || msg.contains("not in the registry"),
            "{msg}"
        );
    }

    #[test]
    fn a_binding_to_an_unregistered_gate_is_refused() {
        let reg = GateRegistry::default();
        let def = good_definition();
        assert!(matches!(
            reg.validate_binding(&binding_to(&def)),
            Err(GateError::BindingUnresolved { .. })
        ));
    }

    /// A qualification that only ever saw bad input cannot distinguish a working
    /// gate from one that rejects everything.
    #[test]
    fn qualification_without_negative_controls_is_refused() {
        let mut def = good_definition();
        def.qualification
            .as_mut()
            .unwrap()
            .negative_controls
            .clear();
        assert!(matches!(
            def.validate(),
            Err(GateError::QualificationNoNegativeControls { .. })
        ));
    }

    /// ...and one that only ever saw good input may never fire at all. This is
    /// the green-gate-that-compared-nothing failure, in qualification form.
    #[test]
    fn qualification_without_positive_controls_is_refused() {
        let mut def = good_definition();
        def.qualification
            .as_mut()
            .unwrap()
            .positive_controls
            .clear();
        assert!(matches!(
            def.validate(),
            Err(GateError::QualificationNoPositiveControls { .. })
        ));
    }

    /// §43.4 — qualification establishes detection of DECLARED fault classes.
    #[test]
    fn a_declared_fault_class_with_no_detection_result_is_refused() {
        let mut def = good_definition();
        def.fault_model.push("truncation".into());
        let err = def.validate().unwrap_err();
        assert!(
            matches!(err, GateError::QualificationUndetectedFaultClass { ref fault_class, .. } if fault_class == "truncation"),
            "{err}"
        );
    }

    /// A detection result that records `detected: false` is not detection.
    #[test]
    fn a_fault_class_that_was_not_detected_does_not_count() {
        let mut def = good_definition();
        def.qualification.as_mut().unwrap().detection_results[0].detected = false;
        assert!(matches!(
            def.validate(),
            Err(GateError::QualificationUndetectedFaultClass { .. })
        ));
    }

    #[test]
    fn a_qualified_lifecycle_without_a_qualification_record_is_refused() {
        let mut def = good_definition();
        def.qualification = None;
        assert!(matches!(
            def.validate(),
            Err(GateError::LifecycleWithoutQualification { .. })
        ));
    }

    #[test]
    fn a_binding_with_no_subject_is_refused() {
        let def = good_definition();
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("insert");
        let mut b = binding_to(&def);
        b.subjects.clear();
        assert!(matches!(
            reg.validate_binding(&b),
            Err(GateError::BindingNoSubject { .. })
        ));
    }

    /// A binding pins a digest so the definition cannot be swapped under it.
    #[test]
    fn a_binding_pinning_a_stale_digest_is_refused() {
        let def = good_definition();
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("insert");
        let mut b = binding_to(&def);
        b.gate.digest = "sha256:stale".into();
        assert!(matches!(
            reg.validate_binding(&b),
            Err(GateError::BindingDigestMismatch { .. })
        ));
    }

    /// §43.3 — definitions are immutable; a fix is a new version.
    #[test]
    fn a_duplicate_definition_is_refused_and_a_new_version_is_not() {
        let def = good_definition();
        let mut reg = GateRegistry::default();
        reg.insert(def.clone()).expect("first");
        assert!(matches!(
            reg.insert(def.clone()),
            Err(GateError::DuplicateDefinition { .. })
        ));
        let mut next = def;
        next.version = "4.0.1".into();
        assert_eq!(reg.insert(next), Ok(()));
        assert_eq!(reg.len(), 2);
    }

    #[test]
    fn gate_uris_parse_and_malformed_ones_are_named() {
        let r = GateRef::parse_uri("gate://software.codec.byte-identity@4.0.0").expect("parses");
        assert_eq!(r.id, "software.codec.byte-identity");
        assert_eq!(r.version, "4.0.0");
        for bad in [
            "gate://no-version",
            "gate://@1.0.0",
            "gate://x@",
            "http://x@1",
        ] {
            assert!(
                matches!(
                    GateRef::parse_uri(bad),
                    Err(GateError::MalformedGateUri { .. })
                ),
                "{bad} should not parse"
            );
        }
    }

    #[test]
    fn citations_are_read_from_declared_bullets() {
        let text = "### OBL-001 — something\n\
                    - **scope:** bounded.\n\
                    - **gate:** `gate://a.b@1.0.0`, gate://c.d@2.1.0\n\
                    - **evidence:** the run.\n";
        assert_eq!(
            cited_gate_uris(text),
            vec!["gate://a.b@1.0.0", "gate://c.d@2.1.0"]
        );
    }

    /// Regression: prose describing a plant is not a citation.
    ///
    /// OW-WAR-0019's own OBL-003 reads "a plant citing `gate://does-not-exist`,
    /// refused by name". Scanning prose flagged that sentence as an unresolved
    /// gate the first time this ran against the real corpus — a gate identified
    /// by pattern-matching prose is the "string, not a gate" failure §43 exists
    /// to end.
    #[test]
    fn prose_mentioning_a_gate_uri_is_not_a_citation() {
        for prose in [
            "- **evidence:** a plant citing `gate://does-not-exist`, refused by name.",
            "The parent project declared gate://software.fake@1.0.0 and never shipped it.",
            "- **evidence:** cargo xtask gate exit status",
        ] {
            assert!(
                cited_gate_uris(prose).is_empty(),
                "prose was read as a citation: {prose:?}"
            );
        }
    }

    /// Regression: controls without a qualifier must not vanish.
    ///
    /// Keyed on qualifier-plus-detection-results, a draft gate carrying real
    /// positive and negative controls parsed as `qualification: None` and skipped
    /// validation entirely — silently discarding what the author wrote.
    #[test]
    fn a_partial_qualification_is_validated_not_dropped() {
        let doc = crate::structured::parse(
            "gate_id: \"a.b\"\nversion: \"1.0.0\"\nlifecycle: \"draft\"\n\
             qualification_positive_controls: [\"a planted fault\"]\n\
             qualification_negative_controls: [\"the clean corpus\"]\n",
        )
        .expect("parses");
        let err = definition_from_structured(&doc)
            .expect_err("a qualification with no qualifier must be refused, not dropped");
        assert!(
            matches!(err, GateError::QualificationNoQualifier { .. }),
            "{err}"
        );
    }

    /// A gate that declares no qualification at all is still a legal draft.
    #[test]
    fn a_gate_declaring_no_qualification_at_all_is_a_legal_draft() {
        let doc = crate::structured::parse(
            "gate_id: \"a.b\"\nversion: \"1.0.0\"\nlifecycle: \"draft\"\n",
        )
        .expect("parses");
        let def = definition_from_structured(&doc).expect("a bare draft is legal");
        assert!(def.qualification.is_none());
        assert_eq!(def.lifecycle, GateLifecycle::Draft);
    }

    /// A placeholder in a digest field renders where a reader expects integrity.
    #[test]
    fn a_placeholder_qualification_digest_is_refused() {
        let mut def = good_definition();
        for fake in ["sha256:pending", "sha256:TBD", "pending", "sha256:abc"] {
            def.qualification.as_mut().unwrap().qualification_digest = fake.to_owned();
            assert!(
                matches!(
                    def.validate(),
                    Err(GateError::QualificationDigestNotADigest { .. })
                ),
                "{fake:?} was accepted as a digest"
            );
        }
        // Empty is honest: not computed yet.
        def.qualification.as_mut().unwrap().qualification_digest = String::new();
        assert_eq!(def.validate(), Ok(()));
        // A real one passes.
        def.qualification.as_mut().unwrap().qualification_digest =
            format!("sha256:{}", "a".repeat(64));
        assert_eq!(def.validate(), Ok(()));
    }

    /// Space-separated citations resolve; a URI contains neither commas nor
    /// spaces, so accepting both separators costs nothing.
    #[test]
    fn citations_split_on_whitespace_as_well_as_commas() {
        assert_eq!(
            cited_gate_uris("- **gate:** gate://a.b@1.0.0 gate://c.d@2.1.0"),
            vec!["gate://a.b@1.0.0", "gate://c.d@2.1.0"]
        );
    }

    /// §43.1 — the distinction that OW-ADR-0005 turns into a field.
    #[test]
    fn provenance_defaults_to_local_candidate() {
        assert_eq!(GateProvenance::default(), GateProvenance::LocalCandidate);
        assert_eq!(
            GateProvenance::LocalCandidate.to_string(),
            "local candidate"
        );
    }

    /// §43.3's lifecycle, transcribed as an external expectation.
    #[test]
    fn the_lifecycle_matches_the_sas() {
        assert_eq!(
            GateLifecycle::ALL
                .iter()
                .map(|l| l.as_str())
                .collect::<Vec<_>>(),
            ["draft", "qualified", "active", "deprecated", "invalidated"]
        );
    }

    /// §43.5 shows a performer-authored report as inadmissible by default.
    #[test]
    fn evidence_policy_defaults_refuse_performer_authored_reports() {
        let p = EvidencePolicy::default();
        assert!(!p.performer_authored_report_admissible);
        assert_eq!(p.producer, "gate_runner");
    }

    #[test]
    fn a_binding_round_trips_through_json() {
        let def = good_definition();
        let b = binding_to(&def);
        let s = serde_json::to_string(&b).expect("serialize");
        let back: GateBinding = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(b, back);
    }
}
