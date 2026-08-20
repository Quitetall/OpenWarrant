// SPDX-License-Identifier: AGPL-3.0-or-later
//! Agent-assisted planning (SAS §74), the agent adapter protocol (§75), and the
//! interview loop (§71.3–§71.4). RQ-071, RQ-072, RQ-073.
//!
//! # The model does not get to write files
//!
//! §74.2: *"The agent SHALL return a structured Draft Proposal, not arbitrary
//! file writes."* §74.5: *"The model does not receive unrestricted filesystem
//! writes through OpenWarrant's planning mode."*
//!
//! So a planning agent's entire output surface is [`DraftProposal`], and its
//! effects are [`AtomOperation`]s from §74.3's closed list. There is no path
//! from a planner to the filesystem, which is why §74.4's eight-step gauntlet is
//! meaningful: the proposal has to survive all eight before anything is written.
//! [`ApplicationPipeline`] models the eight, and refuses to apply while any step
//! is unrun — an unrun step is not a passed step, the same rule §32 needs.
//!
//! # The rule that is hardest to enforce and matters most
//!
//! §74.8: the planner distinguishes existing evidence, evidence it expects to
//! collect, assumptions, recommendations, and unknowns — and *"SHALL never
//! fabricate a source or gate result."*
//!
//! A checker cannot tell a real citation from an invented one. What it CAN do is
//! refuse the shortcut that makes fabrication easy: [`EvidenceClaim`] cannot be
//! `Existing` without a reference, and a claim about a gate result is refused
//! unless it names a gate run. [`DraftProposal::validate`] therefore catches the
//! careless case and is honest that it cannot catch the determined one — which is
//! recorded here rather than implied by silence.
//!
//! # §74.7, decision detection
//!
//! *"If the planner identifies a choice among durable alternatives, it SHALL
//! produce a proposed ADR draft, not bury the choice in a Work Order atom."*
//! [`DraftProposal::validate`] refuses a proposal that records durable
//! alternatives without a corresponding ADR draft.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::vocab::vocabulary;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DraftError {
    #[error("unknown {vocabulary} {found:?}; SAS defines {known}")]
    UnknownTerm {
        vocabulary: &'static str,
        found: String,
        known: String,
    },
    #[error(
        "the proposal records a choice among durable alternatives ({choice:?}) and \
         no proposed ADR draft. §74.7: the planner SHALL produce a proposed ADR \
         draft, not bury the choice in a Work Order atom"
    )]
    DurableChoiceBuried { choice: String },
    #[error(
        "evidence claim {id:?} is classed `existing` and cites nothing. §74.8: the \
         planner SHALL distinguish existing evidence from evidence it expects to \
         collect, and SHALL never fabricate a source"
    )]
    ExistingEvidenceUncited { id: String },
    #[error(
        "evidence claim {id:?} asserts a gate result without naming a gate run. \
         §74.8 — a planner asserting how a gate came out, with no run behind it, is \
         fabricating a gate result"
    )]
    GateResultWithoutRun { id: String },
    #[error(
        "atom operation {op:?} is not in §74.3's vocabulary. A planner's effects are \
         a closed list; anything else is an arbitrary file write by another name"
    )]
    UnknownAtomOperation { op: String },
    #[error(
        "application refused: step {step} of §74.4's eight has not been run. An \
         unrun step is not a passed step, and the last three steps write to the \
         repository"
    )]
    PipelineStepNotRun { step: &'static str },
    #[error("application refused: step {step} failed — {detail}")]
    PipelineStepFailed { step: &'static str, detail: String },
    #[error(
        "the proposal requests {count} unresolved question(s) to be skipped. §74.6 \
         asks the MINIMUM set needed to remove blockers, which is not the empty set \
         when blockers remain"
    )]
    BlockingQuestionsSkipped { count: usize },
}

vocabulary!(
    /// §74.3's atom operations. A planner's entire effect surface.
    AtomOperation, "atom operation", DraftError, {
        CreateAtom => "create_atom",
        ReviseAtom => "revise_atom",
        RetireAtom => "retire_atom",
        AddBinding => "add_binding",
        RemoveBinding => "remove_binding",
        AddRelation => "add_relation",
        ProposeAdr => "propose_adr",
    }
);

