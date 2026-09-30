// SPDX-License-Identifier: Apache-2.0
#[path = "../src/inbox/classify.rs"]
mod classify;
use classify::{HumanAct, NextAct, Warrant, next_act};
use openwarrant_core::Phase;
macro_rules! state_case {
    ($name:ident, $phase:ident, $expected:ident) => {
        #[test]
        fn $name() {
            assert_eq!(
                next_act(&Warrant {
                    phase: Phase::$phase,
                    pending: None,
                    blocking_question: false
                }),
                NextAct::$expected
            );
        }
    };
}
state_case!(draft_needs_agent_preparation, Draft, Agent);
state_case!(proposed_without_valid_request_needs_agent, Proposed, Agent);
state_case!(authorized_needs_agent, Authorized, Agent);
state_case!(ready_needs_agent, Ready, Agent);
state_case!(executing_needs_agent, Executing, Agent);
state_case!(verifying_waits_for_gate, Verifying, Gate);
state_case!(resolved_without_new_act_is_absent, Resolved, None);
#[test]
fn human_requests_win_in_every_phase_and_blocking_answers_win_over_signing() {
    for phase in Phase::ALL {
        for act in [
            HumanAct::Answer,
            HumanAct::Authorize,
            HumanAct::Correct,
            HumanAct::Resolve,
        ] {
            let mut w = Warrant {
                phase: *phase,
                pending: Some(act),
                blocking_question: false,
            };
            assert_eq!(next_act(&w), NextAct::Human(act));
            w.blocking_question = true;
            assert_eq!(next_act(&w), NextAct::Human(HumanAct::Answer));
        }
    }
}
#[test]
fn conformance_table_covers_every_phase() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../conformance/fixtures/inbox/classifier.json"
    ))
    .unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), Phase::ALL.len());
    for case in cases {
        let phase: Phase = serde_json::from_value(case["phase"].clone()).unwrap();
        let actual = next_act(&Warrant {
            phase,
            pending: None,
            blocking_question: false,
        });
        let expected = match case["expected"].as_str().unwrap() {
            "agent" => NextAct::Agent,
            "gate" => NextAct::Gate,
            "none" => NextAct::None,
            _ => panic!("invalid fixture expectation"),
        };
        assert_eq!(actual, expected);
    }
}
