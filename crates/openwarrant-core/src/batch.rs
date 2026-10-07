// SPDX-License-Identifier: Apache-2.0
//! The batch act (OW-WAR-0072): one human signature over many pending acts.
//!
//! A batch is a list. Each entry names one act — authorize, resolve, correct,
//! accept a SAS revision, accept a roadmap revision — the response file that
//! act would have been signed over alone, and that file's exact sha256. The
//! signer signs the batch document once (`ssh-keygen -Y sign -n
//! oh.war/response`); every response it lists is then as signed as if it had
//! carried its own `.sig`, and no more: a response whose bytes differ from the
//! listed digest is not covered.
//!
//! A batch is applied whole or not at all. Partial application would record
//! acts the signer signed as a list they did not sign — the list with a hole
//! in it.

use serde::{Deserialize, Serialize};

pub const BATCH_SCHEMA: &str = "oh.war/batch/v1";

/// One act in a batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchAct {
    /// `authorize`, `resolve`, `correct`, `accept`, `accept-roadmap`.
    pub act: String,
    /// What `war sign <target>` takes.
    pub target: String,
    /// The response file's name under `docs/authority/responses/`.
    pub response: String,
    /// sha256 of the response file's exact bytes, lowercase hex.
    pub response_sha256: String,
    /// The digest the act binds (contract, document, new artifact digest).
    pub bound_digest: String,
}

/// The document a signer signs once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub schema: String,
    pub batch_id: String,
    pub drafted_at: String,
    /// The actor, as `roles.toml` names them. The batch signature must verify
    /// as this actor's principal, and a record covered by the batch must name
    /// this actor.
    pub signer: String,
    /// One role per batch: `authorizer` or `resolver`.
    pub role: String,
    pub acts: Vec<BatchAct>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BatchError {
    #[error("batch.empty: a batch lists no act")]
    Empty,
    #[error("batch.duplicate-act: {0} is listed twice")]
    Duplicate(String),
    #[error("batch.schema: {0:?} is not {BATCH_SCHEMA}")]
    Schema(String),
    #[error("batch.malformed: {0}")]
    Malformed(String),
}

impl Batch {
    /// Structural checks every reader applies before trusting a batch.
    pub fn validate(&self) -> Result<(), BatchError> {
        if self.schema != BATCH_SCHEMA {
            return Err(BatchError::Schema(self.schema.clone()));
        }
        if self.acts.is_empty() {
            return Err(BatchError::Empty);
        }
        let mut seen = std::collections::BTreeSet::new();
        for a in &self.acts {
            if !seen.insert((a.act.as_str(), a.target.as_str())) {
                return Err(BatchError::Duplicate(format!("{} {}", a.act, a.target)));
            }
            if a.response.contains('/')
                || a.response.contains("..")
                || !a.response.ends_with(".response.toml")
            {
                return Err(BatchError::Malformed(format!(
                    "response {:?} is not a bare `*.response.toml` name",
                    a.response
                )));
            }
            if a.response_sha256.len() != 64
                || !a.response_sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err(BatchError::Malformed(format!(
                    "{}: response_sha256 is not 64 hex digits",
                    a.response
                )));
            }
        }
        if self.signer.trim().is_empty() || self.role.trim().is_empty() {
            return Err(BatchError::Malformed(
                "signer and role are required".to_owned(),
            ));
        }
        Ok(())
    }

    /// The entry covering one response file, by name and exact digest.
    #[must_use]
    pub fn covers(&self, response_name: &str, response_sha256: &str) -> Option<&BatchAct> {
        self.acts
            .iter()
            .find(|a| a.response == response_name && a.response_sha256 == response_sha256)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn act(t: &str) -> BatchAct {
        BatchAct {
            act: "authorize".into(),
            target: t.into(),
            response: format!("{t}.response.toml"),
            response_sha256: "a".repeat(64),
            bound_digest: "b".repeat(64),
        }
    }

    fn batch(acts: Vec<BatchAct>) -> Batch {
        Batch {
            schema: BATCH_SCHEMA.into(),
            batch_id: "B-1".into(),
            drafted_at: "2026-09-23T00:00:00Z".into(),
            signer: "Ada".into(),
            role: "authorizer".into(),
            acts,
        }
    }

    #[test]
    fn a_sound_batch_validates_and_covers_by_name_and_digest() {
        let b = batch(vec![act("OW-WAR-0001"), act("OW-WAR-0002")]);
        assert_eq!(b.validate(), Ok(()));
        assert!(
            b.covers("OW-WAR-0001.response.toml", &"a".repeat(64))
                .is_some()
        );
        assert!(
            b.covers("OW-WAR-0001.response.toml", &"c".repeat(64))
                .is_none()
        );
        assert!(
            b.covers("OW-WAR-0009.response.toml", &"a".repeat(64))
                .is_none()
        );
    }

    #[test]
    fn it_refuses_by_name() {
        assert_eq!(batch(vec![]).validate(), Err(BatchError::Empty));
        assert!(matches!(
            batch(vec![act("X"), act("X")]).validate(),
            Err(BatchError::Duplicate(_))
        ));
        let mut bad = act("X");
        bad.response = "../etc/passwd".into();
        assert!(matches!(
            batch(vec![bad]).validate(),
            Err(BatchError::Malformed(_))
        ));
        let mut short = act("Y");
        short.response_sha256 = "ab".into();
        assert!(matches!(
            batch(vec![short]).validate(),
            Err(BatchError::Malformed(_))
        ));
        let mut wrong = batch(vec![act("Z")]);
        wrong.schema = "x".into();
        assert!(matches!(wrong.validate(), Err(BatchError::Schema(_))));
    }
}
