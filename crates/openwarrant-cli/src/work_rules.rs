// SPDX-License-Identifier: Apache-2.0
//! The rule ids of work tracking (FORMAT-1.0 §11, §13.2).
//!
//! Format 1.0 draft 3 names every refusal of the tracker under `work.`. Each
//! of these was emitted under another name before; for one release a
//! message carries its old name too ([`formerly`]), so a script that
//! matched the old id still finds it while it moves to the new one.

/// A write whose base revision is not the current one (§11.3).
pub const STALE_REVISION: &str = "work.stale-revision";
/// A claim on a target someone else holds (§11.3).
pub const CLAIMED_BY_OTHER: &str = "work.claimed-by-other";
/// The claims remote could not be reached: `UNKNOWN`, never a granted
/// claim (§11.3).
pub const CLAIM_REMOTE_UNREACHABLE: &str = "work.claim-remote-unreachable";
/// A journal whose committed lines were changed or removed (§11.2).
pub const JOURNAL_REWRITTEN: &str = "work.journal-rewritten";
/// A journal line that does not parse (§11.2).
pub const JOURNAL_MALFORMED: &str = "work.journal-malformed";
/// An unterminated last journal line (§11.2).
pub const JOURNAL_TORN: &str = "work.journal-torn";
/// An idempotency key already in the journal from another actor (§11.2).
pub const IDEMPOTENCY_CONFLICT: &str = "work.idempotency-conflict";
/// A journal line of an envelope newer than v2: `UNKNOWN`, not malformed
/// (§11.2).
pub const JOURNAL_NEWER: &str = crate::journal_cmd::NEWER;
/// A tick level the journal does not back, read down to the highest backed
/// one (§11.4): a warning.
pub const TICK_UNBACKED: &str = "work.tick-unbacked";

/// Each renamed rule, `(new, old)`. The old ids are recognized for one
/// release: [`formerly`] names them in the message.
pub const RENAMED: [(&str, &str); 7] = [
    (STALE_REVISION, "warrant.stale-revision"),
    (CLAIMED_BY_OTHER, "ticket.claimed-by-other"),
    (CLAIM_REMOTE_UNREACHABLE, "ticket.claim-remote-unreachable"),
    (JOURNAL_REWRITTEN, "journal.rewritten"),
    (JOURNAL_MALFORMED, "journal.malformed"),
    (JOURNAL_TORN, "journal.torn-tail"),
    (IDEMPOTENCY_CONFLICT, "journal.idempotency-conflict"),
];

/// The old id of a renamed rule.
#[must_use]
pub fn old_id(rule: &str) -> Option<&'static str> {
    RENAMED
        .iter()
        .find(|(new, _)| *new == rule)
        .map(|(_, old)| *old)
}

/// ` (formerly <old id>)` for a renamed rule, empty for any other: what a
/// message ends with for one release.
#[must_use]
pub fn formerly(rule: &str) -> String {
    old_id(rule).map_or_else(String::new, |old| format!(" (formerly {old})"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_renamed_rule_names_its_old_id_and_others_name_nothing() {
        for (new, old) in RENAMED {
            assert!(new.starts_with("work."), "{new}");
            assert_eq!(formerly(new), format!(" (formerly {old})"));
        }
        assert_eq!(formerly(TICK_UNBACKED), "", "a new rule had no old name");
        assert_eq!(formerly("ticket.blocked"), "");
    }
}
