//! The command that answers a diagnostic (§76.2, OW-WAR-0112).
//!
//! §76.2: "Errors SHALL identify … likely remediation." Before this module
//! twenty-six of four hundred and forty-seven rules did, as prose inside the
//! message, and a reader — human or agent — had to find the backticks. Now
//! every diagnostic is asked one question, `remedy_for`, and the answer is
//! structured: an argv, what running it accomplishes, and WHOSE act it is.
//!
//! # The kinds
//!
//! - `Auto`: a tool may run it unasked. It reads, recomputes, or refreshes a
//!   record nobody has signed. **Never** a signing act — `sign`, `authorize`,
//!   `resolve`, `correct`, `accept`, `answer` — and a unit test below asserts
//!   that over the whole table, mirroring `next.rs`'s "an agent is never
//!   handed a signing act".
//! - `Human`: the command is right, and only a human may run it, because it
//!   ends in a signature or drafts the request for one.
//! - `Informational`: the command shows more — it does not fix. A verifier
//!   that failed is not remedied by a command; it is understood by one.
//!
//! # Where the table ends
//!
//! Rules not in the table fall back to the message: the first backticked
//! `war …` in it becomes the argv, classified by its verb. So a rule whose
//! author wrote the remedy in prose is not silent here, and a rule with no
//! remedy in either place answers `None` — honestly, rather than with a
//! `war check` that would send the reader in a circle.

use crate::diagnostic::{Diagnostic, Severity};
use serde::Serialize;

/// Whose act the remedy is.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Auto,
    Human,
    Informational,
}

impl Kind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Human => "human",
            Self::Informational => "info",
        }
    }
}

/// One remedy: the command, verbatim as argv, and what it accomplishes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Remedy {
    pub kind: Kind,
    pub argv: Vec<String>,
    pub purpose: String,
}

impl Remedy {
    fn new(kind: Kind, argv: &[&str], purpose: impl Into<String>) -> Self {
        Self {
            kind,
            argv: argv.iter().map(|s| (*s).to_owned()).collect(),
            purpose: purpose.into(),
        }
    }

    /// The argv as a shell line, for the human rendering and the app's
    /// status bar.
    #[must_use]
    pub fn command(&self) -> String {
        self.argv.join(" ")
    }
}

/// The verbs an `Auto` remedy may never carry (§27.2, §51): each ends in a
/// human's signature or drafts the request for one.
pub const SIGNING_VERBS: &[&str] = &[
    "sign",
    "authorize",
    "resolve",
    "correct",
    "accept",
    "answer",
];

/// The rules the table answers directly. Public so a test can walk them.
pub const TABLE: &[&str] = &[
    "generated.drift",
    "generated.missing",
    "generated.compile",
    "relations.child-listed",
    "deliverable.pin-stale",
    "deliverable.digest-drift",
    "deliverable.target-unreadable",
    "correction.new-digest-mismatch",
    "pins.signed",
    "authority.unsigned",
    "dispatch.unauthorized",
    "authorize.deliverables-unsigned",
    "authorize.no-amendment",
    "sas.pin-unknown",
    "sas.proposed-unaccepted",
    "schemas.drift",
    "schemas.missing",
    "evidence.stale-binding",
    "evidence.invalid",
    "evidence.not-a-pass",
    "evidence.receipt-invalid",
    "attest.failed",
    "attest.subject-drift",
    "attest.unchecked",
    "verify.inadmissible",
    "verify.verifier-failed",
    "verify.verifier-timeout",
    "resolution.requirement-unmet",
    "resolution.requirements-unmet",
    "resolution.deliverable-moved",
    "resolution.stale-digest",
    "doctor.authority-absent",
    "doctor.dependencies",
    "doctor.performer",
    "doctor.verifier",
    "journal.not-empty",
];

