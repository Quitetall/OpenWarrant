// SPDX-License-Identifier: Apache-2.0
//! Synthetic arithmetic controls, not provider/ledger durability qualification.
use openwarrant_core::spend::{ChargeBound as B, Fault, Policy, RunBudget, UsdMicros as U};
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
