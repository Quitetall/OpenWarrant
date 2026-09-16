use openwarrant_core::document::schedule::*;

fn digest(c: char) -> String {
    format!("sha256:{}", c.to_string().repeat(64))
}
fn node(stage: &str) -> WorkScope {
    WorkScope {
        warrant: "example:signup".into(),
        contract_digest: digest('a'),
        stage: Some(stage.into()),
    }
}
#[test]
fn dependent_stage_waits_while_independent_preparation_can_start() {
    let nodes = vec![node("api"), node("ui"), node("prepare")];
    let dependencies = vec![Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::ImplementationFinished,
        result_digest: Some(digest('b')),
        reason: "UI uses API result".into(),
    }];
    let report = evaluate_schedule(&nodes, &dependencies, &[], ScheduleLimits::default()).unwrap();
    assert_eq!(report.ready, vec![0, 2]);
    assert_eq!(
        report.waiting,
        vec![Waiting {
            dependency: 0,
            state: Satisfaction::Unknown
        }]
    );
    let facts = vec![ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::ImplementationFinished,
        result_digest: digest('b'),
        state: Satisfaction::Satisfied,
    }];
    let report =
        evaluate_schedule(&nodes, &dependencies, &facts, ScheduleLimits::default()).unwrap();
    assert_eq!(report.ready, vec![0, 1, 2]);
    assert!(report.waiting.is_empty());
}

#[test]
fn cycles_and_conflicting_facts_refuse_instead_of_returning_ready_work() {
    let nodes = vec![node("api"), node("ui")];
    let mut edges = vec![Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::ChecksPassed,
        result_digest: Some(digest('b')),
        reason: "checks".into(),
    }];
    let mut reverse = edges[0].clone();
    reverse.predecessor = 1;
    reverse.successor = 0;
    edges.push(reverse);
    assert_eq!(
        evaluate_schedule(&nodes, &edges, &[], ScheduleLimits::default()),
        Err(ScheduleError::Cycle)
    );
    edges.pop();
    let fact = ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::ChecksPassed,
        result_digest: digest('b'),
        state: Satisfaction::Satisfied,
    };
    let mut conflict = fact.clone();
    conflict.state = Satisfaction::Unmet;
    assert_eq!(
        evaluate_schedule(&nodes, &edges, &[fact, conflict], ScheduleLimits::default()),
        Err(ScheduleError::ConflictingFact)
    );
}

#[test]
fn estimates_are_optional_advisory_metadata_with_provenance() {
    let mut estimate = DifficultyEstimate {
        difficulty: Difficulty::High,
        reason: "Cross-repository contracts".into(),
        confidence: Confidence::Medium,
        estimator: "agent:planner".into(),
        revision: "estimate:1".into(),
    };
    assert_eq!(check_estimate(&estimate, 1024), Ok(()));
    let nodes = vec![node("api")];
    let before = evaluate_schedule(&nodes, &[], &[], ScheduleLimits::default()).unwrap();
    estimate.difficulty = Difficulty::Low;
    estimate.revision = "estimate:2".into();
    assert_eq!(check_estimate(&estimate, 1024), Ok(()));
    assert_eq!(
        evaluate_schedule(&nodes, &[], &[], ScheduleLimits::default()).unwrap(),
        before
    );
    estimate.reason.clear();
    assert_eq!(
        check_estimate(&estimate, 1024),
        Err(ScheduleError::InvalidInput)
    );
}

#[test]
fn facts_bind_exact_contract_stage_result_and_required_meaning() {
    let nodes = vec![node("api"), node("ui")];
    let edge = Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::HumanAccepted,
        result_digest: Some(digest('b')),
        reason: "Explicit human acceptance gate".into(),
    };
    let good = ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::HumanAccepted,
        result_digest: digest('b'),
        state: Satisfaction::Satisfied,
    };
    for changed in 0..4 {
        let mut stale = good.clone();
        match changed {
            0 => stale.scope.contract_digest = digest('c'),
            1 => stale.scope.stage = Some("other".into()),
            2 => stale.result_digest = digest('d'),
            _ => stale.requirement = RequiredResult::ImplementationFinished,
        }
        let report = evaluate_schedule(
            &nodes,
            std::slice::from_ref(&edge),
            &[stale],
            ScheduleLimits::default(),
        )
        .unwrap();
        assert_eq!(report.ready, vec![0]);
        assert_eq!(report.waiting[0].state, Satisfaction::Unknown);
    }
    let mut failed = good.clone();
    failed.state = Satisfaction::Unmet;
    let report = evaluate_schedule(
        &nodes,
        std::slice::from_ref(&edge),
        &[failed],
        ScheduleLimits::default(),
    )
    .unwrap();
    assert_eq!(report.waiting[0].state, Satisfaction::Unmet);
    assert_eq!(
        evaluate_schedule(&nodes, &[edge], &[good], ScheduleLimits::default())
            .unwrap()
            .ready,
        vec![0, 1]
    );
}

