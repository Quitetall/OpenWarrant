// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::authority_transition::{
    Operation, PROPOSAL_SCHEMA, Principal, Proposal, REVISION_SCHEMA, Revision,
    authorize_transition,
};
use std::collections::{BTreeMap, BTreeSet};

const KEY: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
fn prior() -> Revision {
    Revision {
        schema: REVISION_SCHEMA.into(),
        repository: "example-project".into(),
        sequence: 0,
        principals: BTreeMap::from([(
            "alice".into(),
            Principal {
                public_key: KEY.into(),
                roles: BTreeSet::from(["authority-admin".into()]),
            },
        )]),
    }
}
fn proposal(previous: &Revision) -> Proposal {
    let mut next = previous.clone();
    next.sequence += 1;
    Proposal {
        schema: PROPOSAL_SCHEMA.into(),
        operation: Operation::Update,
        previous_digest: previous.digest().unwrap(),
        next,
    }
}
#[test]
fn prior_admin_can_rotate_but_new_admin_cannot_authorize_itself() {
    let previous = prior();
    let mut change = proposal(&previous);
    change.next.principals = BTreeMap::from([("bob".into(), previous.principals["alice"].clone())]);
    assert!(authorize_transition(&previous, &change, &BTreeSet::from(["alice".into()])).is_ok());
    assert_eq!(
        authorize_transition(&previous, &change, &BTreeSet::from(["bob".into()]))
            .unwrap_err()
            .code,
        "authority-signer"
    );
}
#[test]
fn sequence_cannot_alias_through_json_number_rounding() {
    let mut state = prior();
    state.sequence = 9_007_199_254_740_992;
    assert_eq!(state.digest().unwrap_err().code, "authority-sequence");
}
#[test]
fn recovery_uses_previous_recovery_role_not_proposed_policy() {
    let mut previous = prior();
    previous.principals.insert(
        "rescue".into(),
        Principal {
            public_key: KEY.into(),
            roles: BTreeSet::from(["authority-recovery".into()]),
        },
    );
    let mut change = proposal(&previous);
    change.operation = Operation::Recover;
    change.next.principals.remove("rescue");
    assert!(authorize_transition(&previous, &change, &BTreeSet::from(["rescue".into()])).is_ok());
    assert_eq!(
        authorize_transition(&previous, &change, &BTreeSet::from(["alice".into()]))
            .unwrap_err()
            .code,
        "authority-signer"
    );
    assert_eq!(
        authorize_transition(&previous, &change, &BTreeSet::new())
            .unwrap_err()
            .code,
        "authority-signer"
    );
}
#[test]
fn repository_parent_replay_and_admin_removal_refuse() {
    let previous = prior();
    for (field, expected) in [
        ("repository", "authority-repository"),
        ("parent", "authority-parent"),
        ("sequence", "authority-sequence"),
        ("admin", "authority-admin"),
    ] {
        let mut change = proposal(&previous);
        match field {
            "repository" => change.next.repository = "another".into(),
            "parent" => change.previous_digest = format!("sha256:{}", "0".repeat(64)),
            "sequence" => change.next.sequence = 0,
            _ => change
                .next
                .principals
                .get_mut("alice")
                .unwrap()
                .roles
                .clear(),
        }
        assert_eq!(
            change.validate_against(&previous).unwrap_err().code,
            expected
        );
    }
    let change = proposal(&previous);
    assert_eq!(
        change.validate_against(&change.next).unwrap_err().code,
        "authority-parent"
    );
}
#[test]
fn canonical_codec_rejects_unknown_duplicate_and_oversized_records() {
    let previous = prior();
    let change = proposal(&previous);
    let bytes = change.encode().unwrap();
    assert_eq!(Proposal::decode(&bytes).unwrap(), change);
    assert_eq!(
        Revision::decode(&previous.encode().unwrap()).unwrap(),
        previous
    );
    let mut extra = serde_json::to_value(&change).unwrap();
    extra["human"] = true.into();
    assert_eq!(
        Proposal::decode(&serde_jcs::to_vec(&extra).unwrap())
            .unwrap_err()
            .code,
        "authority-json"
    );
    let duplicate =
        String::from_utf8(bytes.clone())
            .unwrap()
            .replacen("{", "{\"operation\":\"update\",", 1);
    assert!(Proposal::decode(duplicate.as_bytes()).is_err());
    let mut padded = bytes;
    padded.push(b' ');
    assert_eq!(
        Proposal::decode(&padded).unwrap_err().code,
        "authority-canonical"
    );
    assert_eq!(
        Proposal::decode(&vec![b' '; 65_537]).unwrap_err().code,
        "authority-bounds"
    );
}
#[test]
fn key_principal_and_resource_controls_reject_unsafe_inputs() {
    for key in [
        "ssh-rsa AAAA",
        "ssh-ed25519 AAAA",
        &format!("{KEY} comment"),
        &KEY.replace("AAAAC3", "AAAAD3"),
    ] {
        let mut revision = prior();
        revision.principals.get_mut("alice").unwrap().public_key = key.into();
        assert_eq!(revision.validate().unwrap_err().code, "authority-key");
    }
    let mut revision = prior();
    let principal = revision.principals.remove("alice").unwrap();
    revision.principals.insert("alice\nroot".into(), principal);
    assert_eq!(revision.validate().unwrap_err().code, "authority-principal");
    let mut revision = prior();
    revision.repository = "x".repeat(129);
    assert_eq!(revision.validate().unwrap_err().code, "authority-bounds");
}
#[test]
fn changes_bind_exact_signing_bytes_and_distinct_digest_domains() {
    let previous = prior();
    let change = proposal(&previous);
    assert!(
        change
            .signing_bytes()
            .unwrap()
            .starts_with(b"openwarrant-authority-proposal-v1\0")
    );
    assert_ne!(previous.digest().unwrap(), change.digest().unwrap());
    let mut altered = change.clone();
    altered.operation = Operation::Recover;
    assert_ne!(
        change.signing_bytes().unwrap(),
        altered.signing_bytes().unwrap()
    );
    assert_ne!(change.digest().unwrap(), altered.digest().unwrap());
}
#[test]
fn transition_checks_wire_bound_even_for_direct_sdk_structs() {
    let previous = prior();
    let mut change = proposal(&previous);
    let roles = (0..32)
        .map(|n| format!("role{n:02}{}", "a".repeat(58)))
        .collect::<BTreeSet<_>>();
    for n in 0..127 {
        change.next.principals.insert(
            format!("person{n}"),
            Principal {
                public_key: KEY.into(),
                roles: roles.clone(),
            },
        );
    }
    assert_eq!(
        change.validate_against(&previous).unwrap_err().code,
        "authority-bounds"
    );
}