vocabulary!(
    /// §74.8's five epistemic kinds a planner must keep apart.
    EvidenceClass, "evidence class", DraftError, {
        /// Already exists and can be cited now.
        Existing => "existing",
        /// The planner expects to collect it. Not yet a fact.
        Expected => "expected",
        Assumption => "assumption",
        Recommendation => "recommendation",
        Unknown => "unknown",
    }
);

impl EvidenceClass {
    /// Whether a claim of this class asserts something that exists today.
    ///
    /// Only `existing`. The other four are about the future, the author's belief,
    /// or the absence of knowledge — and collapsing any of them into `existing`
    /// is how a plan starts citing evidence nobody has.
    #[must_use]
    pub const fn asserts_a_present_fact(self) -> bool {
        matches!(self, Self::Existing)
    }
}

/// §74.4's eight steps, verbatim and in order.
pub const APPLICATION_STEPS: [&str; 8] = [
    "parse the proposal",
    "validate schema",
    "validate semantic references",
    "run risk and authority checks",
    "show a semantic diff",
    "require review or policy approval",
    "write authored atoms",
    "compile and check",
];

/// One §74.8 evidence claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceClaim {
    pub id: String,
    pub statement: String,
    pub class: EvidenceClass,
    #[serde(default)]
    pub reference: String,
    /// Whether this claim asserts how a gate came out.
    #[serde(default)]
    pub asserts_gate_result: bool,
    #[serde(default)]
    pub gate_run_ref: String,
}

impl EvidenceClaim {
    pub fn validate(&self) -> Result<(), DraftError> {
        if self.class.asserts_a_present_fact() && self.reference.trim().is_empty() {
            return Err(DraftError::ExistingEvidenceUncited {
                id: self.id.clone(),
            });
        }
        if self.asserts_gate_result && self.gate_run_ref.trim().is_empty() {
            return Err(DraftError::GateResultWithoutRun {
                id: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// A choice the planner found among durable alternatives (§74.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurableChoice {
    pub statement: String,
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// The proposed ADR draft this choice produced. §74.7 requires one.
    #[serde(default)]
    pub proposed_adr_draft: String,
}

/// §74.2's Draft Proposal — the agent's entire output surface.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DraftProposal {
    pub api_version: String,
    #[serde(default)]
    pub proposed_identity: String,
    #[serde(default)]
    pub atom_operations: Vec<AtomOperation>,
    #[serde(default)]
    pub proposed_adr_drafts: Vec<String>,
    #[serde(default)]
    pub proposed_relations: Vec<String>,
    #[serde(default)]
    pub evidence_claims: Vec<EvidenceClaim>,
    #[serde(default)]
    pub durable_choices: Vec<DurableChoice>,
    #[serde(default)]
    pub unresolved_questions: Vec<InterviewQuestion>,
    #[serde(default)]
    pub risk_assessment: String,
    /// §39.3 — attacks the planner proposes against its own gate set.
    #[serde(default)]
    pub adequacy_attacks: Vec<String>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

impl DraftProposal {
    pub fn validate(&self) -> Result<(), DraftError> {
        for claim in &self.evidence_claims {
            claim.validate()?;
        }
        // §74.7 — a durable choice produces an ADR draft, or it has been buried.
        for choice in &self.durable_choices {
            if choice.proposed_adr_draft.trim().is_empty() {
                return Err(DraftError::DurableChoiceBuried {
                    choice: choice.statement.clone(),
                });
            }
        }
        Ok(())
    }

    /// §74.3 — reject an operation outside the closed list, by name.
    ///
    /// For operations arriving as text from an agent process (§75.2), where the
    /// type system is not between the model and the repository.
    pub fn validate_operation(op: &str) -> Result<AtomOperation, DraftError> {
        op.parse()
            .map_err(|_| DraftError::UnknownAtomOperation { op: op.to_owned() })
    }

    /// What this proposal cannot establish, stated rather than implied.
    ///
    /// §74.8 forbids fabricating a source or gate result. No validator can tell a
    /// real citation from a convincing invented one; what is enforced is that the
    /// SHORTCUTS are closed — an `existing` claim must cite something, and a gate
    /// assertion must name a run. The remaining exposure is a fabricated
    /// reference that looks well-formed, and it is a human review problem.
    #[must_use]
    pub const fn honest_limitation() -> &'static str {
        "Validation confirms that evidence claims are CLASSED and CITED. It cannot \
         confirm that a citation resolves to something real, or that a named gate \
         run produced what the claim says. §74.8's prohibition on fabricating a \
         source is enforced against carelessness, not against intent."
    }
}

/// §74.6 / §71.4's interview question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterviewQuestion {
    pub id: String,
    pub question: String,
    /// Whether an answer is needed to clear a blocker. §74.6 asks the MINIMUM set
    /// needed to remove blockers — which is not the empty set while any remain.
    #[serde(default)]
    pub removes_blocker: bool,
    /// Ranking input. §74.6: "rank them by expected information gain".
    #[serde(default)]
    pub expected_information_gain: u32,
}

/// §74.6 — the minimum set needed to remove blockers, ranked by information gain.
///
/// Blocker-clearing questions are never dropped, whatever their rank. Ranking
/// decides the ORDER of what must be asked and which optional extras are worth
/// asking; it does not decide whether a blocker gets addressed.
#[must_use]
pub fn minimum_question_set(questions: &[InterviewQuestion]) -> Vec<InterviewQuestion> {
    let mut required: Vec<InterviewQuestion> = questions
        .iter()
        .filter(|q| q.removes_blocker)
        .cloned()
        .collect();
    required.sort_by(|a, b| {
        b.expected_information_gain
            .cmp(&a.expected_information_gain)
            .then_with(|| a.id.cmp(&b.id))
    });
    required
}

/// Refuse to proceed while blocker-clearing questions are unanswered.
pub fn require_blockers_answered(
    questions: &[InterviewQuestion],
    answered: &BTreeSet<String>,
) -> Result<(), DraftError> {
    let unanswered = questions
        .iter()
        .filter(|q| q.removes_blocker && !answered.contains(&q.id))
        .count();
    if unanswered == 0 {
        Ok(())
    } else {
        Err(DraftError::BlockingQuestionsSkipped { count: unanswered })
    }
}

/// §74.4's eight-step gauntlet, as state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ApplicationPipeline {
    /// Step name -> whether it ran AND passed. Absent means not run.
    #[serde(default)]
    pub completed: BTreeSet<String>,
    #[serde(default)]
    pub failures: Vec<(String, String)>,
}

impl ApplicationPipeline {
    pub fn complete(&mut self, step: &str) {
        self.completed.insert(step.to_owned());
    }