/// The remedy for one diagnostic, or `None` when nothing answers it. A
/// `Pass` never has one: there is nothing to do.
#[must_use]
pub fn remedy_for(d: &Diagnostic) -> Option<Remedy> {
    if d.severity == Severity::Pass {
        return None;
    }
    let alias = alias_of(&d.message).or_else(|| d.file.as_deref().and_then(alias_in_path));
    let deliverable = deliverable_of(&d.message);
    let a = alias.as_deref().unwrap_or("<alias>");
    let dl = deliverable.as_deref().unwrap_or("<D-id>");
    let sub = format!("{a}/{dl}");

    let r = match d.rule.as_str() {
        "generated.drift"
        | "generated.missing"
        | "generated.compile"
        | "relations.child-listed" => Remedy::new(
            Kind::Auto,
            &["war", "compile"],
            "regenerate every projection from its source and commit the result",
        ),
        "deliverable.pin-stale" => Remedy::new(
            Kind::Auto,
            &["war", "pins", "--refresh", "--alias", a],
            "re-record the bytes this unresolved Warrant's pins describe; nobody has signed \
             over them, so the pin is a note, not a promise",
        ),
        "deliverable.digest-drift" | "correction.new-digest-mismatch" | "pins.signed" => {
            Remedy::new(
                Kind::Human,
                &["war", "correct", a, dl],
                format!(
                    "draft the correction that records why a resolved deliverable moved; a \
                     human then signs it with `war sign {sub} --ssh-sign`"
                ),
            )
        }
        "deliverable.target-unreadable" => Remedy::new(
            Kind::Informational,
            &[
                "war",
                "pins",
                "--history",
                &path_of(&d.message).unwrap_or_default(),
            ],
            "see which Warrant last delivered the path and at which commit its bytes verify",
        ),
        "authority.unsigned" | "dispatch.unauthorized" | "authorize.deliverables-unsigned" => {
            Remedy::new(
                Kind::Human,
                &["war", "sign", a, "--ssh-sign"],
                "a human authorizes the Warrant; try `--dry-run` first to see whether the \
                 ingest would accept it",
            )
        }
        "authorize.no-amendment" => Remedy::new(
            Kind::Informational,
            &["war", "diff", a],
            "see what moved the contract; §31 wants an amendment record under amendments/ \
             before revision N+1 is signed",
        ),
        "sas.pin-unknown" | "sas.proposed-unaccepted" => Remedy::new(
            Kind::Informational,
            &["war", "sas", "status"],
            "see which SAS revisions are recorded, proposed and in force",
        ),
        "schemas.drift" | "schemas.missing" => Remedy::new(
            Kind::Auto,
            &[
                "cargo",
                "run",
                "-q",
                "-p",
                "openwarrant-cli",
                "--features",
                "schema",
                "--",
                "schemas",
            ],
            "regenerate the JSON Schema pack from the record types and commit it",
        ),
        "evidence.stale-binding"
        | "evidence.invalid"
        | "evidence.not-a-pass"
        | "evidence.receipt-invalid" => Remedy::new(
            Kind::Auto,
            &["war", "evidence", "record", a],
            "run the cited gates again and mint receipts bound to the contract as it is now",
        ),
        "attest.failed" | "attest.subject-drift" | "attest.unchecked" => Remedy::new(
            Kind::Informational,
            &["war", "attest", a, "--verify"],
            "verify every signature and subject digest of this Warrant's attestations \
             against the tree today",
        ),
        "verify.inadmissible"
        | "verify.verifier-failed"
        | "verify.verifier-timeout"
        | "resolution.requirement-unmet"
        | "resolution.requirements-unmet"
        | "resolution.deliverable-moved"
        | "resolution.stale-digest" => Remedy::new(
            Kind::Informational,
            &["war", "resolve", a, "--dry-run"],
            "see every §56.1 requirement by name and which are unmet, without recording \
             anything",
        ),
        "doctor.authority-absent"
        | "doctor.dependencies"
        | "doctor.performer"
        | "doctor.verifier" => Remedy::new(
            Kind::Informational,
            &["war", "doctor"],
            "check each component of this installation and what it is missing",
        ),
        "journal.not-empty" => Remedy::new(
            Kind::Informational,
            &["war", "journal", a],
            "read the events already on the journal before asking for a backfill",
        ),
        _ => return from_message(&d.message),
    };
    // A table entry that needed an alias or a deliverable the finding did not
    // name would be a command with a hole in it. Fall back to the message
    // rather than hand anyone `war sign <alias>`.
    if r.argv.iter().any(|t| t.starts_with('<')) {
        return from_message(&d.message);
    }
    Some(r)
}

/// The fallback: the first backticked `war …` in the message, classified by
/// its verb. Placeholders such as `<D-id>` are kept as they are — a reader
/// substitutes; a tool must not guess.
fn from_message(message: &str) -> Option<Remedy> {
    let mut rest = message;
    while let Some(start) = rest.find("`war ") {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        let line = &after[..end];
        let argv: Vec<&str> = line.split_whitespace().collect();
        if argv.len() >= 2 {
            let kind = classify(&argv);
            return Some(Remedy::new(kind, &argv, "as the diagnostic says"));
        }
        rest = &after[end + 1..];
    }
    None
}

/// `war sign …` and every act that ends in a signature is a human's; `war
/// compile` and `war pins --refresh` are safe to run unasked; anything else
/// is shown, not run.
fn classify(argv: &[&str]) -> Kind {
    let verb = argv.get(1).copied().unwrap_or("");
    let dry = argv.contains(&"--dry-run");
    if SIGNING_VERBS.contains(&verb) && !dry {
        return Kind::Human;
    }
    if verb == "sas"
        && argv
            .get(2)
            .is_some_and(|v| *v == "accept" || *v == "propose")
    {
        return Kind::Human;
    }
    match verb {
        "compile" => Kind::Auto,
        "pins" if argv.contains(&"--refresh") => Kind::Auto,
        _ => Kind::Informational,
    }
}

