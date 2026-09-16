// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::records::*;
use serde_json::json;
fn hash(c: char) -> String {
    format!("sha256:{}", c.to_string().repeat(64))
}
fn subject() -> Subject {
    Subject {
        warrant: "test:work".into(),
        contract_digest: hash('a'),
        result_digest: Some(hash('b')),
    }
}
fn observation(verdict: &str, execution: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"schema":"oh.war/record/1.0.0-rc.2","id":"test:check","kind":"observation","subject":subject(),"actor":{"id":"test:checker","kind":"agent","role":"checker"},"policy_ref":null,"evidence_refs":[],"payload":{"check_id":"test:check","check_digest":hash('c'),"execution":execution,"verdict":verdict,"artifact_refs":[],"reason":"test fixture"},"provenance":{"source":"test:fixture","authenticity":"externally-verified"}})).unwrap()
}
#[test]
fn claimed_authenticity_is_not_trust_and_unavailable_is_not_pass() {
    let bytes = observation("UNKNOWN", "unavailable");
    let checked = check_records(
        &[bytes.as_slice()],
        &subject(),
        &[],
        RecordLimits::default(),
    )
    .unwrap();
    assert!(!checked.records()[0].authenticated());
    assert!(
        check_records(
            &[observation("PASS", "unavailable").as_slice()],
            &subject(),
            &[],
            RecordLimits::default()
        )
        .is_err()
    );
}
#[test]
fn exact_trust_binding_rejects_spoofed_actor_changed_bytes_and_duplicate_payload_keys() {
    let bytes = observation("PASS", "completed");
    let mut trust = TrustedRecord {
        id: "test:check".into(),
        raw_digest: raw_digest(&bytes),
        actor_id: "test:checker".into(),
        actor_kind: ActorKind::Agent,
        human_signed: false,
        observation_established: true,
        assurance_criteria: vec![],
        observed_at: Some(10),
    };
    assert!(
        check_records(
            &[&bytes],
            &subject(),
            &[trust.clone()],
            RecordLimits::default()
        )
        .unwrap()
        .records()[0]
            .authenticated()
    );
    trust.actor_kind = ActorKind::Human;
    assert_eq!(
        check_records(&[&bytes], &subject(), &[trust], RecordLimits::default())
            .unwrap_err()
            .code,
        "record.trust-mismatch"
    );
    let duplicate = String::from_utf8(bytes).unwrap().replace(
        "\"verdict\":\"PASS\"",
        "\"verdict\":\"FAIL\",\"verdict\":\"PASS\"",
    );
    assert!(
        check_records(
            &[duplicate.as_bytes()],
            &subject(),
            &[],
            RecordLimits::default()
        )
        .is_err()
    );
}
#[test]
fn explicit_action_gates_and_qualification_timing_stay_separate() {
    let bytes = observation("PASS", "completed");
    let trust = TrustedRecord {
        id: "test:check".into(),
        raw_digest: raw_digest(&bytes),
        actor_id: "test:checker".into(),
        actor_kind: ActorKind::Agent,
        human_signed: false,
        observation_established: true,
        assurance_criteria: vec![],
        observed_at: Some(20),
    };
    let checked = check_records(&[&bytes], &subject(), &[trust], RecordLimits::default()).unwrap();
    let mut condition = Condition {
        id: "test:before-work".into(),
        action: "execute".into(),
        stage: "build".into(),
        purpose: Purpose::Qualification,
        timing: Timing::Before(10),
        requirement: Requirement::PassingCheck {
            record_id: "test:check".into(),
            check_digest: hash('c'),
        },
    };
    let report = evaluate_readiness(
        &checked,
        std::slice::from_ref(&condition),
        "execute",
        "build",
        &[],
        RecordLimits::default(),
    )
    .unwrap();
    assert!(report.action_ready);
    assert_eq!(report.qualification[0].state, FindingState::Unmet);
    condition.purpose = Purpose::ActionGate;
    assert!(
        !evaluate_readiness(
            &checked,
            std::slice::from_ref(&condition),
            "execute",
            "build",
            &[],
            RecordLimits::default()
        )
        .unwrap()
        .action_ready
    );
    assert!(
        evaluate_readiness(
            &checked,
            &[condition],
            "execute",
            "prepare",
            &[],
            RecordLimits::default()
        )
        .unwrap()
        .action_ready
    );
}
#[test]
fn claims_and_missing_contract_never_establish_assurance() {
    let checked = check_records(&[], &subject(), &[], RecordLimits::default()).unwrap();
    let report = evaluate_assurance(
        &checked,
        None,
        "test:verification",
        "test:acceptance",
        &[],
        &[],
        RecordLimits::default(),
    )
    .unwrap();
    assert_eq!(report.standing, AssuranceStanding::Unknown);
    assert!(!report.issues_mark);
}
fn qualified_fixture() -> (Subject, Vec<Vec<u8>>, Vec<TrustedRecord>, Vec<Condition>) {
    use openwarrant_core::document::packet::{DigestDomain, structured_digest};
    let contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/sdk/records/contract.json"
    ))
    .unwrap();
    let mut sub = subject();
    sub.warrant = "example:signup".into();
    sub.contract_digest = structured_digest(DigestDomain::Contract, &contract, 100000).unwrap();
    let mut values = Vec::new();
    let obs =
        serde_json::from_slice::<serde_json::Value>(&observation("PASS", "completed")).unwrap();
    let mut add = |id: &str,
                   kind: &str,
                   actor: &str,
                   actor_kind: &str,
                   payload: serde_json::Value| {
        values.push(json!({"schema":"oh.war/record/1.0.0-rc.2","id":id,"kind":kind,"subject":sub,"actor":{"id":actor,"kind":actor_kind,"role":"fixture"},"policy_ref":null,"evidence_refs":[],"payload":payload,"provenance":{"source":"test:fixture","authenticity":"unverified"}}));
    };
    add(
        "test:check",
        "observation",
        "test:checker",
        "agent",
        obs["payload"].clone(),
    );
    add(
        "test:isolation",
        "observation",
        "test:harness",
        "service",
        obs["payload"].clone(),
    );
    add(
        "test:verification",
        "verification",
        "test:verifier",
        "agent",
        json!({"performer_id":"test:performer","candidate_digest":hash('b'),"obligations":[{"id":"OBL-1","scope":"signup behavior","disposition":"established","evidence_refs":["test:check"]}],"isolation_refs":["test:isolation"]}),
    );
    add(
        "test:acceptance",
        "human-acceptance",
        "test:human",
        "human",
        json!({"decision":"accept","reviewed":["outcome","verification","risks"],"meaning":"Fixture only, no real signature"}),
    );
    let mut conditions = Vec::new();
    for id in [
        "scope-permission",
        "protected-expectations",
        "adequacy",
        "preservation",
    ] {
        let record_id = format!("test:{id}");
        let mut payload = obs["payload"].clone();
        payload["check_id"] = json!(record_id);
        add(
            &record_id,
            "observation",
            "test:assessor",
            "service",
            payload,
        );
        conditions.push(Condition {
            id: id.into(),
            action: "qualify".into(),
            stage: "result".into(),
            purpose: Purpose::Qualification,
            timing: Timing::Any,
            requirement: Requirement::PassingCheck {
                record_id,
                check_digest: hash('c'),
            },
        });
    }
    let bytes: Vec<_> = values
        .iter()
        .map(|v| serde_json::to_vec(v).unwrap())
        .collect();
    let trust = values
        .iter()
        .zip(&bytes)
        .map(|(v, b)| TrustedRecord {
            id: v["id"].as_str().unwrap().into(),
            raw_digest: raw_digest(b),
            actor_id: v["actor"]["id"].as_str().unwrap().into(),
            actor_kind: if v["actor"]["kind"] == "human" {
                ActorKind::Human
            } else if v["actor"]["kind"] == "agent" {
                ActorKind::Agent
            } else {
                ActorKind::Service
            },
            human_signed: v["kind"] == "human-acceptance",
            observation_established: v["kind"] == "observation",
            assurance_criteria: if v["id"] == "test:isolation" {
                vec!["independence".into()]
            } else if [
                "test:scope-permission",
                "test:protected-expectations",
                "test:adequacy",
                "test:preservation",
            ]
            .contains(&v["id"].as_str().unwrap())
            {
                vec![
                    v["id"]
                        .as_str()
                        .unwrap()
                        .strip_prefix("test:")
                        .unwrap()
                        .into(),
                ]
            } else {
                vec![]
            },
            observed_at: Some(20),
        })
        .collect();
    (sub, bytes, trust, conditions)
}
#[test]
fn exact_qualification_requires_all_checks_independence_and_secure_human_act() {
    let (sub, bytes, trust, conditions) = qualified_fixture();
    let inputs: Vec<_> = bytes.iter().map(Vec::as_slice).collect();
    let checked = check_records(&inputs, &sub, &trust, RecordLimits::default()).unwrap();
    let contract = include_bytes!("../../../conformance/sdk/records/contract.json");
    let evaluate = |c: &CheckedRecords| {
        evaluate_assurance(
            c,
            Some(contract),
            "test:verification",
            "test:acceptance",
            &conditions,
            &[],
            RecordLimits::default(),
        )
        .unwrap()
    };
    assert_eq!(evaluate(&checked).standing, AssuranceStanding::Eligible);
    let mut no_human = trust.clone();
    no_human.iter_mut().for_each(|t| t.human_signed = false);
    assert_eq!(
        evaluate(&check_records(&inputs, &sub, &no_human, RecordLimits::default()).unwrap())
            .standing,
        AssuranceStanding::Unknown
    );
    let mut no_execution = trust.clone();
    no_execution
        .iter_mut()
        .for_each(|t| t.observation_established = false);
    assert_eq!(
        evaluate(&check_records(&inputs, &sub, &no_execution, RecordLimits::default()).unwrap())
            .standing,
        AssuranceStanding::Unknown
    );
    let mut changed = sub;
    changed.result_digest = Some(hash('d'));
    assert_eq!(
        check_records(&inputs, &changed, &trust, RecordLimits::default())
            .unwrap_err()
            .code,
        "record.subject"
    );
}
#[test]
fn unverified_agent_completion_is_valid_but_cannot_be_human_acceptance() {
    let mut v = json!({"schema":"oh.war/agent-act/1.0.0-rc.3","id":"test:complete","act":"complete","subject":subject(),"actor":{"id":"test:performer","kind":"agent","role":"performer"},"policy_ref":null,"meaning":"Prototype finished","evidence_refs":[],"signature_ref":null});
    let bytes = serde_json::to_vec(&v).unwrap();
    let decoded = decode_agent_act(&bytes, RecordLimits::default()).unwrap();
    assert!(!decoded.authenticated);
    assert!(!decoded.human_acceptance);
    v["actor"]["kind"] = json!("human");
    assert!(decode_agent_act(&serde_json::to_vec(&v).unwrap(), RecordLimits::default()).is_err());
    v["actor"]["kind"] = json!("agent");
    v["act"] = json!("authorize");
    assert!(decode_agent_act(&serde_json::to_vec(&v).unwrap(), RecordLimits::default()).is_err());
}
fn replace_record(
    bytes: &mut [Vec<u8>],
    trust: &mut [TrustedRecord],
    id: &str,
    edit: impl FnOnce(&mut serde_json::Value),
) {
    let i = trust.iter().position(|t| t.id == id).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes[i]).unwrap();
    edit(&mut v);
    bytes[i] = serde_json::to_vec(&v).unwrap();
    trust[i].raw_digest = raw_digest(&bytes[i]);
    trust[i].actor_id = v["actor"]["id"].as_str().unwrap().into();
}
#[test]
fn independent_controls_reject_claims_self_review_missing_isolation_and_wrong_contract() {
    let contract = include_bytes!("../../../conformance/sdk/records/contract.json");
    for control in 0..6 {
        let (mut sub, mut bytes, mut trust, mut conditions) = qualified_fixture();
        match control {
            0 => replace_record(&mut bytes, &mut trust, "test:verification", |v| {
                v["actor"]["id"] = json!("test:performer")
            }),
            1 => replace_record(&mut bytes, &mut trust, "test:verification", |v| {
                v["payload"]["isolation_refs"] = json!(["test:check"])
            }),
            2 => replace_record(&mut bytes, &mut trust, "test:check", |v| {
                v["payload"]["artifact_refs"] = json!(["test:missing-artifact"])
            }),
            3 => replace_record(&mut bytes, &mut trust, "test:check", |v| {
                v["kind"] = json!("claim");
                v["payload"] = json!({"statement":"Everything passed","scope":"signup behavior"});
            }),
            4 => {
                conditions[1].timing = Timing::Before(10);
            }
            _ => {
                sub.warrant = "example:other".into();
                for (b, t) in bytes.iter_mut().zip(&mut trust) {
                    let mut v: serde_json::Value = serde_json::from_slice(b).unwrap();
                    v["subject"] = json!(sub);
                    *b = serde_json::to_vec(&v).unwrap();
                    t.raw_digest = raw_digest(b);
                }
            }
        }
        let inputs: Vec<_> = bytes.iter().map(Vec::as_slice).collect();
        let checked = check_records(&inputs, &sub, &trust, RecordLimits::default()).unwrap();
        let result = evaluate_assurance(
            &checked,
            Some(contract),
            "test:verification",
            "test:acceptance",
            &conditions,
            &[],
            RecordLimits::default(),
        );
        if control == 5 {
            assert_eq!(result.unwrap_err().code, "contract.invalid");
        } else {
            assert_ne!(result.unwrap().standing, AssuranceStanding::Eligible);
        }
    }
}
#[test]
fn unknown_and_failure_remain_distinct_and_artifact_quota_applies() {
    for (verdict, execution, standing) in [
        ("FAIL", "completed", AssuranceStanding::Ineligible),
        ("UNKNOWN", "unavailable", AssuranceStanding::Unknown),
    ] {
        let (sub, mut bytes, mut trust, conditions) = qualified_fixture();
        replace_record(&mut bytes, &mut trust, "test:check", |v| {
            v["payload"]["verdict"] = json!(verdict);
            v["payload"]["execution"] = json!(execution);
        });
        let inputs: Vec<_> = bytes.iter().map(Vec::as_slice).collect();
        let checked = check_records(&inputs, &sub, &trust, RecordLimits::default()).unwrap();
        assert_eq!(
            evaluate_assurance(
                &checked,
                Some(include_bytes!(
                    "../../../conformance/sdk/records/contract.json"
                )),
                "test:verification",
                "test:acceptance",
                &conditions,
                &[],
                RecordLimits::default()
            )
            .unwrap()
            .standing,
            standing
        );
    }
    let mut v: serde_json::Value =
        serde_json::from_slice(&observation("PASS", "completed")).unwrap();
    v["payload"]["artifact_refs"] = json!(["test:a", "test:b"]);
    let b = serde_json::to_vec(&v).unwrap();
    assert_eq!(
        check_records(
            &[&b],
            &subject(),
            &[],
            RecordLimits {
                references: 1,
                ..RecordLimits::default()
            }
        )
        .unwrap_err()
        .code,
        "resource-limit"
    );
}
#[test]
fn permission_constraints_do_not_disappear_under_a_human_signature() {
    let v = json!({"schema":"oh.war/record/1.0.0-rc.2","id":"test:permission","kind":"permission","subject":subject(),"actor":{"id":"test:human","kind":"human","role":"owner"},"policy_ref":null,"evidence_refs":[],"payload":{"acts":["execute"],"constraints":{"stage":"release-only"},"effective_policy_digest":hash('d')},"provenance":{"source":"test:fixture","authenticity":"unverified"}});
    let bytes = serde_json::to_vec(&v).unwrap();
    let trust = TrustedRecord {
        id: "test:permission".into(),
        raw_digest: raw_digest(&bytes),
        actor_id: "test:human".into(),
        actor_kind: ActorKind::Human,
        human_signed: true,
        observation_established: false,
        assurance_criteria: vec![],
        observed_at: None,
    };
    let checked = check_records(&[&bytes], &subject(), &[trust], RecordLimits::default()).unwrap();
    let condition = Condition {
        id: "permission".into(),
        action: "execute".into(),
        stage: "build".into(),
        purpose: Purpose::ActionGate,
        timing: Timing::Any,
        requirement: Requirement::Permission {
            record_id: "test:permission".into(),
            act: Act::Execute,
        },
    };
    let report = evaluate_readiness(
        &checked,
        &[condition],
        "execute",
        "build",
        &[],
        RecordLimits::default(),
    )
    .unwrap();
    assert!(!report.action_ready);
    assert_eq!(report.action_gates[0].state, FindingState::Unknown);
}
#[test]
fn delegated_adoption_needs_human_policy_and_cannot_edit_effective_policy() {
    let make = |id: &str,
                kind: &str,
                actor: &str,
                actor_kind: &str,
                policy: serde_json::Value,
                payload: serde_json::Value| {
        serde_json::to_vec(&json!({"schema":"oh.war/record/1.0.0-rc.2","id":id,"kind":kind,"subject":subject(),"actor":{"id":actor,"kind":actor_kind,"role":"fixture"},"policy_ref":policy,"evidence_refs":[],"payload":payload,"provenance":{"source":"test:fixture","authenticity":"unverified"}})).unwrap()
    };
    let permission = make(
        "test:permission",
        "permission",
        "test:agent",
        "agent",
        json!("test:policy"),
        json!({"acts":["adopt-sas","edit-policy"],"constraints":{},"effective_policy_digest":hash('d')}),
    );
    let policy = make(
        "test:policy",
        "policy-disposition",
        "test:human",
        "human",
        json!(null),
        json!({"decision":"approve policy","limitations":[]}),
    );
    let mut trust = vec![
        TrustedRecord {
            id: "test:permission".into(),
            raw_digest: raw_digest(&permission),
            actor_id: "test:agent".into(),
            actor_kind: ActorKind::Agent,
            human_signed: false,
            observation_established: false,
            assurance_criteria: vec![],
            observed_at: Some(10),
        },
        TrustedRecord {
            id: "test:policy".into(),
            raw_digest: raw_digest(&policy),
            actor_id: "test:human".into(),
            actor_kind: ActorKind::Human,
            human_signed: true,
            observation_established: false,
            assurance_criteria: vec![],
            observed_at: Some(5),
        },
    ];
    let policies = vec![PolicyAuthority {
        record_id: "test:policy".into(),
        digest: hash('d'),
        acts: vec![Act::AdoptSas],
    }];
    let mut condition = Condition {
        id: "adoption".into(),
        action: "adopt".into(),
        stage: "architecture".into(),
        purpose: Purpose::ActionGate,
        timing: Timing::Any,
        requirement: Requirement::Permission {
            record_id: "test:permission".into(),
            act: Act::AdoptSas,
        },
    };
    let checked = check_records(
        &[&permission, &policy],
        &subject(),
        &trust,
        RecordLimits::default(),
    )
    .unwrap();
    assert!(
        evaluate_readiness(
            &checked,
            std::slice::from_ref(&condition),
            "adopt",
            "architecture",
            &policies,
            RecordLimits::default()
        )
        .unwrap()
        .action_ready
    );
    condition.requirement = Requirement::Permission {
        record_id: "test:permission".into(),
        act: Act::EditPolicy,
    };
    assert!(
        !evaluate_readiness(
            &checked,
            std::slice::from_ref(&condition),
            "adopt",
            "architecture",
            &policies,
            RecordLimits::default()
        )
        .unwrap()
        .action_ready
    );
    condition.requirement = Requirement::Permission {
        record_id: "test:permission".into(),
        act: Act::AdoptSas,
    };
    trust[1].human_signed = false;
    let checked = check_records(
        &[&permission, &policy],
        &subject(),
        &trust,
        RecordLimits::default(),
    )
    .unwrap();
    assert!(
        !evaluate_readiness(
            &checked,
            &[condition],
            "adopt",
            "architecture",
            &policies,
            RecordLimits::default()
        )
        .unwrap()
        .action_ready
    );
}
#[test]
fn unsigned_payload_digest_preserves_absent_fields_and_excludes_attestation() {
    let mut v = json!({"schema":"oh.war/agent-act/1.0.0-rc.3","id":"test:authorize","act":"authorize","subject":{"warrant":"test:work","contract_digest":hash('a')},"actor":{"id":"test:agent","kind":"agent","role":"performer"},"policy_ref":{"id":"test:policy","digest":hash('d')},"meaning":"Fixture delegation","evidence_refs":[],"signature_ref":null});
    let first =
        decode_agent_act(&serde_json::to_vec(&v).unwrap(), RecordLimits::default()).unwrap();
    let mut payload = v.clone();
    payload.as_object_mut().unwrap().remove("signature_ref");
    let expected = raw_digest(
        &serde_jcs::to_vec(
            &json!({"digest_domain":"oh.war/agent-act/1.0.0-rc.3","payload":payload}),
        )
        .unwrap(),
    );
    assert_eq!(first.unsigned_digest, expected);
    v["signature_ref"] = json!({"id":"test:sig","digest":hash('e')});
    assert_eq!(
        decode_agent_act(&serde_json::to_vec(&v).unwrap(), RecordLimits::default())
            .unwrap()
            .unsigned_digest,
        expected
    );
}

