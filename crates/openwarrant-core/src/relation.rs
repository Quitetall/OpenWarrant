// SPDX-License-Identifier: Apache-2.0
//! Relation kinds, and the record and relation vocabulary a profile declares
//! (OW-WAR-0148 M3; OW-ADR-0031).
//!
//! A relation is `{from, kind, to}`, where `to` may pin a revision of its
//! target (`REQ-pr1@sha256:<hex>`). Kinds come in two families:
//!
//! - **Core** — a closed set the kernel computes from: `part_of`,
//!   `depends_on`, `implements`, `constrains`, `evaluates`, `supersedes`,
//!   and the rationale edges of `rationale.rs` (§35.7): `supports`,
//!   `refutes`, `trades_off_against`, `causes`, `qualifies`,
//!   `selected_over` (`constrains` and `depends_on` are in both). A profile
//!   names the core kinds its records may use; a core kind it does not name
//!   is refused.
//! - **Namespaced** — `<namespace>.<name>` (`x.mentions`,
//!   `contractor.bills`). Carried and displayed, never declared and never
//!   computed from: a namespaced kind drives no state, no readiness and no
//!   check, and `war plan impact` does not walk it. A profile cannot give one
//!   kernel meaning.
//!
//! A word that is neither is refused: a relation line is a claim, and a kind
//! the kernel cannot place is a typo or an invention, not prose.
//!
//! The record types a profile composes are its own nouns (`requirement`,
//! `outcome`, …). The kernel knows two record types, the two its capabilities
//! compute on — `obligation` (an assurance atom's) and `item` (a ticket
//! checklist's) — and no profile declares either.
//!
//! Pure: values, parses and validation. No I/O (§79.1).

use std::collections::BTreeSet;
use std::fmt;

use crate::rationale::RationaleEdge;

/// The record types the kernel itself knows (OW-ADR-0031): the two its
/// capabilities compute on. Every other record type is a profile noun.
pub const KERNEL_RECORD_TYPES: [&str; 2] = ["obligation", "item"];

/// The closed set of core relation kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CoreKind {
    PartOf,
    DependsOn,
    Implements,
    Constrains,
    Evaluates,
    Supersedes,
    Supports,
    Refutes,
    TradesOffAgainst,
    Causes,
    Qualifies,
    SelectedOver,
}

impl CoreKind {
    /// Every core kind: the six structural kinds, then the rationale edges
    /// not already among them.
    pub const ALL: [Self; 12] = [
        Self::PartOf,
        Self::DependsOn,
        Self::Implements,
        Self::Constrains,
        Self::Evaluates,
        Self::Supersedes,
        Self::Supports,
        Self::Refutes,
        Self::TradesOffAgainst,
        Self::Causes,
        Self::Qualifies,
        Self::SelectedOver,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PartOf => "part_of",
            Self::DependsOn => "depends_on",
            Self::Implements => "implements",
            Self::Constrains => "constrains",
            Self::Evaluates => "evaluates",
            Self::Supersedes => "supersedes",
            Self::Supports => "supports",
            Self::Refutes => "refutes",
            Self::TradesOffAgainst => "trades_off_against",
            Self::Causes => "causes",
            Self::Qualifies => "qualifies",
            Self::SelectedOver => "selected_over",
        }
    }

    /// A core kind by name.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == name)
    }

    /// The rationale edge (§35.7) this kind is, if it is one.
    #[must_use]
    pub fn rationale_edge(self) -> Option<RationaleEdge> {
        RationaleEdge::ALL
            .iter()
            .copied()
            .find(|e| e.as_str() == self.as_str())
    }

    /// Every core kind's name, for a message that helps.
    #[must_use]
    pub fn known() -> String {
        Self::ALL.map(Self::as_str).join(", ")
    }
}

impl fmt::Display for CoreKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A relation kind as authored.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelationKind {
    Core(CoreKind),
    /// `<namespace>.<name>`: carried, inert.
    Namespaced(String),
}

/// Why a relation kind was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KindError {
    #[error(
        "`{0}` is not a relation kind: a core kind is one of {known}, and an extension kind \
         is namespaced (`<namespace>.<name>`, e.g. `x.mentions`)",
        known = CoreKind::known()
    )]
    Unknown(String),
}

