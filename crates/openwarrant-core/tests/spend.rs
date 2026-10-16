// SPDX-License-Identifier: Apache-2.0
//! Synthetic arithmetic controls, not provider/ledger durability qualification.
use openwarrant_core::spend::{ChargeBound as B, Fault, Policy, RunBudget, UsdMicros as U};
use openwarrant_core::spend::{Event, History, RecoveryFault};
#[test]
fn one_ceiling_covers_all_calls_retries_and_uncertain_outcomes() {
    let mut b = RunBudget::new(Policy::default());
    assert_eq!(
        b.reserve("estimate", B::Estimate(U(1))),
        Err(Fault::Unavailable)
    );
    assert_eq!(b.reserve("unknown", B::Unknown), Err(Fault::Unavailable));
    b.reserve("first", B::AuthoritativeMaximum(U(6_000_000)))
        .unwrap();
    assert_eq!(
        b.reserve("other-worker", B::AuthoritativeMaximum(U(5_000_000))),
        Err(Fault::Ceiling)
    );
    assert_eq!(b.settle("first", None), Err(Fault::Unavailable));
    assert_eq!(b.accounting().reserved_maximum, U(6_000_000));
    b.reserve("second", B::AuthoritativeMaximum(U(4_000_000)))
        .unwrap();
    assert_eq!(
        b.reserve("retry", B::AuthoritativeMaximum(U(1))),
        Err(Fault::Ceiling)
    );
    b.settle("first", Some(U(3_000_000))).unwrap();
    b.reserve("retry", B::AuthoritativeMaximum(U(3_000_000)))
        .unwrap();
    assert_eq!(
        b.reserve("first", B::AuthoritativeMaximum(U(0))),
        Err(Fault::Call)
    );
    assert_eq!(b.settle("first", Some(U(0))), Err(Fault::Call));
    assert_eq!(b.accounting().settled, U(3_000_000));
    assert_eq!(b.accounting().unsettled_calls, 2);
}
#[test]
fn actual_overcharge_is_retained_and_blocks_every_further_call() {
    let mut b = RunBudget::new(Policy::default());
    b.reserve("first", B::AuthoritativeMaximum(U(1_000_000)))
        .unwrap();
    assert_eq!(b.settle("first", Some(U(2_000_000))), Err(Fault::Breached));
    assert_eq!(b.accounting().settled, U(2_000_000));
    assert!(b.accounting().guarantee_breached);
    assert_eq!(
        b.reserve("free-looking", B::AuthoritativeMaximum(U(0))),
        Err(Fault::Breached)
    );
}
#[test]
fn undefined_cap_requires_explicit_unknown_policy_and_preserves_missing_cost() {
    let mut denied = RunBudget::new(Policy {
        ceiling: None,
        allow_unknown: false,
    });
    assert_eq!(denied.reserve("a", B::Unknown), Err(Fault::Unavailable));
    let mut b = RunBudget::new(Policy {
        ceiling: None,
        allow_unknown: true,
    });
    b.reserve("a", B::Estimate(U(0))).unwrap();
    assert_eq!(b.accounting().settled, U(0));
    assert_eq!(b.accounting().unknown_bound_calls, 1);
    assert_eq!(b.settle("a", None), Err(Fault::Unavailable));
    assert_eq!(b.accounting().unsettled_calls, 1);
    b.settle("a", Some(U(25))).unwrap();
    assert_eq!(b.accounting().settled, U(25));
    assert_eq!(b.accounting().unknown_bound_calls, 0);
}
#[test]
fn overflow_is_not_wrapped_into_new_spending_permission() {
    let mut b = RunBudget::new(Policy {
        ceiling: None,
        allow_unknown: false,
    });
    b.reserve("a", B::AuthoritativeMaximum(U(u64::MAX)))
        .unwrap();
    assert_eq!(
        b.reserve("b", B::AuthoritativeMaximum(U(1))),
        Err(Fault::Ceiling)
    );
    b.settle("a", Some(U(u64::MAX))).unwrap();
    b.reserve("b", B::AuthoritativeMaximum(U(0))).unwrap();
    assert_eq!(b.settle("b", Some(U(1))), Err(Fault::Breached));
    assert_eq!(b.accounting().unsettled_calls, 1);
    assert_eq!(
        b.reserve("c", B::AuthoritativeMaximum(U(0))),
        Err(Fault::Breached)
    );
}

