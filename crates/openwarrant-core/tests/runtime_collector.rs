// SPDX-License-Identifier: Apache-2.0
//! Synthetic trust-boundary controls, not OpenSSH or operator qualification.
use openwarrant_core::{
    authority_transition::{Policy, Principal, REVISION_SCHEMA_V2, Revision},
    contract::ActorKind,
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
#[test]
fn current_human_admin_can_enroll_only_exact_declared_inputs() {
    let (authority, signed) = fixture();
    let grant = signed
        .authenticate(&authority, "example", &checker(&signed))
        .unwrap();
    assert!(grant.allows(&authority, &usage(&signed.enrollment)).is_ok());
    let mut changed = signed.enrollment.clone();
    changed.verifier_digest = format!("sha256:{}", "b".repeat(64));
    assert!(grant.allows(&authority, &usage(&changed)).is_err());
    let mut use_record = usage(&signed.enrollment);
    use_record.warrant = "01a0f502-4941-70a1-a446-e1eb77dff192";
    assert!(grant.allows(&authority, &use_record).is_err());
}
#[test]
fn agent_self_grant_unsigned_and_changed_payload_refuse() {
    let (authority, signed) = fixture();
    let check = checker(&signed);
    let mut altered = signed.clone();
    altered.signatures = BTreeMap::from([("collector".into(), "synthetic-signature".into())]);
    assert!(altered.authenticate(&authority, "example", &check).is_err());
    altered.signatures.clear();
    assert!(altered.authenticate(&authority, "example", &check).is_err());
    altered = signed.clone();
    altered.enrollment.provider.identity = "substitute".into();
    assert!(altered.authenticate(&authority, "example", &check).is_err());
}
#[test]
fn authority_changes_invalidate_enrollment_and_already_authenticated_grant() {
    let (mut authority, signed) = fixture();
    let check = checker(&signed);
    let grant = signed.authenticate(&authority, "example", &check).unwrap();
    authority.sequence += 1;
    assert!(signed.authenticate(&authority, "example", &check).is_err());
    assert!(
        grant
            .allows(&authority, &usage(&signed.enrollment))
            .is_err()
    );
}
#[test]
fn missing_signature_observation_and_missing_required_presence_are_unknown() {
    let (mut authority, mut signed) = fixture();
    let mut check = checker(&signed);
    check.evidence = Err(Fault::Unavailable("signature verifier unavailable"));
    assert!(matches!(
        signed.authenticate(&authority, "example", &check),
        Err(Fault::Unavailable(_))
    ));
    authority.policy.as_mut().unwrap().require_user_presence = true;
    signed.enrollment.authority_digest = authority.digest().unwrap();
    assert!(matches!(
        signed.authenticate(&authority, "example", &checker(&signed)),
        Err(Fault::Unavailable(_))
    ));
    check = checker(&signed);
    check.evidence = Ok(SignatureEvidence {
        valid: true,
        user_present: Some(false),
    });
    assert!(matches!(
        signed.authenticate(&authority, "example", &check),
        Err(Fault::Rejected(_))
    ));
    check.evidence = Ok(SignatureEvidence {
        valid: true,
        user_present: Some(true),
    });
    assert!(signed.authenticate(&authority, "example", &check).is_ok());
}
#[test]
fn undeclared_signer_kind_is_unknown_but_a_known_agent_admin_refuses() {
    let (mut authority, mut signed) = fixture();
    let owner = authority.principals.get_mut("owner").unwrap();
    owner.kind = None;
    owner.actor = None;
    signed.enrollment.authority_digest = authority.digest().unwrap();
    assert!(matches!(
        signed.authenticate(&authority, "example", &checker(&signed)),
        Err(Fault::Unavailable(_))
    ));
    let owner = authority.principals.get_mut("owner").unwrap();
    owner.kind = Some(ActorKind::Agent);
    owner.actor = Some("Owner".into());
    signed.enrollment.authority_digest = authority.digest().unwrap();
    assert!(matches!(
        signed.authenticate(&authority, "example", &checker(&signed)),
        Err(Fault::Rejected(_))
    ));
}
#[test]
fn provider_identity_version_collector_and_repository_cannot_be_substituted() {
    let (authority, signed) = fixture();
    let grant = signed
        .authenticate(&authority, "example", &checker(&signed))
        .unwrap();
    for field in 0..3 {
        let mut changed = signed.enrollment.clone();
        match field {
            0 => changed.provider.kind = "blut".into(),
            1 => changed.provider.identity = "other".into(),
            _ => changed.provider.version = "other".into(),
        }
        assert!(grant.allows(&authority, &usage(&changed)).is_err());
    }
    let mut observed = usage(&signed.enrollment);
    observed.collector = "other";
    assert!(grant.allows(&authority, &observed).is_err());
    observed = usage(&signed.enrollment);
    observed.repository = "other";
    assert!(grant.allows(&authority, &observed).is_err());
    assert!(
        signed
            .authenticate(&authority, "other", &checker(&signed))
            .is_err()
    );
}
#[test]
fn canonical_wire_refuses_duplicates_unknown_fields_and_unbounded_inputs() {
    let (_, signed) = fixture();
    let bytes = signed.encode().unwrap();
    assert_eq!(Signed::decode(&bytes).unwrap(), signed);
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen("\"signatures\":", "\"signatures\":{},\"signatures\":", 1);
    assert!(Signed::decode(duplicate.as_bytes()).is_err());
    let unknown = text.replacen('{', "{\"assurance\":true,", 1);
    assert!(Signed::decode(unknown.as_bytes()).is_err());
    assert!(Signed::decode(&vec![b' '; 65537]).is_err());
}
