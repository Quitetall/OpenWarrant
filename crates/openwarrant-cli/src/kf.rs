// SPDX-License-Identifier: AGPL-3.0-or-later
//! `war kf` — the §67 Knowledge Fabric seam.
//!
//! # This build cannot speak HTTPS, and says so
//!
//! `ureq` is compiled WITHOUT a TLS backend. That is a license decision, not an
//! oversight: every TLS configuration of ureq 3.4 pulls Mozilla's CA bundle
//! (`webpki-roots` or `webpki-root-certs`) under `CDLA-Permissive-2.0`, which
//! `deny.toml`'s permissive-only allowlist rejects. The allowlist's own comment
//! says adding a license "is a decision about whether OpenWarrant can still be
//! relicensed afterwards", and that decision belongs to whoever owns the
//! Apache-2.0 path, not to the commit that needed one HTTP request.
//!
//! So [`Client::new`] REFUSES any URL this build cannot honestly serve, naming
//! the missing feature. A client that accepts an `https://` URL and then fails
//! inside the transport teaches the operator that KF is down; this one says the
//! binary cannot reach it.
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
        if base.starts_with("https://") {
            return Err(RepoError::Message(format!(
                "this build cannot reach {base}: `ureq` is compiled without a TLS backend. \
                 Every TLS configuration of ureq 3.4 pulls Mozilla's CA bundle under \
                 CDLA-Permissive-2.0, which deny.toml's permissive-only allowlist rejects. \
                 Enabling TLS means adding a narrow `exceptions` entry for the CA-bundle \
                 crate — a decision about the Apache-2.0 relicense path, recorded \
                 deliberately or not at all."
            )));
        }
        if !base.starts_with("http://") {
            return Err(RepoError::Message(format!("not an http(s) URL: {base}")));
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

    /// The refusal is the feature. An `https://` URL must fail with a message
    /// about THIS BUILD, not about the network.
    #[test]
    fn an_https_url_is_refused_by_name() {
        let err = Client::new("https://kf.example.org").expect_err("no TLS in this build");
        let msg = err.to_string();
        assert!(msg.contains("without a TLS backend"), "{msg}");
        assert!(
            msg.contains("CDLA-Permissive-2.0"),
            "the refusal must name WHY, so it is fixable: {msg}"
        );
    }

    #[test]
    fn a_non_http_url_is_refused() {
        assert!(Client::new("kf.example.org").is_err());
        assert!(Client::new("ftp://kf.example.org").is_err());
        assert!(Client::new("http://127.0.0.1:4000").is_ok());
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
