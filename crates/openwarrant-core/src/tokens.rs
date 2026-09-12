// SPDX-License-Identifier: Apache-2.0
//! Token accounting for Dispatches (slice C2; SAS §33.7 "the compiler SHALL
//! record a budget").
//!
//! One method, named in every record that carries an estimate: bytes divided
//! by four, rounded up. It is an estimate of the reading cost of UTF-8 prose
//! for the tokenisers in use today, not a measurement; no model API is called,
//! ever. A later method gets a new id, and a record says which it used.

use serde::{Deserialize, Serialize};

pub const METHOD: &str = "oh.war/token-estimate/bytes-div-4/v1";

/// The estimate for `bytes` of context under [`METHOD`].
#[must_use]
pub const fn estimate(bytes: u64) -> u64 {
    bytes.div_ceil(4)
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
/// What a Dispatch records about its size (§33.7): inside the packet, so
/// inside its digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAccount {
    pub estimated_tokens: u64,
    pub budget_tokens: u64,
    pub method: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_estimate_rounds_up_and_names_its_method() {
        assert_eq!(estimate(0), 0);
        assert_eq!(estimate(1), 1);
        assert_eq!(estimate(4), 1);
        assert_eq!(estimate(5), 2);
        assert_eq!(estimate(4000), 1000);
        assert!(METHOD.starts_with("oh.war/token-estimate/"));
    }
}
