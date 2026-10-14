// SPDX-License-Identifier: Apache-2.0
//! Pure accounting for one host-owned run. No billing or enforcement observation
//! is established here. The trusted host must serialize and durably commit this
//! state BEFORE any paid request, protect policy/state from the performer, and
//! obtain charge guarantees and settlement independently of model claims.
//! Constructing another ledger or losing this memory does not preserve a cap.
use std::collections::BTreeMap;

/// USD millionths; integer arithmetic, never a floating point estimate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsdMicros(pub u64);
#[derive(Clone, Copy, Debug)]
pub struct Policy {
    pub ceiling: Option<UsdMicros>,
    pub allow_unknown: bool,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            ceiling: Some(UsdMicros(10_000_000)),
            allow_unknown: false,
        }
    }
}
/// Host-supplied observations, not authenticated by naming this variant.
#[derive(Clone, Copy, Debug)]
pub enum ChargeBound {
    AuthoritativeMaximum(UsdMicros),
    Estimate(UsdMicros),
    Unknown,
}
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Fault {
    #[error("unavailable: authoritative charge bound or settlement missing")]
    Unavailable,
    #[error("refused: invalid, duplicate or already settled call")]
    Call,
    #[error("refused: run ceiling or arithmetic range exceeded")]
    Ceiling,
    #[error("refused: charge guarantee breached; further requests blocked")]
    Breached,
}
#[derive(Debug)]
struct Entry {
    maximum: Option<UsdMicros>,
    settled: bool,
}
/// One ledger for ALL callers/attempts in the run. Policy has no mutation API.
/// These values are deliberately not a decoded/serialized enforcement receipt.
#[derive(Debug)]
pub struct RunBudget {
    policy: Policy,
    accounted: u64,
    held: u64,
    calls: BTreeMap<String, Entry>,
    breached: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct Accounting {
    /// Known settled subtotal, not a claim that unknown spending is zero.
    pub settled: UsdMicros,
    pub reserved_maximum: UsdMicros,
    pub unsettled_calls: usize,
    pub unknown_bound_calls: usize,
    pub guarantee_breached: bool,
}
impl RunBudget {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            accounted: 0,
            held: 0,
            calls: BTreeMap::new(),
            breached: false,
        }
    }
    pub fn reserve(&mut self, call: &str, bound: ChargeBound) -> Result<(), Fault> {
        if self.breached {
            return Err(Fault::Breached);
        }
        if call.is_empty()
            || call.len() > 128
            || call.chars().any(char::is_control)
            || self.calls.contains_key(call)
        {
            return Err(Fault::Call);
        }
        let maximum = match bound {
            ChargeBound::AuthoritativeMaximum(value) => Some(value),
            ChargeBound::Estimate(_) | ChargeBound::Unknown => {
                if self.policy.ceiling.is_some() || !self.policy.allow_unknown {
                    return Err(Fault::Unavailable);
                }
                None
            }
        };
        let held = self
            .held
            .checked_add(maximum.map_or(0, |m| m.0))
            .ok_or(Fault::Ceiling)?;
        let total = self.accounted.checked_add(held).ok_or(Fault::Ceiling)?;
        if self.policy.ceiling.is_some_and(|cap| total > cap.0) {
            return Err(Fault::Ceiling);
        }
        self.calls.insert(
            call.into(),
            Entry {
                maximum,
                settled: false,
            },
        );
        self.held = held;
        Ok(())
    }
    /// Missing/error/crash settlement leaves the reservation held. A reported
    /// overcharge is recorded even though this returns Breached: money already
    /// charged cannot be undone. Overflow instead retains the unresolved entry.
    /// Every subsequent reserve then refuses.
    pub fn settle(&mut self, call: &str, actual: Option<UsdMicros>) -> Result<(), Fault> {
        let entry = self.calls.get_mut(call).ok_or(Fault::Call)?;
        if entry.settled {
            return Err(Fault::Call);
        }
        let actual = actual.ok_or(Fault::Unavailable)?;
        let Some(total) = self.accounted.checked_add(actual.0) else {
            self.breached = true;
            return Err(Fault::Breached);
        };
        self.accounted = total;
        self.held -= entry.maximum.map_or(0, |m| m.0);
        entry.settled = true;
        if entry.maximum.is_some_and(|max| actual.0 > max.0)
            || self.policy.ceiling.is_some_and(|cap| total > cap.0)
        {
            self.breached = true;
            return Err(Fault::Breached);
        }
        Ok(())
    }
    pub fn accounting(&self) -> Accounting {
        Accounting {
            settled: UsdMicros(self.accounted),
            reserved_maximum: UsdMicros(self.held),
            unsettled_calls: self.calls.values().filter(|e| !e.settled).count(),
            unknown_bound_calls: self
                .calls
                .values()
                .filter(|e| !e.settled && e.maximum.is_none())
                .count(),
            guarantee_breached: self.breached,
        }
    }
}