#[test]
fn agent_act_rejects_array_encoded_structs_without_panicking() {
    let d = format!("sha256:{}", "a".repeat(64));
    let subject = serde_json::json!({"warrant":"test:w","contract_digest":d,"result_digest":d});
    let actor = serde_json::json!({"id":"test:a","kind":"agent","role":"actor"});
    let value = serde_json::json!([
        "oh.war/agent-act/1.0.0-rc.3",
        "test:act",
        "complete",
        subject,
        actor,
        null,
        "done",
        [],
        null
    ]);
    assert!(
        decode_agent_act(
            &serde_json::to_vec(&value).unwrap(),
            RecordLimits::default()
        )
        .is_err()
    );
    let nested = serde_json::json!({"schema":"oh.war/agent-act/1.0.0-rc.3","id":"test:act","act":"complete","subject":["test:w",d,d],"actor":actor,"policy_ref":null,"meaning":"done","evidence_refs":[],"signature_ref":null});
    assert!(
        decode_agent_act(
            &serde_json::to_vec(&nested).unwrap(),
            RecordLimits::default()
        )
        .is_err()
    );
}

#[test]
fn explicit_null_optional_source_fields_remain_valid() {
    use openwarrant_core::document::packet::{DigestDomain, structured_digest};
    let mut contract: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/sdk/records/contract.json"
    ))
    .unwrap();
    contract["sources"][0]["holder"]["commit"] = serde_json::Value::Null;
    let mut sub = subject();
    sub.warrant = "example:signup".into();
    sub.contract_digest = structured_digest(DigestDomain::Contract, &contract, 100000).unwrap();
    let checked = check_records(&[], &sub, &[], RecordLimits::default()).unwrap();
    let bytes = serde_json::to_vec(&contract).unwrap();
    assert!(
        evaluate_assurance(
            &checked,
            Some(&bytes),
            "test:v",
            "test:a",
            &[],
            &[],
            RecordLimits::default()
        )
        .is_ok()
    );
}