fn is_word(s: &str) -> bool {
    let mut c = s.chars();
    c.next().is_some_and(|c| c.is_ascii_lowercase())
        && c.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

impl RelationKind {
    /// A core kind by name, or a namespaced one; anything else is refused.
    pub fn parse(name: &str) -> Result<Self, KindError> {
        if let Some(core) = CoreKind::parse(name) {
            return Ok(Self::Core(core));
        }
        if name.contains('.') && name.split('.').all(is_word) {
            return Ok(Self::Namespaced(name.to_owned()));
        }
        Err(KindError::Unknown(name.to_owned()))
    }

    /// Whether this kind drives nothing (a namespaced kind).
    #[must_use]
    pub const fn is_inert(&self) -> bool {
        matches!(self, Self::Namespaced(_))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Core(k) => k.as_str(),
            Self::Namespaced(s) => s,
        }
    }
}

impl fmt::Display for RelationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether `id` is a record id: an uppercase letter, then uppercase letters
/// or digits, then one or more `-<alphanumerics>` groups, at most 64 bytes.
/// `REQ-pr1`, `OUT-001`, `OW-WAR-0148`, `DEC-auth-2` are; `req-1`, `REQ`,
/// `REQ-`, `REQ--1` are not.
#[must_use]
pub fn is_record_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 64 {
        return false;
    }
    let mut groups = id.split('-');
    let Some(head) = groups.next() else {
        return false;
    };
    let mut hc = head.chars();
    if !hc.next().is_some_and(|c| c.is_ascii_uppercase())
        || !hc.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return false;
    }
    let mut tail = 0;
    for g in groups {
        if g.is_empty() || !g.chars().all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
        tail += 1;
    }
    tail > 0
}

fn is_ticket_id(s: &str, prefix: char) -> bool {
    s.strip_prefix(prefix)
        .and_then(|r| r.strip_prefix('-'))
        .is_some_and(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()))
}

/// Whether `target` can name a record of the compiled model: a record id
/// (`REQ-pr1`), a Warrant's own record (`OW-WAR-0148/OBL-001`), a ticket
/// (`t-3f2a`), a ticket's item (`t-3f2a/i-9c01`), or a section of an
/// instruction file (`md:CLAUDE.md#testing`, [`crate::instruction`]).
#[must_use]
pub fn is_target(target: &str) -> bool {
    if target.starts_with(crate::instruction::ID_PREFIX) {
        return crate::instruction::is_section_id(target);
    }
    match target.split_once('/') {
        None => is_record_id(target) || is_ticket_id(target, 't'),
        Some((a, b)) => {
            (is_record_id(a) && is_record_id(b)) || (is_ticket_id(a, 't') && is_ticket_id(b, 'i'))
        }
    }
}

/// A relation's target as written: the id, and the revision it pins.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub id: String,
    /// `sha256:<64 hex>` when the relation pins a revision of its target.
    pub pin: Option<String>,
}

/// Parse `ID` or `ID@sha256:<64 lowercase hex>`. `None` when it is neither.
#[must_use]
pub fn parse_target(token: &str) -> Option<Target> {
    let (id, pin) = match token.split_once('@') {
        None => (token, None),
        Some((id, pin)) => {
            let hex = pin.strip_prefix("sha256:")?;
            if hex.len() != 64 || !hex.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')) {
                return None;
            }
            (id, Some(pin.to_owned()))
        }
    };
    is_target(id).then(|| Target {
        id: id.to_owned(),
        pin,
    })
}

/// `implements <target>` phrases in free text (a ticket item's line): each
/// `implements` word followed by a target, surrounding punctuation trimmed.
/// Only `implements` is read from prose: it is intended coverage and never
/// satisfaction, and every other kind is authored on a relation line.
#[must_use]
pub fn implements_phrases(text: &str) -> Vec<Target> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let trim = |w: &str| -> String {
        w.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | ',' | ';' | ':' | '.' | '`'))
            .to_owned()
    };
    let mut out = Vec::new();
    for pair in words.windows(2) {
        if trim(pair[0]) == "implements"
            && let Some(t) = parse_target(&trim(pair[1]))
            && !out.contains(&t)
        {
            out.push(t);
        }
    }
    out
}