#[test]
fn malformed_duplicate_and_over_limit_inputs_never_return_partial_readiness() {
    let nodes = vec![node("a"), node("b")];
    let edge = Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::ContractPublished,
        result_digest: Some(digest('b')),
        reason: "published API".into(),
    };
    let defaults = ScheduleLimits::default();
    let tiny = ScheduleLimits {
        scopes: 1,
        ..defaults
    };
    assert_eq!(
        evaluate_schedule(&nodes, &[], &[], tiny),
        Err(ScheduleError::ResourceLimit)
    );
    assert_eq!(
        evaluate_schedule(
            &nodes,
            &[],
            &[],
            ScheduleLimits {
                text_bytes: 1,
                ..defaults
            }
        ),
        Err(ScheduleError::ResourceLimit)
    );
    assert_eq!(
        evaluate_schedule(&[nodes[0].clone(), nodes[0].clone()], &[], &[], defaults),
        Err(ScheduleError::DuplicateScope)
    );
    assert_eq!(
        evaluate_schedule(&nodes, &[edge.clone(), edge.clone()], &[], defaults),
        Err(ScheduleError::DuplicateDependency)
    );
    let mut bad = edge.clone();
    bad.successor = 2;
    assert_eq!(
        evaluate_schedule(&nodes, &[bad], &[], defaults),
        Err(ScheduleError::InvalidInput)
    );
    let mut bad = edge.clone();
    bad.result_digest = Some("sha256:no".into());
    assert_eq!(
        evaluate_schedule(&nodes, &[bad], &[], defaults),
        Err(ScheduleError::InvalidInput)
    );
    let mut self_edge = edge;
    self_edge.successor = 0;
    assert_eq!(
        evaluate_schedule(&nodes, &[self_edge], &[], defaults),
        Err(ScheduleError::Cycle)
    );
    let fact = ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::ChecksPassed,
        result_digest: digest('b'),
        state: Satisfaction::Satisfied,
    };
    assert_eq!(
        evaluate_schedule(&nodes, &[], &[fact.clone(), fact], defaults),
        Err(ScheduleError::DuplicateFact)
    );
}

#[test]
fn future_output_can_be_unpinned_but_competing_results_need_selection() {
    let nodes = vec![node("api"), node("ui")];
    let mut edge = Dependency {
        predecessor: 0,
        successor: 1,
        requirement: RequiredResult::ImplementationFinished,
        result_digest: None,
        reason: "API must exist".into(),
    };
    let first = ResultFact {
        scope: nodes[0].clone(),
        requirement: RequiredResult::ImplementationFinished,
        result_digest: digest('b'),
        state: Satisfaction::Satisfied,
    };
    let report = evaluate_schedule(
        &nodes,
        std::slice::from_ref(&edge),
        std::slice::from_ref(&first),
        ScheduleLimits::default(),
    )
    .unwrap();
    assert_eq!(report.ready, vec![0, 1]);
    assert_eq!(
        report.satisfied,
        vec![SatisfiedDependency {
            dependency: 0,
            fact: 0
        }]
    );
    let mut second = first.clone();
    second.result_digest = digest('c');
    let facts = vec![first, second];
    assert_eq!(
        evaluate_schedule(
            &nodes,
            std::slice::from_ref(&edge),
            &facts,
            ScheduleLimits::default()
        ),
        Err(ScheduleError::AmbiguousResult)
    );
    edge.result_digest = Some(digest('c'));
    assert_eq!(
        evaluate_schedule(&nodes, &[edge], &facts, ScheduleLimits::default())
            .unwrap()
            .satisfied,
        vec![SatisfiedDependency {
            dependency: 0,
            fact: 1
        }]
    );
}
