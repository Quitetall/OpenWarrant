// SPDX-License-Identifier: Apache-2.0
//! Caller-selected provider verification and exact runtime receipt matching.
//!
//! This candidate SDK interface defines no provider wire format or seal. The
//! trusted caller supplies the recorded dispatch, resolved policy facts and a
//! provider-owned verifier. Implementing that verifier by trusting JSON claims
//! does not authenticate a receipt. A result neither executes nor authorizes work.
use crate::{
    execution::StageDispatch,
    seam::{BlutLineageReceipt, KatanaReceipt, realized_within_authorization},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    Katana,
    Blut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProviderInterface {
    pub kind: ProviderKind,
    pub identity: String,
    pub version: String,
}

/// Returned only after the provider checks its own seal and native references.
/// These values must describe the same receipt bytes, not caller-supplied IDs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeBinding {
    pub warrant_ref: String,
    pub contract_digest: String,
    pub dispatch_digest: String,
    pub stage_id: String,
    pub attempt_id: String,
}

impl RuntimeBinding {
    /// The caller must obtain this dispatch from its checked recorded attempt.
    /// This projection does not validate its canonical digest or provenance.
    pub fn from_dispatch(dispatch: &StageDispatch) -> Self {
        Self {
            warrant_ref: dispatch.warrant_ref.clone(),
            contract_digest: dispatch.contract_digest.clone(),
            dispatch_digest: dispatch.dispatch_digest.clone(),
            stage_id: dispatch.stage_id.clone(),
            attempt_id: dispatch.attempt_id.clone(),
        }
    }