/// A relation a profile requires: every record of type `from` has at least
/// one `kind` relation to a record of type `to`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequiredRelation {
    pub from: String,
    pub kind: CoreKind,
    pub to: String,
}

/// The record types a profile composes and the core relation kinds it
/// allows and requires. Empty for a profile that declares none: the kernel
/// knows no profile nouns, so a record of any type under it is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Vocabulary {
    pub types: BTreeSet<String>,
    pub allow: BTreeSet<CoreKind>,
    pub require: Vec<RequiredRelation>,
}

impl Vocabulary {
    /// Build and check a profile's declaration. `Err` is the detail of the
    /// refusal: a type that is not a lowercase word, a kernel type, a name
    /// listed twice, a kind outside the core set (a namespaced kind needs no
    /// declaration), and a required relation over an undeclared type or
    /// kind.
    pub fn declare(
        types: &[String],
        allow: &[String],
        require: &[[String; 3]],
    ) -> Result<Self, String> {
        let mut v = Self::default();
        for t in types {
            if !is_word(t) {
                return Err(format!(
                    "record type {t:?} is not a lowercase word ([a-z][a-z0-9_-]*)"
                ));
            }
            if KERNEL_RECORD_TYPES.contains(&t.as_str()) {
                return Err(format!(
                    "record type `{t}` is a kernel type; the kernel knows `obligation` and \
                     `item`, and a profile declares only its own nouns"
                ));
            }
            if !v.types.insert(t.clone()) {
                return Err(format!("record type `{t}` is listed twice"));
            }
        }
        for k in allow {
            match RelationKind::parse(k) {
                Ok(RelationKind::Core(core)) => {
                    if !v.allow.insert(core) {
                        return Err(format!("relation kind `{k}` is listed twice"));
                    }
                }
                Ok(RelationKind::Namespaced(_)) => {
                    return Err(format!(
                        "relation kind `{k}` is namespaced; a namespaced kind is carried and \
                         inert, and needs no declaration"
                    ));
                }
                Err(e) => return Err(e.to_string()),
            }
        }
        for [from, kind, to] in require {
            let Some(core) = CoreKind::parse(kind) else {
                return Err(format!(
                    "required relation [{from}, {kind}, {to}]: `{kind}` is not a core kind ({})",
                    CoreKind::known()
                ));
            };
            if !v.allow.contains(&core) {
                return Err(format!(
                    "required relation [{from}, {kind}, {to}]: `{kind}` is not in `allow`"
                ));
            }
            for t in [from, to] {
                if !v.types.contains(t) && !KERNEL_RECORD_TYPES.contains(&t.as_str()) {
                    return Err(format!(
                        "required relation [{from}, {kind}, {to}]: `{t}` is not a declared \
                         record type"
                    ));
                }
            }
            v.require.push(RequiredRelation {
                from: from.clone(),
                kind: core,
                to: to.clone(),
            });
        }
        Ok(v)
    }

