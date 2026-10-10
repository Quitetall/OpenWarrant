// SPDX-License-Identifier: Apache-2.0
//! Current source/capture assessment for resolution. Saved verdicts are never inputs.
use openwarrant_compiler::runtime_basis::RuntimeBasisAssessment;
use openwarrant_core::{document::runtime::*, execution::StageDispatch};

use crate::{
    diagnostic::{Diagnostic, Report},
    repo::{Loaded, Repository},
    runtime_capture::{self, Selection, Verification},
};

/// Explicit retained selections and independently established native verification.
/// A caller must authenticate the provider and resolve policy for this dispatch.
/// A parsed receipt, collector assertion or saved verdict is not such a verifier.
pub struct Input<'a> {
    pub selections: &'a [Selection],
    pub native: &'a dyn Fn(&StageDispatch, &ProviderInterface) -> Option<Verification<'a>>,
}

impl Input<'_> {
    /// The reference CLI has no configured native verifier. Missing evidence
    /// stays UNKNOWN; valid contracts with no runtime stages can still match.
    pub fn unavailable() -> Self {
        Self {
            selections: &[],
            native: &|_, _| None,
        }
    }

    pub(super) fn assess(&self, repo: &Repository, one: &Loaded) -> RuntimeBasisAssessment {
        let unavailable = |code, detail: String| RuntimeBasisAssessment {
            standing: ReceiptStanding::Unknown,
            code,
            detail,
            stages: vec![],
        };
        let current = match repo.load_warrant(&one.dir) {
            Ok(current) => current,
            Err(error) => {
                return unavailable("runtime.resolution-source-unavailable", error.to_string());
            }
        };
        if current.basis != one.basis {
            return unavailable(
                "runtime.resolution-basis-changed",
                "loaded resolution basis differs from current source".into(),
            );
        }
        match runtime_capture::assess_selected(repo, &one.alias(), self.selections, self.native) {
            Ok(assessment) => assessment,
            Err(fault) => RuntimeBasisAssessment {
                standing: if fault.unknown {
                    ReceiptStanding::Unknown
                } else {
                    ReceiptStanding::Refused
                },
                code: fault.code,
                detail: fault.message,
                stages: vec![],
            },
        }
    }
}

pub(super) fn report(report: &mut Report, alias: &str, assessment: &RuntimeBasisAssessment) {
    let mut push = |standing, code, detail: String| {
        let message = format!("{alias}: {detail}");
        report.push(match standing {
            ReceiptStanding::Matches => Diagnostic::pass(code, message),
            ReceiptStanding::Unknown => Diagnostic::unknown(code, "", message),
            ReceiptStanding::Refused => Diagnostic::error(code, "", message),
        });
    };
    push(
        assessment.standing,
        assessment.code,
        assessment.detail.clone(),
    );
    for stage in &assessment.stages {
        push(
            stage.receipt.standing,
            stage.receipt.code,
            format!("{}: {}", stage.stage_id, stage.receipt.detail),
        );
    }
}