/// `OW-WAR-0001: …` → `OW-WAR-0001`. The alias is the first token when it
/// looks like one; a message that opens with a path or a rule has none.
fn alias_of(message: &str) -> Option<String> {
    let head = message.split(':').next()?.trim();
    let looks =
        head.contains("-WAR-") && head.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    looks.then(|| head.to_owned())
}

/// `docs/warrants/OW-WAR-0001/authorization.toml` → `OW-WAR-0001`: the file
/// a finding is anchored to names the Warrant when the message does not.
fn alias_in_path(file: &str) -> Option<String> {
    file.split('/')
        .find(|seg| {
            seg.contains("-WAR-") && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
        .map(str::to_owned)
}

/// The first `D-nnn` token in the message.
fn deliverable_of(message: &str) -> Option<String> {
    message
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .find(|t| t.len() > 2 && t.starts_with("D-") && t[2..].chars().all(|c| c.is_ascii_digit()))
        .map(str::to_owned)
}

/// The first token that looks like a repository-relative path.
fn path_of(message: &str) -> Option<String> {
    message
        .split_whitespace()
        .map(|t| t.trim_matches(|c: char| c == ',' || c == ';' || c == '.' || c == '`'))
        .find(|t| t.contains('/') && !t.contains("://"))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(rule: &str) -> Diagnostic {
        Diagnostic::error(
            rule,
            "docs/warrants/OW-WAR-0001/deliverables.toml".to_owned(),
            "OW-WAR-0001: D-002 records sha256:aa for crates/x.rs but the file is now \
             sha256:bb — `war correct OW-WAR-0001 D-002`",
        )
    }

    #[test]
    fn no_auto_remedy_is_a_signing_act() {
        for rule in TABLE {
            let Some(r) = remedy_for(&diag(rule)) else {
                panic!("{rule} is in the table and answered nothing");
            };
            if r.kind == Kind::Auto {
                let verb = r.argv.get(1).map(String::as_str).unwrap_or("");
                assert!(
                    !SIGNING_VERBS.contains(&verb) || r.argv[0] != "war",
                    "{rule}: an Auto remedy carries a signing verb: {}",
                    r.command()
                );
                assert!(
                    !r.command().contains("--ssh-sign"),
                    "{rule}: {}",
                    r.command()
                );
            }
        }
    }

    #[test]
    fn a_pass_has_no_remedy() {
        assert!(remedy_for(&Diagnostic::pass("generated.drift", "fine")).is_none());
    }

    #[test]
    fn drift_is_a_human_correction_naming_alias_and_deliverable() {
        let r = remedy_for(&diag("deliverable.digest-drift")).unwrap();
        assert_eq!(r.kind, Kind::Human);
        assert_eq!(r.argv, ["war", "correct", "OW-WAR-0001", "D-002"]);
        assert!(r.purpose.contains("war sign OW-WAR-0001/D-002 --ssh-sign"));
    }

    #[test]
    fn the_fallback_reads_the_message_and_classifies_the_verb() {
        let d = Diagnostic::warn(
            "some.rule",
            "x".to_owned(),
            "fix it with `war pins --refresh --alias OW-WAR-0009`, or look at `war show OW-WAR-0009`",
        );
        let r = remedy_for(&d).unwrap();
        assert_eq!(r.kind, Kind::Auto);
        assert_eq!(
            r.argv,
            ["war", "pins", "--refresh", "--alias", "OW-WAR-0009"]
        );

        let human = Diagnostic::error(
            "x.y",
            "f".to_owned(),
            "then `war sign OW-WAR-0009 --ssh-sign`",
        );
        assert_eq!(remedy_for(&human).unwrap().kind, Kind::Human);
        let dry = Diagnostic::error(
            "x.y",
            "f".to_owned(),
            "see `war resolve OW-WAR-0009 --dry-run`",
        );
        assert_eq!(remedy_for(&dry).unwrap().kind, Kind::Informational);
        let none = Diagnostic::error("x.y", "f".to_owned(), "no command here");
        assert!(remedy_for(&none).is_none());
    }

    #[test]
    fn alias_and_deliverable_extraction_are_conservative() {
        assert_eq!(
            alias_of("OW-WAR-0112: D-011 moved").as_deref(),
            Some("OW-WAR-0112")
        );
        assert_eq!(alias_of("docs/x.md: bad").as_deref(), None);
        assert_eq!(alias_of("subject drift").as_deref(), None);
        assert_eq!(
            alias_in_path("docs/warrants/OW-WAR-0007/authorization.toml").as_deref(),
            Some("OW-WAR-0007")
        );
        let anchored = Diagnostic::error(
            "authority.unsigned",
            "docs/warrants/OW-WAR-0007/authorization.toml".to_owned(),
            "no signature over this record",
        );
        assert_eq!(
            remedy_for(&anchored).unwrap().argv,
            ["war", "sign", "OW-WAR-0007", "--ssh-sign"]
        );
        assert_eq!(
            deliverable_of("OW-WAR-0112: D-011 moved").as_deref(),
            Some("D-011")
        );
        assert_eq!(deliverable_of("nothing D-x here").as_deref(), None);
    }
}