    pub fn fail(&mut self, step: &str, detail: &str) {
        self.failures.push((step.to_owned(), detail.to_owned()));
    }

    /// §74.4 — every step before the write must have run and passed.
    ///
    /// Steps 7 and 8 write to and verify the repository, so the gate is on steps
    /// 1 through 6. An unrun step blocks exactly as a failed one does.
    pub fn may_apply(&self) -> Result<(), DraftError> {
        for step in &APPLICATION_STEPS[..6] {
            if let Some((_, detail)) = self.failures.iter().find(|(s, _)| s == step) {
                return Err(DraftError::PipelineStepFailed {
                    step,
                    detail: detail.clone(),
                });
            }
            if !self.completed.contains(*step) {
                return Err(DraftError::PipelineStepNotRun { step });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    /// §74.3's seven, transcribed — the planner's whole effect surface.
    #[test]
    fn the_atom_operations_match_the_sas() {
        assert_eq!(
            AtomOperation::ALL
                .iter()
                .map(|o| o.as_str())
                .collect::<Vec<_>>(),
            [
                "create_atom",
                "revise_atom",
                "retire_atom",
                "add_binding",
                "remove_binding",
                "add_relation",
                "propose_adr",
            ]
        );
    }

    /// §74.5 — no unrestricted filesystem writes. Anything outside §74.3's list
    /// is an arbitrary file write by another name.
    #[test]
    fn an_operation_outside_the_vocabulary_is_refused() {
        for forbidden in [
            "write_file",
            "delete_file",
            "run_command",
            "patch",
            "edit",
            "apply_diff",
        ] {
            assert!(
                matches!(
                    DraftProposal::validate_operation(forbidden),
                    Err(DraftError::UnknownAtomOperation { .. })
                ),
                "{forbidden:?} was accepted as a planner operation"
            );
        }
        assert!(DraftProposal::validate_operation("create_atom").is_ok());
    }

    /// §74.4's eight, transcribed.
    #[test]
    fn the_application_steps_match_the_sas() {
        assert_eq!(APPLICATION_STEPS.len(), 8);
        assert_eq!(APPLICATION_STEPS[0], "parse the proposal");
        assert_eq!(APPLICATION_STEPS[5], "require review or policy approval");
        assert_eq!(APPLICATION_STEPS[7], "compile and check");
    }

    /// An unrun step is not a passed step. Each of the six blocks alone.
    #[test]
    fn each_pre_write_step_blocks_application_on_its_own() {
        let all_done = || {
            let mut p = ApplicationPipeline::default();
            for s in &APPLICATION_STEPS[..6] {
                p.complete(s);
            }
            p
        };
        assert_eq!(all_done().may_apply(), Ok(()));

        for step in &APPLICATION_STEPS[..6] {
            let mut p = all_done();
            p.completed.remove(*step);
            match p.may_apply() {
                Err(DraftError::PipelineStepNotRun { step: named }) => {
                    assert_eq!(&named, step);
                }
                other => panic!("skipping {step:?} was permitted: {other:?}"),
            }
        }
    }

    #[test]
    fn a_failed_step_blocks_and_says_why() {
        let mut p = ApplicationPipeline::default();
        for s in &APPLICATION_STEPS[..6] {
            p.complete(s);
        }
        p.fail("validate semantic references", "OBL-999 is not declared");
        match p.may_apply() {
            Err(DraftError::PipelineStepFailed { step, detail }) => {
                assert_eq!(step, "validate semantic references");
                assert!(detail.contains("OBL-999"));
            }
            other => panic!("a failed step was permitted: {other:?}"),
        }
    }

    /// An empty pipeline must not apply. This is the fail-closed case.
    #[test]
    fn an_unrun_pipeline_cannot_apply() {
        assert!(matches!(
            ApplicationPipeline::default().may_apply(),
            Err(DraftError::PipelineStepNotRun { .. })
        ));
    }

    // ---- §74.7 -----------------------------------------------------------

    /// A durable choice produces an ADR draft, or it has been buried.
    #[test]
    fn a_durable_choice_without_an_adr_draft_is_refused() {
        let mut p = DraftProposal {
            api_version: "oh.war/draft-proposal/v1".into(),
            durable_choices: vec![DurableChoice {
                statement: "restricted reader versus a YAML dependency".into(),
                alternatives: vec!["serde_yaml".into(), "hand-written subset".into()],
                proposed_adr_draft: String::new(),
            }],
            ..DraftProposal::default()
        };
        let err = p.validate().unwrap_err();
        assert!(
            matches!(err, DraftError::DurableChoiceBuried { .. }),
            "{err}"
        );
        assert!(err.to_string().contains("restricted reader"), "{err}");

        p.durable_choices[0].proposed_adr_draft = "OW-ADR-0003 draft".into();
        assert_eq!(p.validate(), Ok(()));
    }

    // ---- §74.8 -----------------------------------------------------------

    /// §74.8's five classes, transcribed, and only one asserts a present fact.
    #[test]
    fn the_evidence_classes_match_the_sas() {
        assert_eq!(
            EvidenceClass::ALL
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>(),
            [
                "existing",
                "expected",
                "assumption",
                "recommendation",
                "unknown"
            ]
        );
        assert!(EvidenceClass::Existing.asserts_a_present_fact());
        for c in [
            EvidenceClass::Expected,
            EvidenceClass::Assumption,
            EvidenceClass::Recommendation,
            EvidenceClass::Unknown,
        ] {
            assert!(
                !c.asserts_a_present_fact(),
                "{c} was treated as a present fact"
            );
        }
    }

    /// A claim of existing evidence that cites nothing.
    #[test]
    fn existing_evidence_must_cite_something() {
        let bare = EvidenceClaim {
            id: "EV-1".into(),
            statement: "the byte-equality gate passes".into(),
            class: EvidenceClass::Existing,
            reference: String::new(),
            asserts_gate_result: false,
            gate_run_ref: String::new(),
        };
        assert!(matches!(
            bare.validate(),
            Err(DraftError::ExistingEvidenceUncited { .. })
        ));

        // Classed honestly as expected, it needs no citation yet.
        let expected = EvidenceClaim {
            class: EvidenceClass::Expected,
            ..bare.clone()
        };
        assert_eq!(expected.validate(), Ok(()));
    }

    /// Asserting how a gate came out, with no run behind it, is fabricating a
    /// gate result.
    #[test]
    fn a_gate_result_claim_must_name_a_run() {
        let c = EvidenceClaim {
            id: "EV-2".into(),
            statement: "the conformance gate passed on the corpus".into(),
            class: EvidenceClass::Existing,
            reference: "docs/report.md".into(),
            asserts_gate_result: true,
            gate_run_ref: String::new(),
        };
        assert!(matches!(
            c.validate(),
            Err(DraftError::GateResultWithoutRun { .. })
        ));

        let cited = EvidenceClaim {
            gate_run_ref: "gate-run://GR-1".into(),
            ..c
        };
        assert_eq!(cited.validate(), Ok(()));
    }

    /// The limitation is stated, not implied by silence.
    #[test]
    fn the_module_states_what_it_cannot_check() {
        let l = DraftProposal::honest_limitation();
        assert!(l.contains("cannot confirm that a citation resolves"));
        assert!(l.contains("against carelessness, not against intent"));
    }

    // ---- §74.6 / §71.4 ---------------------------------------------------

    fn q(id: &str, blocker: bool, gain: u32) -> InterviewQuestion {
        InterviewQuestion {
            id: id.into(),
            question: format!("question {id}"),
            removes_blocker: blocker,
            expected_information_gain: gain,
        }
    }

    /// §74.6 — the minimum set is every blocker-clearing question, ranked. A
    /// low-ranked blocker is still a blocker.
    #[test]
    fn ranking_orders_questions_but_never_drops_a_blocker() {
        let questions = vec![
            q("Q1", true, 10),
            q("Q2", false, 99),
            q("Q3", true, 1),
            q("Q4", false, 50),
        ];
        let minimum = minimum_question_set(&questions);
        let ids: Vec<&str> = minimum.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids, ["Q1", "Q3"], "a low-gain blocker was dropped");
        assert!(
            !ids.contains(&"Q2"),
            "a high-gain non-blocker displaced a blocker"
        );
    }

    #[test]
    fn unanswered_blocking_questions_stop_progress() {
        let questions = vec![q("Q1", true, 10), q("Q3", true, 1)];
        let none = BTreeSet::new();
        assert_eq!(
            require_blockers_answered(&questions, &none),
            Err(DraftError::BlockingQuestionsSkipped { count: 2 })
        );

        let some: BTreeSet<String> = ["Q1".to_owned()].into_iter().collect();
        assert_eq!(
            require_blockers_answered(&questions, &some),
            Err(DraftError::BlockingQuestionsSkipped { count: 1 })
        );

        let all: BTreeSet<String> = ["Q1".to_owned(), "Q3".to_owned()].into_iter().collect();
        assert_eq!(require_blockers_answered(&questions, &all), Ok(()));
    }

    #[test]
    fn a_proposal_round_trips_through_json() {
        let p = DraftProposal {
            api_version: "oh.war/draft-proposal/v1".into(),
            atom_operations: vec![AtomOperation::CreateAtom, AtomOperation::ProposeAdr],
            evidence_claims: vec![EvidenceClaim {
                id: "EV-1".into(),
                statement: "x".into(),
                class: EvidenceClass::Assumption,
                reference: String::new(),
                asserts_gate_result: false,
                gate_run_ref: String::new(),
            }],
            ..DraftProposal::default()
        };
        let s = serde_json::to_string(&p).expect("serialize");
        assert_eq!(
            serde_json::from_str::<DraftProposal>(&s).expect("deserialize"),
            p
        );
    }

    #[test]
    fn vocabularies_round_trip() {
        for &o in AtomOperation::ALL {
            assert_eq!(AtomOperation::from_str(o.as_str()), Ok(o));
        }
        for &c in EvidenceClass::ALL {
            assert_eq!(EvidenceClass::from_str(c.as_str()), Ok(c));
        }
    }
}