#[test]
fn recovery_keeps_unsettled_reservations_and_prevents_retry_regrant() {
    let policy = Policy::default();
    let mut original = RunBudget::new(policy);
    original
        .reserve("finished", B::AuthoritativeMaximum(U(6_000_000)))
        .unwrap();
    original.settle("finished", Some(U(4_000_000))).unwrap();
    original
        .reserve("disappeared-worker", B::AuthoritativeMaximum(U(6_000_000)))
        .unwrap();
    assert_eq!(
        original.settle("disappeared-worker", None),
        Err(Fault::Unavailable)
    );
    let history = original.history();
    assert_eq!(history.events.len(), 3);
    let mut resumed = RunBudget::restore(policy, &history).unwrap();
    assert_eq!(resumed.accounting(), original.accounting());
    assert_eq!(resumed.history(), history);
    assert_eq!(
        resumed.reserve("replacement", B::AuthoritativeMaximum(U(1))),
        Err(Fault::Ceiling)
    );
    assert_eq!(
        resumed.reserve("disappeared-worker", B::AuthoritativeMaximum(U(0))),
        Err(Fault::Call)
    );
    resumed
        .settle("disappeared-worker", Some(U(5_000_000)))
        .unwrap();
    resumed
        .reserve("replacement", B::AuthoritativeMaximum(U(1_000_000)))
        .unwrap();
}

#[test]
fn recovery_preserves_unknown_cost_and_rejects_changed_policy() {
    let policy = Policy {
        ceiling: None,
        allow_unknown: true,
    };
    let mut original = RunBudget::new(policy);
    original.reserve("unknown", B::Unknown).unwrap();
    let history = original.history();
    let resumed = RunBudget::restore(policy, &history).unwrap();
    assert_eq!(resumed.accounting().unknown_bound_calls, 1);
    assert_eq!(resumed.accounting().unsettled_calls, 1);
    assert!(matches!(
        RunBudget::restore(Policy::default(), &history),
        Err(RecoveryFault::PolicyChanged)
    ));
}

#[test]
fn recovery_rejects_duplicate_out_of_order_and_impossible_events() {
    let reserve = Event::Reserved {
        call: "a".into(),
        bound: B::AuthoritativeMaximum(U(5_000_000)),
    };
    let settle = Event::Settled {
        call: "a".into(),
        actual: U(1_000_000),
    };
    for (events, index, fault) in [
        (vec![settle.clone(), reserve.clone()], 0, Fault::Call),
        (vec![reserve.clone(), reserve.clone()], 1, Fault::Call),
        (
            vec![reserve.clone(), settle.clone(), settle],
            2,
            Fault::Call,
        ),
        (
            vec![Event::Reserved {
                call: "a".into(),
                bound: B::Estimate(U(0)),
            }],
            0,
            Fault::Unavailable,
        ),
        (
            vec![Event::Reserved {
                call: "a".into(),
                bound: B::AuthoritativeMaximum(U(10_000_001)),
            }],
            0,
            Fault::Ceiling,
        ),
        (
            vec![Event::Reserved {
                call: "".into(),
                bound: B::Unknown,
            }],
            0,
            Fault::Call,
        ),
    ] {
        let history = History {
            policy: Policy::default(),
            events,
        };
        assert_eq!(
            RunBudget::restore(history.policy, &history).unwrap_err(),
            RecoveryFault::InvalidEvent { index, fault }
        );
    }
}

#[test]
fn recovery_retains_poisoned_overcharge_and_overflow_instead_of_resetting_budget() {
    for (policy, first, bound, actual) in [
        (Policy::default(), U(0), U(1), U(2)),
        (
            Policy {
                ceiling: None,
                allow_unknown: false,
            },
            U(u64::MAX),
            U(0),
            U(1),
        ),
    ] {
        let mut original = RunBudget::new(policy);
        original
            .reserve("first", B::AuthoritativeMaximum(first))
            .unwrap();
        original.settle("first", Some(first)).unwrap();
        original
            .reserve("breach", B::AuthoritativeMaximum(bound))
            .unwrap();
        assert_eq!(
            original.settle("breach", Some(actual)),
            Err(Fault::Breached)
        );
        let history = original.history();
        assert_eq!(original.settle("breach", Some(U(0))), Err(Fault::Call));
        assert_eq!(original.history(), history);
        let mut resumed = RunBudget::restore(policy, &history).unwrap();
        assert_eq!(resumed.accounting(), original.accounting());
        assert_eq!(resumed.history(), history);
        assert_eq!(
            resumed.reserve("retry", B::AuthoritativeMaximum(U(0))),
            Err(Fault::Breached)
        );
        let mut invalid = history;
        invalid.events.push(Event::Reserved {
            call: "retry".into(),
            bound: B::AuthoritativeMaximum(U(0)),
        });
        assert!(matches!(
            RunBudget::restore(policy, &invalid),
            Err(RecoveryFault::InvalidEvent {
                fault: Fault::Breached,
                ..
            })
        ));
    }
}
