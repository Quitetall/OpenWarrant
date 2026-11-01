// SPDX-License-Identifier: Apache-2.0
//! Regressions create new unverified work; a prior sign-off record is retained.
use super::{
    CreateArgs, Store, Ticket,
    checks::CheckRecord,
    ladder::{self, SignedOff},
};
use crate::repo::RepoError;
use serde_json::json;

const EVENT: &str = "work.repair_proposed";

pub fn propose(
    store: &Store,
    original: &Ticket,
    item: &str,
    signed: &SignedOff,
    verified: bool,
    failed: &CheckRecord,
    run: &str,
) -> Result<String, RepoError> {
    let _lock = super::scopes::AdmissionLock::acquire(store)?;
    let identity = json!({"document":original.manifest.uuid,"item":item,"signer":signed.signer,"statement":signed.digest,"response":signed.response,"at":signed.at});
    let key = openwarrant_core::format::sha256(
        &serde_jcs::to_vec(&identity).map_err(|e| RepoError::Message(e.to_string()))?,
    );
    let journal = crate::journal_cmd::load(&original.dir)?;
    for event in journal.events.iter().filter(|e| e.event_type == EVENT) {
        let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) else {
            continue;
        };
        if payload["repair_key"] == key {
            let id = payload["repair"]
                .as_str()
                .ok_or_else(|| RepoError::Message("prior repair link has no ID".into()))?;
            store.load(&store.dir.join(id)).map_err(|d| {
                RepoError::Message(format!(
                    "linked repair {id} is unavailable; prior history retained: {}",
                    d.message
                ))
            })?;
            return Ok(id.into());
        }
    }
    let target = format!("{}/{item}", original.id());
    let prior = original
        .item(item)
        .ok_or_else(|| RepoError::Message("repair target no longer exists".into()))?;
    let mut title = format!("Repair {}: {}", original.id(), prior.text);
    while title.len() > 120 {
        title.pop();
    }
    let status = if verified {
        "verified-legacy-signoff"
    } else {
        "unestablished"
    };
    let body = format!(
        "Restore the declared outcome of {target}. Preserve its prior completion, sign-off record and evidence.\n\n## REQ-baseline · requirement — Retain the prior result\nx.repairs {target}\n\nPrior sign-off record: `{}`; statement digest `{}`; recorded at `{}`.\nAcceptance evidence: **{status}**. This legacy record binds item text; no accepted code revision is asserted.\n\nCurrent failure: `{}` (run `{run}`). Current scope and required checks are carried forward for proposed repair. The repair is unverified and receives no assurance mark or inherited permission.\n",
        signed.response, signed.digest, signed.at, failed.global
    );
    let created = super::create(
        store,
        &CreateArgs {
            title,
            body: Some(body),
            items: vec![format!("Restore the outcome of {target}")],
            priority: Some(original.manifest.priority),
            kind: original.manifest.kind.clone(),
            labels: original.manifest.labels.clone(),
            ..CreateArgs::default()
        },
    )?;
    if created.is_refused() {
        return Err(RepoError::Message(created.human));
    }
    let id = created.result["id"]
        .as_str()
        .ok_or_else(|| RepoError::Message("repair writer returned no ID".into()))?
        .to_owned();
    let new_item = created.result["items"][0]["id"]
        .as_str()
        .ok_or_else(|| RepoError::Message("repair writer returned no item".into()))?
        .to_owned();
    let draft = store
        .load(&store.dir.join(&id))
        .map_err(|d| RepoError::Message(d.message))?;
    let required: Vec<_> = super::checks::of_ticket(original)
        .into_iter()
        .filter(|c| {
            c.gating
                && (c.evaluates.contains(&target) || c.evaluates.iter().any(|g| g == original.id()))
        })
        .collect();
    let mut parts = Vec::new();
    for (ordinal, check) in required.iter().enumerate() {
        let command = check.run.as_ref().ok_or_else(|| {
            RepoError::Message(format!(
                "{} has no executable; repair remains a draft",
                check.global
            ))
        })?;
        parts.push(json!({"kind":if check.kind=="kpi" {"kpi"} else {"test"},"name":format!("original-{}",ordinal+1),"cmd":command,"item":new_item,"direction":check.direction,"target":check.target,"mode":check.mode,"on":check.on.join(", "),"timeout":check.timeout,"reopen":check.reopen}));
    }
    let minimum = ladder::minimum(store, original, &ladder::checks_of(original), Some(item)).0;
    parts.push(json!({"kind":"milestone","item":new_item,"min":minimum.as_str()}));
    super::authoring::append_parts(store, &draft, &parts)?;
    let paths = crate::go::packet::allowed_of_ticket(original).paths;
    if !paths.is_empty() {
        super::rewrite(&draft.intent_path, |text| {
            let list = paths
                .iter()
                .map(|p| format!("  - {}\n", serde_json::to_string(p).unwrap_or_default()))
                .collect::<String>();
            let next = text.replacen("\n---\n", &format!("\npaths:\n{list}---\n"), 1);
            super::authoring::validate(&next).map_err(|why| {
                Box::new(super::Outcome::refused(
                    "repair.scope-invalid",
                    store.rel(&draft.intent_path),
                    why,
                ))
            })?;
            Ok((next, ()))
        })?
        .map_err(|e| RepoError::Message(e.human))?;
    }
    store.journal(original, EVENT, &json!({"repair_key":key,"target":target,"repair":id,"prior_signoff":signed,"acceptance_status":status,"accepted_code_revision":null,"failing_check":failed.global,"failing_run":run,"qualification_established":false}))?;
    Ok(id)
}
