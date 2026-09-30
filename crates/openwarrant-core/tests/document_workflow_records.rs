// SPDX-License-Identifier: Apache-2.0
use openwarrant_core::document::records::{workflow::*, *};
use serde_json::json;
fn hash(c: char) -> String {
    format!("sha256:{}", c.to_string().repeat(64))
}
#[test]
fn completion_and_qualification_are_separate_in_context_views() {
    let input = json!({"schema":"oh.war/workflow-record/1.0.0-rc.3","id":"test:view","payload":{"kind":"context-view","value":{"source_digest":hash('a'),"document_maturity":"draft","work_state":"complete","qualification":"unverified","contract_origin":"code-derived"}}});
    let bytes = serde_json::to_vec(&input).unwrap();
    let record = decode_workflow_record(&bytes, RecordLimits::default()).unwrap();
    let WorkflowPayload::ContextView(v) = record.envelope().payload.clone() else {
        panic!()
    };
    assert_eq!(v.work_state, WorkState::Complete);
    assert_eq!(v.qualification, QualificationClaim::Unverified);
}
fn subject() -> Subject {
    Subject {
        warrant: "test:work".into(),
        contract_digest: hash('a'),
        result_digest: Some(hash('b')),
    }
}
fn decode(id: &str, kind: &str, value: serde_json::Value) -> CheckedWorkflowRecord {
    decode_workflow_record(
        &serde_json::to_vec(
            &json!({"schema":SCHEMA,"id":id,"payload":{"kind":kind,"value":value}}),
        )
        .unwrap(),
        RecordLimits::default(),
    )
    .unwrap()
}
#[test]
fn stale_overview_cannot_emit_safeword_or_undo_completed_work() {
    let stop = decode(
        "test:event",
        "stop",
        json!({"subject":subject(),"class":"work","scope":"feature","scope_id":"test:feature","parent_scope":"test:work","cause":"completed","work_state":"complete","worker_state":"stopped"}),
    );
    let overview = decode(
        "test:overview",
        "overview",
        json!({"tracker":"test:tracker","revision":hash('c'),"as_of_unix_ms":10,"completions":[]}),
    );
    let handoff = decode(
        "test:handoff",
        "handoff",
        json!({"event_id":"test:event","overview":{"id":"test:overview","digest":overview.raw_digest()},"overview_url":"http://localhost/progress","safeword":"DONE","qualification":"unverified","assurance_refs":[],"notes":"notes.md","document_trail":["result.md"],"next_steps":["Next stage"]}),
    );
    let trust = TrackerTrust {
        overview_url: "http://localhost/progress".into(),
        raw_digest: overview.raw_digest().into(),
        tracker: "test:tracker".into(),
        revision: hash('c'),
    };
    let result = render_handoff(
        &stop,
        &overview,
        &handoff,
        &trust,
        "DONE",
        ResponseStyle::Minimal,
        1024,
    )
    .unwrap();
    assert!(result.work_complete);
    assert!(result.response.is_none());
    assert_eq!(result.state, FindingState::Unknown);
}
#[test]
fn confirmed_feature_handoff_uses_pointers_without_completing_parent() {
    let stop = decode(
        "test:event",
        "stop",
        json!({"subject":subject(),"class":"work","scope":"feature","scope_id":"test:feature","parent_scope":"test:work","cause":"completed","work_state":"complete","worker_state":"stopped"}),
    );
    let overview = decode(
        "test:overview",
        "overview",
        json!({"tracker":"test:tracker","revision":hash('c'),"as_of_unix_ms":10,"completions":[{"event_id":"test:event","subject":subject(),"scope_id":"test:feature","qualification":"unverified","assurance_refs":[],"notes":"notes.md","document_trail":["result.md"],"next_steps":["Next stage"]}]}),
    );
    let handoff = decode(
        "test:handoff",
        "handoff",
        json!({"event_id":"test:event","overview":{"id":"test:overview","digest":overview.raw_digest()},"overview_url":"http://localhost/progress","safeword":"DONE","qualification":"unverified","assurance_refs":[],"notes":"notes.md","document_trail":["result.md"],"next_steps":["Next stage"]}),
    );
    let mut trust = TrackerTrust {
        overview_url: "http://localhost/progress".into(),
        raw_digest: overview.raw_digest().into(),
        tracker: "test:tracker".into(),
        revision: hash('c'),
    };
    let minimal = render_handoff(
        &stop,
        &overview,
        &handoff,
        &trust,
        "DONE",
        ResponseStyle::Minimal,
        1024,
    )
    .unwrap();
    assert_eq!(
        minimal.response.as_deref(),
        Some("DONE\nhttp://localhost/progress")
    );
    let rich = render_handoff(
        &stop,
        &overview,
        &handoff,
        &trust,
        "DONE",
        ResponseStyle::WithNotes,
        1024,
    )
    .unwrap();
    assert!(rich.response.unwrap().ends_with("notes.md"));
    assert!(
        render_handoff(
            &stop,
            &overview,
            &handoff,
            &trust,
            "DONE",
            ResponseStyle::Minimal,
            1
        )
        .is_err()
    );
    trust.overview_url = "http://wrong.invalid/progress".into();
    assert!(
        render_handoff(
            &stop,
            &overview,
            &handoff,
            &trust,
            "DONE",
            ResponseStyle::Minimal,
            1024
        )
        .unwrap()
        .response
        .is_none()
    );
    trust.overview_url = "http://localhost/progress".into();
    let changed_claim = decode(
        "test:handoff",
        "handoff",
        json!({"event_id":"test:event","overview":{"id":"test:overview","digest":overview.raw_digest()},"overview_url":"http://localhost/progress","safeword":"DONE","qualification":"unknown","assurance_refs":[],"notes":"notes.md","document_trail":["result.md"],"next_steps":["Next stage"]}),
    );
    assert!(
        render_handoff(
            &stop,
            &overview,
            &changed_claim,
            &trust,
            "DONE",
            ResponseStyle::Minimal,
            1024
        )
        .unwrap()
        .response
        .is_none()
    );
    trust.raw_digest = hash('e');
    assert!(
        render_handoff(
            &stop,
            &overview,
            &handoff,
            &trust,
            "DONE",
            ResponseStyle::Minimal,
            1024
        )
        .unwrap()
        .response
        .is_none()
    );
    let WorkflowPayload::Overview(v) = &overview.envelope().payload else {
        panic!()
    };
    assert_eq!(v.completions.len(), 1);
    assert_eq!(v.completions[0].scope_id, "test:feature");
    assert!(check_replay(&stop, &stop).unwrap());
    let changed = decode(
        "test:event",
        "stop",
        json!({"subject":subject(),"class":"harness","scope":"feature","scope_id":"test:feature","parent_scope":"test:work","cause":"timeout","work_state":"in-progress","worker_state":"unknown"}),
    );
    assert_eq!(
        check_replay(&stop, &changed).unwrap_err().code,
        "event.conflict"
    );
}
#[test]
fn shared_contract_mismatch_and_timeout_as_completion_refuse() {
    let input = |kind: &str, value: serde_json::Value| {
        serde_json::to_vec(
            &json!({"schema":SCHEMA,"id":"test:record","payload":{"kind":kind,"value":value}}),
        )
        .unwrap()
    };
    let participants = json!([{"project":"project:a","scope":"API","basis_digest":hash('b'),"contract_digest":hash('a')},{"project":"project:b","scope":"UI","basis_digest":hash('c'),"contract_digest":hash('a')}]);
    let good = input(
        "shared-contract",
        json!({"contract_digest":hash('a'),"participants":participants}),
    );
    assert!(decode_workflow_record(&good, RecordLimits::default()).is_ok());
    let mut bad: serde_json::Value = serde_json::from_slice(&good).unwrap();
    bad["payload"]["value"]["participants"][1]["contract_digest"] = json!(hash('d'));
    assert!(
        decode_workflow_record(&serde_json::to_vec(&bad).unwrap(), RecordLimits::default())
            .is_err()
    );
    let timeout = input(
        "stop",
        json!({"subject":subject(),"class":"work","scope":"warrant","scope_id":"test:work","parent_scope":null,"cause":"timeout","work_state":"complete","worker_state":"unknown"}),
    );
    assert!(decode_workflow_record(&timeout, RecordLimits::default()).is_err());
}
#[test]
fn evidence_loss_preserves_history_and_exact_duplicates_can_retain_support() {
    for state in ["present", "absent", "deleted", "restored"] {
        let record = decode(
            "test:availability",
            "evidence-availability",
            json!({"evidence":{"id":"test:evidence","digest":hash('b')},"state":state,"retained_digests":[hash('b')],"history_refs":["test:signature"],"affected_assurance_refs":["test:assurance"]}),
        );
        let mut facts = AvailabilityFacts {
            record_digest: record.raw_digest().into(),
            retained_exact_bytes: false,
            expected_history: vec!["test:signature".into()],
        };
        assert_eq!(
            evaluate_availability(&record, &facts).unwrap().support,
            FindingState::Unknown
        );
        facts.retained_exact_bytes = true;
        assert_eq!(
            evaluate_availability(&record, &facts).unwrap().support,
            FindingState::Established
        );
        facts.expected_history.push("test:earlier-signature".into());
        let report = evaluate_availability(&record, &facts).unwrap();
        assert_eq!(report.support, FindingState::Unmet);
        assert!(!report.history_preserved);
    }
}
#[test]
fn resume_requires_fencing_for_work_changes_and_next_tool_boundary_for_harness() {
    for class in ["work", "harness"] {
        let record = decode(
            "test:change",
            "work-change",
            json!({"class":class,"old_basis":hash('a'),"new_basis":hash('b'),"scope_id":"test:stage","apply_at":if class=="work"{"next-work-stop"}else{"next-tool-call"},"choice_ref":"test:choice","fence_ref":"test:fence","context_ref":"test:context","limits":["spend <= configured cap"]}),
        );
        let mut facts = ResumeFacts {
            change_digest: record.raw_digest().into(),
            choice_established: true,
            writers_fenced: false,
            context_available: true,
            within_limits: true,
            at_work_stop: false,
            before_next_tool: false,
        };
        assert_ne!(
            evaluate_resume(&record, &facts).unwrap().state,
            FindingState::Established
        );
        facts.before_next_tool = true;
        if class == "work" {
            facts.at_work_stop = true;
            assert_eq!(
                evaluate_resume(&record, &facts).unwrap().state,
                FindingState::Unknown
            );
            facts.writers_fenced = true;
        }
        assert_eq!(
            evaluate_resume(&record, &facts).unwrap().state,
            FindingState::Established
        );
        facts.context_available = false;
        assert_eq!(
            evaluate_resume(&record, &facts).unwrap().state,
            FindingState::Unknown
        );
    }
}
#[test]
fn batch_acceptance_binds_exact_manifest_member_profile_and_evidence() {
    let member = ReviewMember {
        subject: subject(),
        profile: ExactReference {
            id: "test:profile".into(),
            digest: hash('c'),
        },
        evidence: vec![ExactReference {
            id: "test:checks".into(),
            digest: hash('d'),
        }],
    };
    let manifest = decode(
        "test:release",
        "review-manifest",
        json!({"members":[member]}),
    );
    let mut trust = HumanReviewTrust {
        manifest_digest: manifest.raw_digest().into(),
        actor_id: "test:human".into(),
        actor_kind: ActorKind::Human,
        secure_human_act: true,
        reviewed_outcome_evidence_risks: true,
    };
    assert_eq!(
        batch_acceptance(&manifest, &member, &trust).unwrap().state,
        FindingState::Established
    );
    let mut changed = member.clone();
    changed.subject.result_digest = Some(hash('e'));
    assert_eq!(
        batch_acceptance(&manifest, &changed, &trust).unwrap().state,
        FindingState::Unmet
    );
    trust.actor_kind = ActorKind::Agent;
    assert_eq!(
        batch_acceptance(&manifest, &member, &trust).unwrap().state,
        FindingState::Unknown
    );
    trust.actor_kind = ActorKind::Human;
    trust.manifest_digest = hash('f');
    assert_eq!(
        batch_acceptance(&manifest, &member, &trust).unwrap().state,
        FindingState::Unknown
    );
}

#[test]
fn stop_states_and_scalar_reference_limits_remain_explicit() {
    for state in ["blocked", "failed", "cancelled", "unknown"] {
        let record = decode(
            "test:event",
            "stop",
            json!({"subject":subject(),"class":"harness","scope":"warrant","scope_id":"test:work","cause":"unknown","work_state":state,"worker_state":"stop-requested"}),
        );
        assert!(matches!(
            record.envelope().payload,
            WorkflowPayload::Stop(_)
        ));
    }
    let value = json!({"schema":SCHEMA,"id":"test:change","payload":{"kind":"work-change","value":{"class":"work","old_basis":hash('a'),"new_basis":hash('b'),"scope_id":"test:work","apply_at":"stop-now","choice_ref":"test:choice","fence_ref":"test:fence","context_ref":"test:context","limits":[]}}});
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        decode_workflow_record(
            &bytes,
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
