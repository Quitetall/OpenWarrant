// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war kf` — the §67 Knowledge Fabric seam.
//!
//! # HTTPS works, after a licence decision that was recorded rather than assumed
//!
//! This shipped without a TLS backend for one commit. Every TLS configuration of
//! ureq 3.4 pulls Mozilla's CA bundle (`webpki-roots` or `webpki-root-certs`)
//! under `CDLA-Permissive-2.0`, and `deny.toml`'s permissive-only allowlist
//! rejected all of them — so the seam was HTTP-only and said so at the point of
//! use rather than failing in transport.
//!
//! `deny.toml` now carries a NARROW exception naming the two CA-bundle crates,
//! which is what its own header says to do instead of widening the blanket
//! allow list. The reasoning is recorded there: CDLA-Permissive-2.0 is the
//! permissive member of its family, it covers certificate DATA rather than code,
//! and it is unavoidable for TLS in this ecosystem. A future CDLA-licensed crate
//! carrying code will still fail the gate, which is correct — that is a
//! different decision from this one.
//!
//! [`Client::new`] still refuses a scheme it cannot serve at all.
//!
//! # Nothing here writes without being asked twice
//!
//! §67 actions mutate an authoritative external record. [`Client::post_action`]
//! is the only function that writes, it is reachable only from `war kf act`,
//! and that subcommand requires `--confirm-write`. The seam being easy to reach
//! by accident is how a diagnostic becomes a fabrication.

use crate::repo::RepoError;

/// KF's action envelope (`POST /actions/:actionType`), read from the running
/// service's route contract rather than invented here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ActionEnvelope {
    #[serde(rename = "targetIds")]
    pub target_ids: Vec<String>,
    pub payload: serde_json::Value,
    pub reason: String,
    /// KF refuses a key shorter than 8 characters. Supplied by the CALLER on
    /// purpose — an idempotency key the server generates is not idempotency.
    #[serde(rename = "idempotencyKey")]
    pub idempotency_key: String,
}

/// Who is acting. KF takes these as headers, not as body fields, so a receipt
/// cannot be attributed by editing a payload.
#[derive(Debug, Clone)]
pub struct Actor {
    pub actor: String,
    pub acting_role: String,
    pub organization: String,
}

pub struct Client {
    base: String,
    agent: ureq::Agent,
}

/// Hand-written: `ureq::Agent` has no `Debug`, and the base URL is the only
/// field a failing test wants to see anyway.
impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("base", &self.base).finish()
    }
}

impl Client {
    /// Refuse a URL this build cannot serve, rather than failing in transport.
    pub fn new(base: &str) -> Result<Self, RepoError> {
        if !base.starts_with("http://") && !base.starts_with("https://") {
            return Err(RepoError::Message(format!(
                "not an http(s) URL: {base}. This build speaks both; a scheme it cannot \
                 serve is refused here rather than inside the transport, where it would \
                 read as the service being unreachable."
            )));
        }
        Ok(Self {
            base: base.trim_end_matches('/').to_owned(),
            agent: ureq::Agent::new_with_defaults(),
        })
    }

    /// `GET /health` — the one call that reads and cannot mutate.
    pub fn health(&self) -> Result<String, RepoError> {
        self.agent
            .get(&format!("{}/health", self.base))
            .call()
            .map_err(|e| RepoError::Message(format!("KF health: {e}")))?
            .body_mut()
            .read_to_string()
            .map_err(|e| RepoError::Message(format!("KF health body: {e}")))
    }

    /// `POST /actions/:action_type` — §67. WRITES to an authoritative record.
    ///
    /// Returns the response body verbatim. OW-WAR-0044's OBL-001 wants a
    /// receipt "where `recorded_at` was assigned BY THE SERVER", so nothing here
    /// stamps a time: whatever KF returns is the record, and a client that
    /// helpfully filled in a timestamp would be manufacturing the very field the
    /// obligation exists to check.
    pub fn post_action(
        &self,
        action_type: &str,
        actor: &Actor,
        envelope: &ActionEnvelope,
    ) -> Result<String, RepoError> {
        if envelope.idempotency_key.len() < 8 {
            return Err(RepoError::Message(
                "idempotencyKey must be at least 8 characters — KF refuses shorter ones, and \
                 failing here names the caller's mistake instead of reporting a 400"
                    .to_owned(),
            ));
        }
        let mut resp = self
            .agent
            .post(&format!("{}/actions/{action_type}", self.base))
            .header("x-kf-actor", &actor.actor)
            .header("x-kf-acting-role", &actor.acting_role)
            .header("x-kf-organization", &actor.organization)
            .send_json(envelope)
            .map_err(|e| RepoError::Message(format!("KF action {action_type}: {e}")))?;
        resp.body_mut()
            .read_to_string()
            .map_err(|e| RepoError::Message(format!("KF action body: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HTTPS is accepted now. This test exists because it did NOT hold for one
    /// commit, and the reason it holds is a recorded licence exception rather
    /// than a code change — so a future narrowing of `deny.toml` that silently
    /// dropped TLS would break here rather than at a remote KF.
    #[test]
    fn an_https_url_is_accepted() {
        assert!(Client::new("https://kf.example.org").is_ok());
    }

    #[test]
    fn a_non_http_url_is_refused() {
        assert!(Client::new("kf.example.org").is_err());
        assert!(Client::new("ftp://kf.example.org").is_err());
        assert!(Client::new("http://127.0.0.1:4000").is_ok());
        assert!(Client::new("https://127.0.0.1:4000").is_ok());
    }

    /// KF's own floor, enforced client-side so the caller learns which field is
    /// wrong rather than reading a 400 body.
    #[test]
    fn a_short_idempotency_key_never_reaches_the_network() {
        let c = Client::new("http://127.0.0.1:9").expect("valid url");
        let err = c
            .post_action(
                "document.create",
                &Actor {
                    actor: "t".into(),
                    acting_role: "r".into(),
                    organization: "o".into(),
                },
                &ActionEnvelope {
                    target_ids: vec![],
                    payload: serde_json::Value::Null,
                    reason: "t".into(),
                    idempotency_key: "short".into(),
                },
            )
            .expect_err("refused before dialling");
        assert!(err.to_string().contains("at least 8 characters"), "{err}");
    }
}
