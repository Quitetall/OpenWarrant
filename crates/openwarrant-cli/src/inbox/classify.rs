// SPDX-License-Identifier: Apache-2.0
//! Classification uses existing request readiness, never a phase as permission.
use openwarrant_core::Phase;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanAct {
    Answer,
    Authorize,
    Correct,
    Resolve,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NextAct {
    Human(HumanAct),
    Agent,
    Gate,
    None,
}

/// Minimal, loaded view: no I/O and no signing capability.
pub struct Warrant {
    pub phase: Phase,
    pub pending: Option<HumanAct>,
    pub blocking_question: bool,
}

pub fn next_act(w: &Warrant) -> NextAct {
    // An answer can change the contract being signed; surface that first.
    let human = if w.blocking_question {
        Some(HumanAct::Answer)
    } else {
        w.pending
    };
    if let Some(act) = human {
        return NextAct::Human(act);
    }
    match w.phase {
        Phase::Draft | Phase::Proposed | Phase::Authorized | Phase::Ready | Phase::Executing => {
            NextAct::Agent
        }
        Phase::Verifying => NextAct::Gate,
        Phase::Resolved => NextAct::None,
    }
}