    /// Whether the profile declares nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.types.is_empty() && self.allow.is_empty() && self.require.is_empty()
    }

    /// Whether a record of this profile may be of `ty`.
    #[must_use]
    pub fn has_type(&self, ty: &str) -> bool {
        self.types.contains(ty)
    }

    /// Whether a relation of this profile may be of `kind`. A namespaced
    /// kind always may: it means nothing to the kernel.
    #[must_use]
    pub fn allows(&self, kind: &RelationKind) -> bool {
        match kind {
            RelationKind::Core(k) => self.allow.contains(k),
            RelationKind::Namespaced(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_kinds_cover_the_structural_six_and_every_rationale_edge() {
        for name in [
            "part_of",
            "depends_on",
            "implements",
            "constrains",
            "evaluates",
            "supersedes",
        ] {
            assert!(CoreKind::parse(name).is_some(), "{name}");
        }
        for edge in RationaleEdge::ALL {
            let k = CoreKind::parse(edge.as_str()).expect("every rationale edge is core");
            assert_eq!(k.rationale_edge(), Some(*edge));
        }
        assert_eq!(CoreKind::ALL.len(), 12);
    }

    #[test]
    fn a_namespaced_kind_is_inert_and_a_bare_unknown_word_is_refused() {
        let k = RelationKind::parse("x.mentions").unwrap();
        assert!(k.is_inert());
        assert!(!RelationKind::parse("implements").unwrap().is_inert());
        assert!(matches!(
            RelationKind::parse("mentions"),
            Err(KindError::Unknown(_))
        ));
        assert!(RelationKind::parse("X.mentions").is_err());
        assert!(RelationKind::parse("x.").is_err());
    }

    #[test]
    fn record_ids_and_targets() {
        for ok in ["REQ-pr1", "OUT-001", "OW-WAR-0148", "DEC-auth-2", "A-1"] {
            assert!(is_record_id(ok), "{ok}");
        }
        for bad in ["req-1", "REQ", "REQ-", "REQ--1", "REQ-p_1", "-REQ-1", ""] {
            assert!(!is_record_id(bad), "{bad}");
        }
        for ok in [
            "REQ-pr1",
            "OW-WAR-0148/OBL-001",
            "t-3f2a",
            "t-3f2a/i-9c01",
            "md:CLAUDE.md#testing",
            "md:crates/x/CLAUDE.md#build",
        ] {
            assert!(is_target(ok), "{ok}");
        }
        for bad in [
            "t-xyz",
            "REQ-1/",
            "t-3f2a/OBL-1",
            "everything",
            "md:CLAUDE.md",
            "md:../CLAUDE.md#x",
        ] {
            assert!(!is_target(bad), "{bad}");
        }
        let pin = format!("REQ-pr1@sha256:{}", "a".repeat(64));
        assert_eq!(
            parse_target(&pin).unwrap().pin.as_deref(),
            Some(format!("sha256:{}", "a".repeat(64)).as_str())
        );
        assert!(parse_target("REQ-pr1@sha256:abc").is_none());
        assert!(parse_target("REQ-pr1@md5:abc").is_none());
        let md = parse_target(&format!("md:CLAUDE.md#testing@sha256:{}", "c".repeat(64))).unwrap();
        assert_eq!(md.id, "md:CLAUDE.md#testing");
        assert!(md.pin.is_some());
    }

    #[test]
    fn implements_phrases_read_only_implements() {
        let t = implements_phrases("Expire tokens (implements REQ-pr1); constrains CON-pr1");
        assert_eq!(
            t,
            vec![Target {
                id: "REQ-pr1".into(),
                pin: None
            }]
        );
        assert!(implements_phrases("implements the reset flow").is_empty());
    }

    #[test]
    fn a_vocabulary_is_checked_when_declared() {
        let s = |v: &[&str]| v.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        let v = Vocabulary::declare(
            &s(&["outcome", "requirement"]),
            &s(&["implements"]),
            &[["requirement".into(), "implements".into(), "outcome".into()]],
        )
        .unwrap();
        assert!(v.has_type("requirement") && !v.has_type("risk"));
        assert!(v.allows(&RelationKind::Core(CoreKind::Implements)));
        assert!(!v.allows(&RelationKind::Core(CoreKind::Supersedes)));
        assert!(v.allows(&RelationKind::Namespaced("x.mentions".into())));
        for (types, allow, why) in [
            (s(&["item"]), s(&[]), "kernel type"),
            (s(&["Outcome"]), s(&[]), "lowercase word"),
            (s(&["a", "a"]), s(&[]), "listed twice"),
            (s(&[]), s(&["x.mentions"]), "namespaced"),
            (s(&[]), s(&["mentions"]), "not a relation kind"),
        ] {
            let err = Vocabulary::declare(&types, &allow, &[]).unwrap_err();
            assert!(err.contains(why), "{err}");
        }
        let err = Vocabulary::declare(
            &s(&["requirement"]),
            &s(&["implements"]),
            &[["requirement".into(), "implements".into(), "outcome".into()]],
        )
        .unwrap_err();
        assert!(
            err.contains("`outcome` is not a declared record type"),
            "{err}"
        );
    }
}
