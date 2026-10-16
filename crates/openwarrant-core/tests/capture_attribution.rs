// SPDX-License-Identifier: Apache-2.0
//! Synthetic SDK controls; not cryptographic or operator qualification.
use openwarrant_core::{
    authority_transition::{Policy, Principal, REVISION_SCHEMA_V2, Revision},
    contract::ActorKind,
    document::runtime::RuntimeBinding,
    runtime_collector::attribution::{self, Context, Proof, SignedAttribution},
    runtime_collector::{
        Enrollment, Fault, NAMESPACE, Provider, SCHEMA, SignatureCheck, SignatureEvidence, Signed,
        Use,
    },
};
use std::collections::{BTreeMap, BTreeSet};
const KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
const WARRANT: &str = "01a0f502-4941-70a1-a446-e1eb77dff191";
fn fixture() -> (Revision, Signed) {
    let authority = Revision {
        schema: REVISION_SCHEMA_V2.into(),
        repository: "example".into(),
        sequence: 0,
        policy: Some(Policy::default()),
        principals: BTreeMap::from([
            (
                "owner".into(),
                Principal {
                    public_key: KEY.into(),
                    roles: BTreeSet::from(["authority-admin".into()]),
                    actor: Some("Owner".into()),
                    kind: Some(ActorKind::Human),
                },
            ),
            (
                "collector".into(),
                Principal {
                    public_key: KEY.into(),
                    roles: BTreeSet::from(["runtime-collector".into()]),
                    actor: Some("Collector".into()),
                    kind: Some(ActorKind::Agent),
                },
            ),
        ]),
    };
    let enrollment = Enrollment {
        schema: SCHEMA.into(),
        repository: "example".into(),
        authority_digest: authority.digest().unwrap(),
        provider: Provider {
            kind: "katana".into(),
            identity: "local-katana".into(),
            version: "katana/openwarrant-verification/v1".into(),
        },
        verifier_digest: format!("sha256:{}", "a".repeat(64)),
        collector: "collector".into(),
        warrants: BTreeSet::from([WARRANT.into()]),
    };
    (
        authority,
        Signed {
            enrollment,
            signatures: BTreeMap::from([("owner".into(), "synthetic-signature".into())]),
        },
    )
}
struct Synthetic {
    payload: Vec<u8>,
    evidence: Result<SignatureEvidence, Fault>,
}
impl SignatureCheck for Synthetic {
    fn verify(
        &self,
        key: &str,
        namespace: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault> {
        assert_eq!(key, KEY);
        assert_eq!(namespace, NAMESPACE);
        assert_eq!(signature, "synthetic-signature");
        if payload != self.payload {
            return Ok(SignatureEvidence {
                valid: false,
                user_present: None,
            });
        }
        self.evidence.clone()
    }
}
fn checker(signed: &Signed) -> Synthetic {
    Synthetic {
        payload: signed.enrollment.encode().unwrap(),
        evidence: Ok(SignatureEvidence {
            valid: true,
            user_present: None,
        }),
    }
}
fn usage(enrollment: &Enrollment) -> Use<'_> {
    Use {
        repository: "example",
        warrant: WARRANT,
        provider: &enrollment.provider,
        verifier_digest: &enrollment.verifier_digest,
        collector: &enrollment.collector,
    }
}

