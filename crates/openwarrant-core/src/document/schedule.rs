// SPDX-License-Identifier: Apache-2.0
//! Dependency readiness over supplied facts. No authentication, dispatch or assurance.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// An exact Warrant contract and optional bounded stage/milestone.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WorkScope {
    pub warrant: String,
    pub contract_digest: String,
    pub stage: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RequiredResult {
    ImplementationFinished,
    ChecksPassed,
    HumanAccepted,
    ContractPublished,
}

/// An action prerequisite; qualification-only conditions do not belong here.
#[derive(Clone, Debug)]
pub struct Dependency {
    pub predecessor: usize,
    pub successor: usize,
    pub requirement: RequiredResult,
    /// Optional expected output. None permits future output, but never ambiguous versions.
    pub result_digest: Option<String>,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Satisfaction {
    Satisfied,
    Unmet,
    Unknown,
}

/// The caller establishes authenticity and admissibility before supplying facts.
/// This type does not turn an actor's completion claim into verification evidence.
#[derive(Clone, Debug)]
pub struct ResultFact {
    pub scope: WorkScope,
    pub requirement: RequiredResult,
    pub result_digest: String,
    pub state: Satisfaction,
}

#[derive(Clone, Copy, Debug)]
pub struct ScheduleLimits {
    pub scopes: usize,
    pub dependencies: usize,
    pub facts: usize,
    pub text_bytes: usize,
}
impl Default for ScheduleLimits {
    fn default() -> Self {
        Self {
            scopes: 4096,
            dependencies: 16384,
            facts: 16384,
            text_bytes: 4 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Waiting {
    pub dependency: usize,
    pub state: Satisfaction,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SatisfiedDependency {
    pub dependency: usize,
    pub fact: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ScheduleReport {
    /// Input scope indexes in stable input order. Dependency readiness only.
    pub ready: Vec<usize>,
    /// Input edge indexes with unknown/unmet result facts; reasons stay in input.
    pub waiting: Vec<Waiting>,
    /// Input fact indexes retain the exact result consumed, including unpinned edges.
    pub satisfied: Vec<SatisfiedDependency>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ScheduleError {
    ResourceLimit,
    InvalidInput,
    DuplicateScope,
    DuplicateDependency,
    DuplicateFact,
    ConflictingFact,
    Cycle,
    AmbiguousResult,
}

/// Validate the whole graph before reporting any ready work. No partial success.
pub fn evaluate_schedule(
    scopes: &[WorkScope],
    dependencies: &[Dependency],
    facts: &[ResultFact],
    limits: ScheduleLimits,
) -> Result<ScheduleReport, ScheduleError> {
    if limits.scopes == 0
        || limits.dependencies == 0
        || limits.facts == 0
        || limits.text_bytes == 0
        || scopes.len() > limits.scopes
        || dependencies.len() > limits.dependencies
        || facts.len() > limits.facts
    {
        return Err(ScheduleError::ResourceLimit);
    }
    // Charge all borrowed text before allocating indexes or result arrays.
    let mut remaining = limits.text_bytes;
    for scope in scopes.iter().chain(facts.iter().map(|f| &f.scope)) {
        charge(&scope.warrant, &mut remaining)?;
        charge(&scope.contract_digest, &mut remaining)?;
        if let Some(stage) = &scope.stage {
            charge(stage, &mut remaining)?;
        }
        if scope.warrant.trim().is_empty()
            || !is_digest(&scope.contract_digest)
            || scope.stage.as_ref().is_some_and(|s| s.trim().is_empty())
        {
            return Err(ScheduleError::InvalidInput);
        }
    }
    for edge in dependencies {
        if let Some(digest) = &edge.result_digest {
            charge(digest, &mut remaining)?;
        }
        charge(&edge.reason, &mut remaining)?;
        if edge.predecessor >= scopes.len()
            || edge.successor >= scopes.len()
            || edge.result_digest.as_ref().is_some_and(|d| !is_digest(d))
            || edge.reason.trim().is_empty()
        {
            return Err(ScheduleError::InvalidInput);
        }
    }
    for fact in facts {
        charge(&fact.result_digest, &mut remaining)?;
        if !is_digest(&fact.result_digest) {
            return Err(ScheduleError::InvalidInput);
        }
    }
    let mut unique = BTreeSet::new();
    for scope in scopes {
        if !unique.insert(scope) {
            return Err(ScheduleError::DuplicateScope);
        }
    }
    let mut fact_index = BTreeMap::new();
    let mut result_groups = BTreeMap::new();
    for (index, fact) in facts.iter().enumerate() {
        let key = (&fact.scope, fact.requirement, fact.result_digest.as_str());
        if let Some(previous) = fact_index.insert(key, index) {
            return Err(if facts[previous].state == fact.state {
                ScheduleError::DuplicateFact
            } else {
                ScheduleError::ConflictingFact
            });
        }
        let group = result_groups
            .entry((&fact.scope, fact.requirement))
            .or_insert((index, 0usize));
        group.1 += 1;
    }
    let mut edges = BTreeSet::new();
    let mut successors = vec![Vec::new(); scopes.len()];
    let mut indegree = vec![0usize; scopes.len()];
    for edge in dependencies {
        if !edges.insert((
            edge.predecessor,
            edge.successor,
            edge.requirement,
            &edge.result_digest,
        )) {
            return Err(ScheduleError::DuplicateDependency);
        }
        successors[edge.predecessor].push(edge.successor);
        indegree[edge.successor] += 1;
    }
    // Iterative traversal also bounds stack usage for long valid chains.
    let mut frontier: VecDeque<_> = indegree
        .iter()
        .enumerate()
        .filter_map(|(i, n)| (*n == 0).then_some(i))
        .collect();
    let mut visited = 0;
    while let Some(node) = frontier.pop_front() {
        visited += 1;
        for &next in &successors[node] {
            indegree[next] -= 1;
            if indegree[next] == 0 {
                frontier.push_back(next);
            }
        }
    }
    if visited != scopes.len() {
        return Err(ScheduleError::Cycle);
    }
    let mut blocked = vec![false; scopes.len()];
    let mut waiting = Vec::new();
    let mut satisfied = Vec::new();
    for (index, edge) in dependencies.iter().enumerate() {
        let scope = &scopes[edge.predecessor];
        let matched = if let Some(digest) = &edge.result_digest {
            fact_index
                .get(&(scope, edge.requirement, digest.as_str()))
                .copied()
        } else {
            match result_groups.get(&(scope, edge.requirement)) {
                Some((_, count)) if *count > 1 => return Err(ScheduleError::AmbiguousResult),
                Some((fact, _)) => Some(*fact),
                None => None,
            }
        };
        let state = matched.map_or(Satisfaction::Unknown, |i| facts[i].state);
        if state != Satisfaction::Satisfied {
            blocked[edge.successor] = true;
            waiting.push(Waiting {
                dependency: index,
                state,
            });
        } else if let Some(fact) = matched {
            satisfied.push(SatisfiedDependency {
                dependency: index,
                fact,
            });
        }
    }
    Ok(ScheduleReport {
        ready: blocked
            .iter()
            .enumerate()
            .filter_map(|(i, b)| (!b).then_some(i))
            .collect(),
        waiting,
        satisfied,
    })
}

fn charge(text: &str, remaining: &mut usize) -> Result<(), ScheduleError> {
    *remaining = remaining
        .checked_sub(text.len())
        .ok_or(ScheduleError::ResourceLimit)?;
    Ok(())
}
fn is_digest(text: &str) -> bool {
    text.len() == 71
        && text.starts_with("sha256:")
        && text.as_bytes()[7..]
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Difficulty {
    Low,
    Medium,
    High,
    Unknown,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// Optional advisory metadata. Deliberately not an input to dependency evaluation.
#[derive(Clone, Debug)]
pub struct DifficultyEstimate {
    pub difficulty: Difficulty,
    pub reason: String,
    pub confidence: Confidence,
    pub estimator: String,
    pub revision: String,
}

/// Check metadata shape only; neither authenticates its author nor predicts cost.
pub fn check_estimate(
    estimate: &DifficultyEstimate,
    text_bytes: usize,
) -> Result<(), ScheduleError> {
    if text_bytes == 0 {
        return Err(ScheduleError::ResourceLimit);
    }
    let mut remaining = text_bytes;
    for value in [&estimate.reason, &estimate.estimator, &estimate.revision] {
        charge(value, &mut remaining)?;
        if value.trim().is_empty() {
            return Err(ScheduleError::InvalidInput);
        }
    }
    Ok(())
}