    fn complete(&self) -> bool {
        [
            &self.warrant_ref,
            &self.contract_digest,
            &self.dispatch_digest,
            &self.stage_id,
            &self.attempt_id,
        ]
        .iter()
        .all(|value| !value.trim().is_empty())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Established,
    Refuted,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuntimeOutcome {
    Completed,
    Failed,
    Halted,
    Cancelled,
    Running,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum NativeReceipt {
    Katana(KatanaReceipt),
    Blut(BlutLineageReceipt),
}

/// Facts established by the trusted provider adapter, not by the receipt's
/// claims. The adapter owns native terminal-status mapping, execution/log/job
/// observations, seal validation, confinement and accounting observations.
#[derive(Clone, Debug)]
pub struct VerifiedReceipt {
    pub interface: ProviderInterface,
    /// Ordinary raw-byte identity; deliberately distinct from provider seal.
    pub raw_digest: String,
    pub binding: RuntimeBinding,
    pub receipt: NativeReceipt,
    pub outcome: RuntimeOutcome,
    pub execution: Observation,
    pub confinement: Observation,
    pub metered_cost: Observation,
    pub spend_cap: Observation,
    /// The adapter checks the actual BLUT job's registry, not an input hint.
    pub registry_digest: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderFailure {
    Unsupported(String),
    Unavailable(String),
    Rejected(String),
}

/// Explicit provider boundary. No discovery, fallback, retry or model call is
/// performed by this module. The caller owns any I/O inside its chosen adapter.
/// `verify` must reject altered seals and mixed native references, and must not
/// reconstruct provider facts from an OpenWarrant transcript.
pub trait ReceiptVerifier {
    fn interface(&self) -> &ProviderInterface;
    fn verify(&self, raw_receipt: &[u8]) -> Result<VerifiedReceipt, ProviderFailure>;
}

/// Trusted caller facts for one checked recorded dispatch. The caller must
/// resolve capability and spend policy against that dispatch's policy identity;
/// these supplied facts are not authenticated by this pure matching function.
pub struct RuntimeExpectation<'a> {
    pub dispatch: &'a StageDispatch,
    pub provider: &'a ProviderInterface,
    /// Resolved from the actual BLUT lowering, not the returned receipt.
    pub registry_digest: Option<&'a str>,
    /// Caller-established capabilities for this exact dispatch. None means
    /// unresolved policy, not an unrestricted permission set.
    pub authorized_capabilities: Option<&'a [String]>,
    pub confinement_required: bool,
    pub hard_spend_cap_required: bool,
    pub max_receipt_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptStanding {
    Matches,
    Refused,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiptAssessment {
    pub standing: ReceiptStanding,
    pub code: &'static str,
    pub detail: String,
    pub raw_digest: Option<String>,
    pub provider_receipt_digest: Option<String>,
    /// Unknown remains visible even when policy permits an unmetered run.
    pub cost_observation: Observation,
}

fn assessment(
    standing: ReceiptStanding,
    code: &'static str,
    detail: impl Into<String>,
) -> ReceiptAssessment {
    ReceiptAssessment {
        standing,
        code,
        detail: detail.into(),
        raw_digest: None,
        provider_receipt_digest: None,
        cost_observation: Observation::Unknown,
    }
}

/// Match one provider-verified receipt to one recorded dispatch. This assesses
/// receipt eligibility only: no assurance mark, human acceptance or resolution.
pub fn assess_runtime_receipt(
    expected: &RuntimeExpectation<'_>,
    raw_receipt: &[u8],
    verifier: Option<&dyn ReceiptVerifier>,
) -> ReceiptAssessment {
    use ReceiptStanding::{Matches, Refused, Unknown};
    let binding = RuntimeBinding::from_dispatch(expected.dispatch);
    if !binding.complete()
        || expected.dispatch.validate(&[]).is_err()
        || expected.provider.identity.trim().is_empty()
        || expected.provider.version.trim().is_empty()
    {
        return assessment(
            Refused,
            "runtime.invalid-expectation",
            "incomplete dispatch or provider identity",
        );
    }
    if raw_receipt.is_empty() || raw_receipt.len() > expected.max_receipt_bytes {
        return assessment(
            Refused,
            "runtime.receipt-limit",
            "empty or oversized receipt",
        );
    }
    let Some(verifier) = verifier else {
        return assessment(
            Unknown,
            "runtime.verifier-unavailable",
            "no provider verification interface",
        );
    };
    if verifier.interface() != expected.provider {
        return assessment(
            Refused,
            "runtime.provider-mismatch",
            "configured provider interface differs",
        );
    }
    let checked = match verifier.verify(raw_receipt) {
        Ok(checked) => checked,
        Err(ProviderFailure::Unsupported(why)) => {
            return assessment(Unknown, "runtime.unsupported", why);
        }
        Err(ProviderFailure::Unavailable(why)) => {
            return assessment(Unknown, "runtime.unavailable", why);
        }
        Err(ProviderFailure::Rejected(why)) => {
            return assessment(Refused, "runtime.provider-rejected", why);
        }
    };
    if checked.interface != *expected.provider {
        return assessment(
            Refused,
            "runtime.provider-mismatch",
            "returned provider interface differs",
        );
    }
    let raw_digest = super::records::raw_digest(raw_receipt);
    if checked.raw_digest != raw_digest {
        return assessment(
            Refused,
            "runtime.raw-mismatch",
            "provider observation describes different raw bytes",
        );
    }
    if checked.binding != binding {
        return assessment(
            Refused,
            "runtime.binding-mismatch",
            "Warrant, contract, dispatch, stage or attempt differs",
        );
    }
    let provider_receipt_digest = match (&checked.receipt, expected.provider.kind) {
        (NativeReceipt::Katana(receipt), ProviderKind::Katana) => {
            if let Err(error) = receipt.validate(&binding.dispatch_digest) {
                return assessment(Refused, "runtime.invalid-native-receipt", error.to_string());
            }
            let Some(allowed) = expected.authorized_capabilities else {
                return assessment(
                    Unknown,
                    "runtime.capabilities-unknown",
                    "dispatch capability policy is unresolved",
                );
            };
            if let Err(error) =
                realized_within_authorization(allowed, &receipt.realized_capabilities)
            {
                return assessment(Refused, "runtime.capability-refused", error.to_string());
            }
            receipt.receipt_digest.clone()
        }
        (NativeReceipt::Blut(receipt), ProviderKind::Blut) => {
            if let Err(error) = receipt.validate() {
                return assessment(Refused, "runtime.invalid-native-receipt", error.to_string());
            }
            let Some(registry) = expected
                .registry_digest
                .filter(|value| !value.trim().is_empty())
            else {
                return assessment(
                    Unknown,
                    "runtime.registry-unknown",
                    "actual lowering registry is unresolved",
                );
            };
            match checked.registry_digest.as_deref() {
                None => {
                    return assessment(
                        Unknown,
                        "runtime.registry-unknown",
                        "provider did not establish the job registry",
                    );
                }
                Some(actual) if actual.trim().is_empty() => {
                    return assessment(
                        Unknown,
                        "runtime.registry-unknown",
                        "provider did not establish the job registry",
                    );
                }
                Some(actual) if actual != registry => {
                    return assessment(
                        Refused,
                        "runtime.registry-mismatch",
                        "job registry differs from actual lowering",
                    );
                }
                _ => {}
            }
            receipt.receipt_digest.clone()
        }
        _ => {
            return assessment(
                Refused,
                "runtime.provider-mismatch",
                "native receipt belongs to another provider kind",
            );
        }
    };
    match checked.outcome {
        RuntimeOutcome::Completed => {}
        RuntimeOutcome::Unknown => {
            return assessment(
                Unknown,
                "runtime.outcome-unknown",
                "native terminal outcome is unavailable",
            );
        }
        other => {
            return assessment(
                Refused,
                "runtime.not-completed",
                format!("native outcome is {other:?}"),
            );
        }
    }
    for (observed, required, code) in [
        (checked.execution, true, "runtime.execution-unestablished"),
        (
            checked.confinement,
            expected.confinement_required,
            "runtime.confinement-unestablished",
        ),
        (
            checked.metered_cost,
            expected.hard_spend_cap_required,
            "runtime.accounting-unestablished",
        ),
        (
            checked.spend_cap,
            expected.hard_spend_cap_required,
            "runtime.spend-cap-unestablished",
        ),
    ] {
        if required && observed != Observation::Established {
            return assessment(
                if observed == Observation::Refuted {
                    Refused
                } else {
                    Unknown
                },
                code,
                "required provider observation is not established",
            );
        }
    }
    let mut result = assessment(
        Matches,
        "runtime.matches",
        "provider-verified receipt matches this dispatch; no assurance is issued",
    );
    result.raw_digest = Some(raw_digest);
    result.provider_receipt_digest = Some(provider_receipt_digest);
    result.cost_observation = checked.metered_cost;
    result
}