fn binding() -> RuntimeBinding {
    RuntimeBinding {
        warrant_ref: format!("war://{WARRANT}"),
        contract_digest: "contract-a".into(),
        dispatch_digest: "dispatch-a".into(),
        stage_id: "stage-a".into(),
        attempt_id: "attempt-a".into(),
    }
}
struct CaptureCheck {
    payload: Vec<u8>,
    valid: bool,
}
impl SignatureCheck for CaptureCheck {
    fn verify(
        &self,
        key: &str,
        namespace: &str,
        payload: &[u8],
        signature: &str,
    ) -> Result<SignatureEvidence, Fault> {
        assert_eq!(key, KEY);
        assert_eq!(namespace, attribution::NAMESPACE);
        Ok(SignatureEvidence {
            valid: self.valid && payload == self.payload && signature == "capture-test-signature",
            user_present: None,
        })
    }
}
#[test]
fn exact_attribution_survives_transport_but_substitutions_refuse() {
    let (authority, signed) = fixture();
    let enrollment = signed
        .authenticate(&authority, "example", &checker(&signed))
        .unwrap();
    let observed = usage(&signed.enrollment);
    let b = binding();
    let context = Context {
        authority: &authority,
        enrollment: &enrollment,
        usage: &observed,
        binding: &b,
        payload: b"actual retained capture bytes",
    };
    let proof = Proof::for_context(&context).unwrap();
    let check = CaptureCheck {
        payload: proof.encode().unwrap(),
        valid: true,
    };
    let value = SignedAttribution {
        proof,
        signature: "capture-test-signature".into(),
    };
    let wire = value.encode().unwrap();
    let decoded = SignedAttribution::decode(&wire).unwrap();
    assert_eq!(
        decoded.authenticate(&context, &check).unwrap().proof(),
        &value.proof
    );
    for field in [
        "collector",
        "authority_digest",
        "enrollment_digest",
        "warrant_ref",
        "contract_digest",
        "dispatch_digest",
        "stage_id",
        "attempt_id",
        "payload_digest",
    ] {
        let mut forged = serde_json::to_value(&value).unwrap();
        forged["proof"][field] = serde_json::json!("substitute");
        let altered: SignedAttribution = serde_json::from_value(forged).unwrap();
        assert!(altered.authenticate(&context, &check).is_err(), "{field}");
    }
    let changed = Context {
        payload: b"different bytes",
        ..context
    };
    assert!(decoded.authenticate(&changed, &check).is_err());
    let invalid = CaptureCheck {
        valid: false,
        ..check
    };
    assert!(decoded.authenticate(&context, &invalid).is_err());
}
#[test]
fn scope_and_revocation_cannot_be_bypassed_by_a_valid_signature_claim() {
    let (authority, signed) = fixture();
    let enrollment = signed
        .authenticate(&authority, "example", &checker(&signed))
        .unwrap();
    let mut observed = usage(&signed.enrollment);
    let mut b = binding();
    b.warrant_ref = "war://another-warrant".into();
    let bad = Context {
        authority: &authority,
        enrollment: &enrollment,
        usage: &observed,
        binding: &b,
        payload: b"capture",
    };
    assert!(Proof::for_context(&bad).is_err());
    b = binding();
    let context = Context {
        authority: &authority,
        enrollment: &enrollment,
        usage: &observed,
        binding: &b,
        payload: b"capture",
    };
    let proof = Proof::for_context(&context).unwrap();
    let check = CaptureCheck {
        payload: proof.encode().unwrap(),
        valid: true,
    };
    let value = SignedAttribution {
        proof,
        signature: "capture-test-signature".into(),
    };
    let mut revoked = authority.clone();
    revoked.sequence += 1;
    let changed = Context {
        authority: &revoked,
        ..context
    };
    assert!(value.authenticate(&changed, &check).is_err());
    observed.warrant = "01a0f502-4941-70a1-a446-e1eb77dff192";
    let outside = Context {
        authority: &authority,
        enrollment: &enrollment,
        usage: &observed,
        binding: &b,
        payload: b"capture",
    };
    assert!(value.authenticate(&outside, &check).is_err());
}
#[test]
fn malformed_unsupported_and_oversized_proofs_do_not_authenticate() {
    let (authority, signed) = fixture();
    let enrollment = signed
        .authenticate(&authority, "example", &checker(&signed))
        .unwrap();
    let observed = usage(&signed.enrollment);
    let b = binding();
    let context = Context {
        authority: &authority,
        enrollment: &enrollment,
        usage: &observed,
        binding: &b,
        payload: b"capture",
    };
    let proof = Proof::for_context(&context).unwrap();
    let value = SignedAttribution {
        proof,
        signature: "capture-test-signature".into(),
    };
    let wire = value.encode().unwrap();
    assert!(SignedAttribution::decode(b"{}").is_err());
    let mut duplicated = String::from_utf8(wire.clone()).unwrap();
    duplicated.insert_str(1, "\"signature\":\"duplicate\",");
    assert!(SignedAttribution::decode(duplicated.as_bytes()).is_err());
    let mut extra = serde_json::to_value(&value).unwrap();
    extra["assurance_granted"] = serde_json::json!(true);
    assert!(SignedAttribution::decode(&serde_jcs::to_vec(&extra).unwrap()).is_err());
    let mut future = value.clone();
    future.proof.schema = "oh.war/runtime-capture-attribution/v99".into();
    assert!(matches!(
        SignedAttribution::decode(&serde_jcs::to_vec(&future).unwrap()),
        Err(attribution::Fault::Unavailable(_))
    ));
    assert!(SignedAttribution::decode(&vec![b' '; 65537]).is_err());
    let bytes = vec![0; 6 * 1024 * 1024 + 1];
    let oversized = Context {
        payload: &bytes,
        ..context
    };
    assert!(Proof::for_context(&oversized).is_err());
}
