// SPDX-License-Identifier: Apache-2.0
//! A standing authorization (OW-ADR-0029, SAS §28.8 as proposed in 1.2.0):
//! one human signature over a closed CLASS of routine work, and the
//! deterministic check that says whether one Warrant is inside it.
//!
//! # What this module is, and what it refuses to be
//!
//! The class is a record the owner signs once: the paths a covered Warrant
//! may declare, `delivery` at `basic`, the gates its obligations may cite, a
//! budget per stage, an expiry and a count. [`covers`] compares a compiled
//! contract with the class term by term and returns one [`Refusal`] per term
//! broken. It reads no file, no clock and no network: every input is an
//! argument, so the same class and the same contract always give the same
//! answer (§30.4 — a check that needs judgment has the wrong term).
//!
//! # The terms are closed
//!
//! A class states every term or it is refused ([`Refusal::MissingTerm`]). A
//! term the record does not know is refused, not ignored
//! ([`Refusal::UnknownTerm`]) — which is how a class carrying a resolution
//! term, an ADR term or a residual-risk term is refused: there are no such
//! terms. A covered Warrant is therefore never resolved by a class, never
//! carries an ADR and never accepts a risk.
//!
//! # The paths no class may cover
//!
//! [`NEVER_COVERABLE`] is a constant, compiled in. Nothing reads it from a
//! file, and nothing extends or shrinks it at run time: the class cannot
//! reach the code that checks the class, and neither can an agent that
//! edits a configuration file. `conformance/plants.d/69-standing.sh` greps
//! this file for any read that could extend it and fails if one exists.
//!
//! A class glob is refused when it could match one of these paths
//! ([`class_refusals`]). An ANCHORED entry (`docs/authority/**`) is compared
//! glob against glob: any path both could match refuses the class. An entry
//! that holds at any depth (`**/Cargo.toml`) cannot be compared that way
//! without refusing every glob ending in `**`, so a class glob is refused for
//! it only when its own last segment could name that file; the declared path
//! itself is held to the whole set again when a Warrant is checked, so a
//! `Cargo.toml` that appears under a covered directory later is still never
//! covered.

use serde::{Deserialize, Serialize};

/// The record's schema id.
pub const SCHEMA: &str = "oh.war/standing-authorization/v1";

/// The `policy_basis` scheme a covered authorization records.
pub const SCHEME: &str = "standing://";

/// Q-002 as drafted: at most 90 days between the signature and the expiry.
pub const MAX_EXPIRY_SECONDS: i64 = 90 * 86_400;
/// Q-002 as drafted: at most 50 covered Warrants per class revision.
pub const MAX_WARRANTS: u32 = 50;
/// Q-002 as drafted: a stage's `budget_tokens` ceiling.
pub const MAX_BUDGET_TOKENS: u64 = 24_000;
/// Q-002 as drafted: a stage's `wall_time_seconds` ceiling.
pub const MAX_WALL_TIME_SECONDS: u64 = 1_800;

/// Every class names these gates (OW-ADR-0029): the battery and `war check`.
/// The battery is cited at `@1.1.0`, the askable version OW-WAR-0142 AM-001
/// moved every citation to; `@1.0.0` can never produce a receipt.
pub const REQUIRED_GATES: &[&str] = &[
    "gate://ops.conformance.plants@1.1.0",
    "gate://software.repo.war-check@1.0.0",
];

/// The paths no class may cover (OW-ADR-0029), as globs over
/// repository-relative paths. `*` matches within one segment, `**` any
/// number of segments.
///
/// Not read from anywhere. A change to this list is a change to the code
/// that decides authority, and this file is itself on it.
pub const NEVER_COVERABLE: &[&str] = &[
    // The authority files.
    "docs/authority/**",
    "openwarrant.toml",
    "docs/sas/**",
    "docs/roadmap/**",
    "docs/adr/**",
    // The gates and their fixtures.
    "docs/gates/**",
    "conformance/**",
    // The guards.
    ".claude/hooks/**",
    ".claude-plugin/**",
    // Dependency manifests, at any depth (a new production dependency is
    // §30.3's manual band).
    "**/Cargo.toml",
    "**/Cargo.lock",
    // The code that decides authority. A class cannot cover the code that
    // checks it.
    "crates/openwarrant-core/src/standing.rs",
    "crates/openwarrant-cli/src/standing_cmd.rs",
    "crates/openwarrant-cli/src/authorize.rs",
    "crates/openwarrant-cli/src/sign.rs",
    "crates/openwarrant-cli/src/batch_cmd.rs",
    "crates/openwarrant-cli/src/authority_check.rs",
    "crates/openwarrant-cli/src/attest.rs",
    "crates/openwarrant-cli/src/verify.rs",
    "crates/openwarrant-cli/src/resolve.rs",
    "crates/openwarrant-cli/src/resolution_cmd.rs",
    "crates/openwarrant-cli/src/ownership.rs",
    // Every Warrant's authorization records and every generated projection.
    // A resolved Warrant's `deliverables.toml` is refused where the corpus
    // is known (the CLI), since "resolved" is not a fact a constant holds.
    "docs/warrants/*/authorization*.toml",
    "**/generated/**",
];

/// A stage budget the class allows, per stage and per Warrant.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budget {
    /// Ceiling on each stage's `budget_tokens`; at most [`MAX_BUDGET_TOKENS`].
    pub budget_tokens: u64,
    /// Ceiling on each stage's `wall_time_seconds`; at most
    /// [`MAX_WALL_TIME_SECONDS`].
    pub wall_time_seconds: u64,
    /// Most stages one covered Warrant may declare.
    pub max_stages: u32,
    /// Most deliverables one covered Warrant may declare.
    pub max_deliverables: u32,
}

/// `oh.war/standing-authorization/v1`: the class a human signs.
///
/// Every field is required and no other field is accepted.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StandingAuthorization {
    pub schema: String,
    /// `[a-z0-9][a-z0-9-]*`, the name the class is referred to by.
    pub id: String,
    /// 1, 2, …: a wider or narrower class is a new revision, signed again.
    pub revision: u32,
    /// The owner's own words for what the signature grants.
    pub meaning: String,
    /// Globs a covered Warrant may declare as deliverables.
    pub paths: Vec<String>,
    /// `delivery`, the only profile a class may cover.
    pub profile: String,
    /// `basic`, the only assurance level a class may cover.
    pub assurance: String,
    /// Gates a covered obligation may cite; includes [`REQUIRED_GATES`].
    pub gates: Vec<String>,
    pub budget: Budget,
    /// RFC 3339 UTC; at most [`MAX_EXPIRY_SECONDS`] after the signature.
    pub expires_at: String,
    /// At most this many Warrants are covered; at most [`MAX_WARRANTS`].
    pub max_warrants: u32,
}

/// One stage of a contract, as far as a class bounds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageBudget {
    pub id: String,
    pub executor_kind: String,
    pub budget_tokens: Option<u64>,
    pub wall_time_seconds: Option<u64>,
}

/// What [`covers`] reads of one compiled Warrant. Every field is a fact the
/// compiled contract or its records already state; nothing is inferred.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contract {
    pub profile: String,
    pub assurance: String,
    /// Declared deliverable paths, repository-relative.
    pub declared_paths: Vec<String>,
    /// Whether the manifest composes an `adr` atom.
    pub has_adr_atom: bool,
    /// Ids of assumptions carrying `accepted_residual_risk`.
    pub residual_risks: Vec<String>,
    /// Every `gate://` an assurance atom cites.
    pub cited_gates: Vec<String>,
    pub stages: Vec<StageBudget>,
}

/// Why a class, or a Warrant under it, is refused. Each names its term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The record is not TOML, or a field has the wrong type.
    Malformed {
        why: String,
    },
    /// A term the class does not know — a resolution, ADR or risk term, say.
    UnknownTerm {
        term: String,
    },
    /// A term the class must state and does not.
    MissingTerm {
        term: String,
    },
    /// A class glob could match a never-coverable path, or a declared path is
    /// one.
    NeverCoverable {
        glob: String,
        path: String,
    },
    /// A class term outside what Q-002 or OW-ADR-0029 allows.
    Bound {
        term: &'static str,
        why: String,
    },
    /// A declared path no class glob matches.
    OutsideClass {
        path: String,
    },
    Profile {
        found: String,
    },
    /// The class names a profile whose data does not allow standing
    /// coverage (OW-ADR-0031: `standing_coverage`; of the core profiles,
    /// `delivery` only).
    ProfileNotCoverable {
        found: String,
    },
    Assurance {
        found: String,
    },
    AdrAtom,
    ResidualRisk {
        assumption: String,
    },
    /// An obligation cites a gate the class does not name.
    Gate {
        gate: String,
    },
    /// A stage over (or not stating) a budget the class bounds.
    Budget {
        stage: String,
        term: &'static str,
        found: Option<u64>,
        max: u64,
    },
    /// More stages or deliverables than the class allows in one Warrant.
    Size {
        term: &'static str,
        found: usize,
        max: u32,
    },
    Expired {
        at: String,
        expires_at: String,
    },
    /// Stamped before the class was signed.
    NotYetSigned {
        at: String,
        signed_at: String,
    },
    Exhausted {
        used: u32,
        max: u32,
    },
    Revoked {
        revoked_at: String,
    },
}

impl Refusal {
    /// The rule this refusal is reported under.
    #[must_use]
    pub const fn rule(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => "standing.malformed",
            Self::UnknownTerm { .. } => "standing.unknown-term",
            Self::MissingTerm { .. } => "standing.missing-term",
            Self::NeverCoverable { .. } => "standing.never-coverable",
            Self::Bound { .. } => "standing.bound",
            Self::OutsideClass { .. } => "standing.outside-class",
            Self::Profile { .. } | Self::ProfileNotCoverable { .. } => "standing.profile",
            Self::Assurance { .. } => "standing.assurance",
            Self::AdrAtom => "standing.adr",
            Self::ResidualRisk { .. } => "standing.residual-risk",
            Self::Gate { .. } => "standing.gate",
            Self::Budget { .. } => "standing.budget",
            Self::Size { .. } => "standing.size",
            Self::Expired { .. } => "standing.expired",
            Self::NotYetSigned { .. } => "standing.not-yet-signed",
            Self::Exhausted { .. } => "standing.exhausted",
            Self::Revoked { .. } => "standing.revoked",
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed { why } => write!(f, "the class record does not parse: {why}"),
            Self::UnknownTerm { term } => write!(
                f,
                "term `{term}` is not a term of {SCHEMA}; a class states only its closed terms, \
                 and a term it does not know is refused, never ignored"
            ),
            Self::MissingTerm { term } => write!(
                f,
                "term `{term}` is not stated; a term the class does not state is refused, not \
                 defaulted (§30.4)"
            ),
            Self::NeverCoverable { glob, path } => write!(
                f,
                "`{glob}` could cover {path}, which no class may cover (OW-ADR-0029: the \
                 authority files, the gates and their fixtures, the guards, dependency \
                 manifests, the code that decides authority, authorization records and \
                 generated projections)"
            ),
            Self::Bound { term, why } => write!(f, "term `{term}`: {why}"),
            Self::OutsideClass { path } => write!(
                f,
                "term `paths`: {path} is declared and no glob of the class matches it"
            ),
            Self::Profile { found } => write!(
                f,
                "term `profile`: the Warrant's profile is `{found}`; a class covers its own \
                 profile only"
            ),
            Self::ProfileNotCoverable { found } => write!(
                f,
                "term `profile`: `{found}` is not a profile a class may cover; its data does \
                 not set `standing_coverage` (of the core profiles, `delivery` only)"
            ),
            Self::Assurance { found } => write!(
                f,
                "term `assurance`: the Warrant is at `{found}`; a class covers `basic` only"
            ),
            Self::AdrAtom => f.write_str(
                "term `adr`: the Warrant composes an ADR atom, and a class has no ADR term — a \
                 normative decision is signed on its own",
            ),
            Self::ResidualRisk { assumption } => write!(
                f,
                "term `residual_risk`: assumption {assumption} is `accepted_residual_risk`, and a \
                 class has no term under which a risk is accepted (§27.2)"
            ),
            Self::Gate { gate } => write!(
                f,
                "term `gates`: an obligation cites {gate}, which the class does not name"
            ),
            Self::Budget {
                stage,
                term,
                found: Some(found),
                max,
            } => write!(
                f,
                "term `budget.{term}`: {stage} declares {found}, over the class's {max}"
            ),
            Self::Budget {
                stage,
                term,
                found: None,
                max,
            } => write!(
                f,
                "term `budget.{term}`: {stage} is an agent stage and declares no {term}, so its \
                 budget is unbounded and the class's {max} cannot be checked"
            ),
            Self::Size { term, found, max } => write!(
                f,
                "term `budget.{term}`: the Warrant declares {found}, over the class's {max}"
            ),
            Self::Expired { at, expires_at } => write!(
                f,
                "term `expires_at`: {at} is after the class expired at {expires_at}; it covers \
                 nothing now"
            ),
            Self::NotYetSigned { at, signed_at } => write!(
                f,
                "{at} is before the class was signed at {signed_at}; a class covers nothing \
                 before its signature"
            ),
            Self::Exhausted { used, max } => write!(
                f,
                "term `max_warrants`: {used} Warrant(s) are already covered and the class covers \
                 at most {max}"
            ),
            Self::Revoked { revoked_at } => write!(
                f,
                "the class was revoked at {revoked_at}; it covers nothing after that, and records \
                 made before it stand (§31)"
            ),
        }
    }
}

/// The terms of a class, in declaration order: what [`parse`] names when one
/// is missing, and what the unknown-term check compares against.
pub const TERMS: &[&str] = &[
    "schema",
    "id",
    "revision",
    "meaning",
    "paths",
    "profile",
    "assurance",
    "gates",
    "budget",
    "expires_at",
    "max_warrants",
];

const BUDGET_TERMS: &[&str] = &[
    "budget_tokens",
    "wall_time_seconds",
    "max_stages",
    "max_deliverables",
];

/// Parse a class record and apply every rule that needs no clock:
/// closed terms, the never-coverable set, the profile, assurance, gates and
/// budget bounds. The expiry's distance from the signature is
/// [`expiry_refusal`], because the signature's time is not in the record.
///
/// # Errors
/// Every refusal found, not only the first.
pub fn parse(text: &str) -> Result<StandingAuthorization, Vec<Refusal>> {
    parse_in(text, &|profile| profile == "delivery")
}

/// [`parse`], with `may_cover` answering whether a profile's data allows
/// standing coverage (OW-ADR-0031) — the program's registry, not a name.
///
/// # Errors
/// Every refusal found, not only the first.
pub fn parse_in(
    text: &str,
    may_cover: &dyn Fn(&str) -> bool,
) -> Result<StandingAuthorization, Vec<Refusal>> {
    // Terms first, from the raw table: serde would name one unknown field and
    // stop, and a missing term reads as a type error there.
    let raw: toml::Table =
        toml::from_str(text).map_err(|e| vec![Refusal::Malformed { why: e.to_string() }])?;
    let mut refusals = Vec::new();
    for key in raw.keys() {
        if !TERMS.contains(&key.as_str()) {
            refusals.push(Refusal::UnknownTerm { term: key.clone() });
        }
    }
    for term in TERMS {
        if !raw.contains_key(*term) {
            refusals.push(Refusal::MissingTerm {
                term: (*term).to_owned(),
            });
        }
    }
    if let Some(toml::Value::Table(b)) = raw.get("budget") {
        for key in b.keys() {
            if !BUDGET_TERMS.contains(&key.as_str()) {
                refusals.push(Refusal::UnknownTerm {
                    term: format!("budget.{key}"),
                });
            }
        }
        for term in BUDGET_TERMS {
            if !b.contains_key(*term) {
                refusals.push(Refusal::MissingTerm {
                    term: format!("budget.{term}"),
                });
            }
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let class: StandingAuthorization =
        toml::from_str(text).map_err(|e| vec![Refusal::Malformed { why: e.to_string() }])?;
    let refusals = validate_in(&class, may_cover);
    if refusals.is_empty() {
        Ok(class)
    } else {
        Err(refusals)
    }
}

/// Every clock-free rule over a parsed class, `delivery` the only profile a
/// class may cover (the builtin registry's answer).
#[must_use]
pub fn validate(class: &StandingAuthorization) -> Vec<Refusal> {
    validate_in(class, &|profile| profile == "delivery")
}

/// [`validate`], with `may_cover` answering from the profile's data.
#[must_use]
pub fn validate_in(
    class: &StandingAuthorization,
    may_cover: &dyn Fn(&str) -> bool,
) -> Vec<Refusal> {
    let mut out = Vec::new();
    let bound = |term: &'static str, why: String| Refusal::Bound { term, why };
    if class.schema != SCHEMA {
        out.push(bound(
            "schema",
            format!("{:?} is not {SCHEMA:?}", class.schema),
        ));
    }
    if !valid_id(&class.id) {
        out.push(bound(
            "id",
            format!(
                "{:?} is not `[a-z0-9][a-z0-9-]*` (at most 64 characters)",
                class.id
            ),
        ));
    }
    if class.revision == 0 {
        out.push(bound("revision", "revisions start at 1".to_owned()));
    }
    if class.meaning.trim().is_empty() {
        out.push(bound(
            "meaning",
            "empty; the signer's words for what the signature grants are required (§42)".to_owned(),
        ));
    }
    if class.paths.is_empty() {
        out.push(bound(
            "paths",
            "no glob; the class covers nothing".to_owned(),
        ));
    }
    for glob in &class.paths {
        if let Err(why) = valid_glob(glob) {
            out.push(bound("paths", format!("`{glob}`: {why}")));
        }
    }
    out.extend(class_refusals(&class.paths));
    if !may_cover(&class.profile) {
        out.push(Refusal::ProfileNotCoverable {
            found: class.profile.clone(),
        });
    }
    if class.assurance != "basic" {
        out.push(Refusal::Assurance {
            found: class.assurance.clone(),
        });
    }
    for g in REQUIRED_GATES {
        if !class.gates.iter().any(|c| c == g) {
            out.push(bound(
                "gates",
                format!("{g} is required in every class (OW-ADR-0029)"),
            ));
        }
    }
    for g in &class.gates {
        if !g.starts_with("gate://") || !g.contains('@') {
            out.push(bound(
                "gates",
                format!("{g:?} is not a versioned `gate://<id>@<version>`"),
            ));
        }
    }
    let b = &class.budget;
    if b.budget_tokens == 0 || b.budget_tokens > MAX_BUDGET_TOKENS {
        out.push(bound(
            "budget.budget_tokens",
            format!(
                "{} is outside 1..={MAX_BUDGET_TOKENS} (Q-002)",
                b.budget_tokens
            ),
        ));
    }
    if b.wall_time_seconds == 0 || b.wall_time_seconds > MAX_WALL_TIME_SECONDS {
        out.push(bound(
            "budget.wall_time_seconds",
            format!(
                "{} is outside 1..={MAX_WALL_TIME_SECONDS} (Q-002)",
                b.wall_time_seconds
            ),
        ));
    }
    if b.max_stages == 0 {
        out.push(bound("budget.max_stages", "0 covers nothing".to_owned()));
    }
    if b.max_deliverables == 0 {
        out.push(bound(
            "budget.max_deliverables",
            "0 covers nothing".to_owned(),
        ));
    }
    if class.max_warrants == 0 || class.max_warrants > MAX_WARRANTS {
        out.push(bound(
            "max_warrants",
            format!(
                "{} is outside 1..={MAX_WARRANTS} (Q-002)",
                class.max_warrants
            ),
        ));
    }
    if epoch_seconds(&class.expires_at).is_none() {
        out.push(bound(
            "expires_at",
            format!(
                "{:?} is not an RFC 3339 UTC time (`YYYY-MM-DDTHH:MM:SSZ`)",
                class.expires_at
            ),
        ));
    }
    out
}

/// The expiry against the moment the class is signed (or proposed): after
/// it, and at most [`MAX_EXPIRY_SECONDS`] later.
#[must_use]
pub fn expiry_refusal(class: &StandingAuthorization, signed_at: &str) -> Option<Refusal> {
    let (Some(exp), Some(at)) = (epoch_seconds(&class.expires_at), epoch_seconds(signed_at)) else {
        return Some(Refusal::Bound {
            term: "expires_at",
            why: format!(
                "{:?} or {signed_at:?} is not an RFC 3339 UTC time",
                class.expires_at
            ),
        });
    };
    if exp <= at {
        return Some(Refusal::Bound {
            term: "expires_at",
            why: format!(
                "{} is not after {signed_at}; a class that has already expired covers nothing",
                class.expires_at
            ),
        });
    }
    if exp - at > MAX_EXPIRY_SECONDS {
        return Some(Refusal::Bound {
            term: "expires_at",
            why: format!(
                "{} is more than 90 days after {signed_at} (Q-002)",
                class.expires_at
            ),
        });
    }
    None
}

/// The class-glob half of the never-coverable rule: one refusal per glob
/// that could reach a never-coverable path, naming that path.
#[must_use]
pub fn class_refusals(globs: &[String]) -> Vec<Refusal> {
    let mut out = Vec::new();
    for glob in globs {
        for nc in NEVER_COVERABLE {
            let hit = if let Some(tail) = nc.strip_prefix("**/") {
                // Any depth: refused when the glob's own last segment could
                // name the file, or the glob names the directory literally.
                any_depth_overlap(glob, tail)
            } else {
                globs_overlap(glob, nc)
            };
            if hit {
                out.push(Refusal::NeverCoverable {
                    glob: glob.clone(),
                    path: (*nc).to_owned(),
                });
            }
        }
    }
    out
}

/// Whether a class glob reaches an any-depth never-coverable entry whose
/// part after `**/` is `tail` (`Cargo.toml`, `generated/**`).
fn any_depth_overlap(glob: &str, tail: &str) -> bool {
    let segs: Vec<&str> = glob.split('/').collect();
    let tail_segs: Vec<&str> = tail.split('/').collect();
    match tail_segs.as_slice() {
        // `**/<file>`: the glob's last segment, when it is not `**`.
        [file] => segs
            .last()
            .is_some_and(|last| *last != "**" && segment_overlap(last, file)),
        // `**/<dir>/**`: any literal segment equal to the directory.
        [dir, "**"] => segs.iter().any(|s| s == dir),
        _ => globs_overlap(glob, &format!("**/{tail}")),
    }
}

/// The path half: whether one declared path is never coverable.
#[must_use]
pub fn never_coverable(path: &str) -> Option<&'static str> {
    NEVER_COVERABLE
        .iter()
        .copied()
        .find(|nc| glob_match(nc, path))
}

/// Whether the class admits one contract, term by term.
///
/// # Errors
/// One refusal per term broken, in the order the terms are declared.
pub fn covers(class: &StandingAuthorization, contract: &Contract) -> Result<(), Vec<Refusal>> {
    let mut out = Vec::new();
    for path in &contract.declared_paths {
        if let Some(nc) = never_coverable(path) {
            out.push(Refusal::NeverCoverable {
                glob: path.clone(),
                path: nc.to_owned(),
            });
        } else if !class.paths.iter().any(|g| glob_match(g, path)) {
            out.push(Refusal::OutsideClass { path: path.clone() });
        }
    }
    if contract.profile != class.profile {
        out.push(Refusal::Profile {
            found: contract.profile.clone(),
        });
    }
    if contract.assurance != class.assurance {
        out.push(Refusal::Assurance {
            found: contract.assurance.clone(),
        });
    }
    if contract.has_adr_atom {
        out.push(Refusal::AdrAtom);
    }
    for id in &contract.residual_risks {
        out.push(Refusal::ResidualRisk {
            assumption: id.clone(),
        });
    }
    for g in &contract.cited_gates {
        if !class.gates.iter().any(|c| c == g) {
            out.push(Refusal::Gate { gate: g.clone() });
        }
    }
    let b = &class.budget;
    if contract.stages.len() > b.max_stages as usize {
        out.push(Refusal::Size {
            term: "max_stages",
            found: contract.stages.len(),
            max: b.max_stages,
        });
    }
    if contract.declared_paths.len() > b.max_deliverables as usize {
        out.push(Refusal::Size {
            term: "max_deliverables",
            found: contract.declared_paths.len(),
            max: b.max_deliverables,
        });
    }
    for s in &contract.stages {
        match s.budget_tokens {
            Some(t) if t > b.budget_tokens => out.push(Refusal::Budget {
                stage: s.id.clone(),
                term: "budget_tokens",
                found: Some(t),
                max: b.budget_tokens,
            }),
            None if s.executor_kind == "agent" => out.push(Refusal::Budget {
                stage: s.id.clone(),
                term: "budget_tokens",
                found: None,
                max: b.budget_tokens,
            }),
            _ => {}
        }
        if let Some(w) = s.wall_time_seconds
            && w > b.wall_time_seconds
        {
            out.push(Refusal::Budget {
                stage: s.id.clone(),
                term: "wall_time_seconds",
                found: Some(w),
                max: b.wall_time_seconds,
            });
        }
    }
    if out.is_empty() { Ok(()) } else { Err(out) }
}

/// Whether the class is in force at `at` for one more Warrant: signed by
/// then, not expired, not revoked, and under its count. `used` is the number
/// of Warrants the class already covers, counted before `at`.
///
/// # Errors
/// The first bound broken, in that order.
pub fn in_force(
    class: &StandingAuthorization,
    at: &str,
    signed_at: &str,
    revoked_at: Option<&str>,
    used: u32,
) -> Result<(), Refusal> {
    let t = epoch_seconds(at);
    if let Some(r) = revoked_at
        && epoch_seconds(r)
            .zip(t)
            .is_none_or(|(revoked, now)| revoked <= now)
    {
        return Err(Refusal::Revoked {
            revoked_at: r.to_owned(),
        });
    }
    if epoch_seconds(signed_at)
        .zip(t)
        .is_none_or(|(signed, now)| now < signed)
    {
        return Err(Refusal::NotYetSigned {
            at: at.to_owned(),
            signed_at: signed_at.to_owned(),
        });
    }
    if epoch_seconds(&class.expires_at)
        .zip(t)
        .is_none_or(|(exp, now)| now > exp)
    {
        return Err(Refusal::Expired {
            at: at.to_owned(),
            expires_at: class.expires_at.clone(),
        });
    }
    if used >= class.max_warrants {
        return Err(Refusal::Exhausted {
            used,
            max: class.max_warrants,
        });
    }
    Ok(())
}

/// `standing://<id>@<revision>`.
#[must_use]
pub fn reference(id: &str, revision: u32) -> String {
    format!("{SCHEME}{id}@{revision}")
}

/// The inverse of [`reference`]: `(id, revision)`.
#[must_use]
pub fn parse_reference(r: &str) -> Option<(String, u32)> {
    let rest = r.strip_prefix(SCHEME)?;
    let (id, rev) = rest.split_once('@')?;
    let rev: u32 = rev.parse().ok()?;
    (valid_id(id) && rev > 0).then(|| (id.to_owned(), rev))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !id.starts_with('-')
}

fn valid_glob(glob: &str) -> Result<(), &'static str> {
    if glob.is_empty() {
        return Err("empty");
    }
    if glob.starts_with('/') {
        return Err("absolute; globs are repository-relative");
    }
    if glob
        .split('/')
        .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err("an empty, `.` or `..` segment");
    }
    if glob.contains(['[', ']', '{', '}', '\\']) {
        return Err("only `*`, `**` and `?` are glob syntax here");
    }
    if glob.split('/').any(|s| s.contains("**") && s != "**") {
        return Err("`**` must be a whole segment");
    }
    Ok(())
}

/// Whether `path` matches `glob`. `*` and `?` stay within a segment; `**`
/// is any number of whole segments, zero included.
#[must_use]
pub fn glob_match(glob: &str, path: &str) -> bool {
    let g: Vec<&str> = glob.split('/').collect();
    let p: Vec<&str> = path.split('/').collect();
    match_segments(&g, &p)
}

fn match_segments(g: &[&str], p: &[&str]) -> bool {
    match g.split_first() {
        None => p.is_empty(),
        Some((&"**", rest)) => (0..=p.len()).any(|i| match_segments(rest, &p[i..])),
        Some((seg, rest)) => p
            .split_first()
            .is_some_and(|(head, tail)| segment_match(seg, head) && match_segments(rest, tail)),
    }
}

fn segment_match(glob: &str, s: &str) -> bool {
    let g: Vec<char> = glob.chars().collect();
    let c: Vec<char> = s.chars().collect();
    fn go(g: &[char], c: &[char]) -> bool {
        match g.split_first() {
            None => c.is_empty(),
            Some(('*', rest)) => (0..=c.len()).any(|i| go(rest, &c[i..])),
            Some(('?', rest)) => !c.is_empty() && go(rest, &c[1..]),
            Some((ch, rest)) => c.first() == Some(ch) && go(rest, &c[1..]),
        }
    }
    go(&g, &c)
}

/// Whether some path matches both globs.
#[must_use]
pub fn globs_overlap(a: &str, b: &str) -> bool {
    let a: Vec<&str> = a.split('/').collect();
    let b: Vec<&str> = b.split('/').collect();
    overlap_segments(&a, &b)
}

fn overlap_segments(a: &[&str], b: &[&str]) -> bool {
    match (a.split_first(), b.split_first()) {
        (None, None) => true,
        (None, Some(_)) => b.iter().all(|s| *s == "**"),
        (Some(_), None) => a.iter().all(|s| *s == "**"),
        (Some((&"**", ar)), _) => {
            overlap_segments(ar, b) || (!b.is_empty() && overlap_segments(a, &b[1..]))
        }
        (_, Some((&"**", br))) => {
            overlap_segments(a, br) || (!a.is_empty() && overlap_segments(&a[1..], b))
        }
        (Some((x, ar)), Some((y, br))) => segment_overlap(x, y) && overlap_segments(ar, br),
    }
}

/// Whether some segment matches both segment globs.
fn segment_overlap(a: &str, b: &str) -> bool {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    fn go(a: &[char], b: &[char]) -> bool {
        match (a.split_first(), b.split_first()) {
            (None, None) => true,
            (None, Some(_)) => b.iter().all(|c| *c == '*'),
            (Some(_), None) => a.iter().all(|c| *c == '*'),
            (Some(('*', ar)), _) => go(ar, b) || (!b.is_empty() && go(a, &b[1..])),
            (_, Some(('*', br))) => go(a, br) || (!a.is_empty() && go(&a[1..], b)),
            (Some((x, ar)), Some((y, br))) => (*x == '?' || *y == '?' || x == y) && go(ar, br),
        }
    }
    go(&a, &b)
}

/// Seconds since the Unix epoch of an RFC 3339 UTC time
/// (`YYYY-MM-DDTHH:MM:SS[.fff]Z`), or `None` when it is not one.
#[must_use]
pub fn epoch_seconds(s: &str) -> Option<i64> {
    crate::timestamp::validate_rfc3339_utc(s).ok()?;
    // An offset would have to be applied; a class compares instants, so only
    // `Z` is read and anything else is not a time here.
    if !s.ends_with(['Z', 'z']) {
        return None;
    }
    let b = s.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        std::str::from_utf8(b.get(from..from + len)?)
            .ok()?
            .parse()
            .ok()
    };
    let (y, m, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hh, mm, ss) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    // Days from civil (Howard Hinnant's algorithm).
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(days * 86_400 + hh * 3600 + mm * 60 + ss)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_text(paths: &str) -> String {
        format!(
            r#"schema = "oh.war/standing-authorization/v1"
id = "routine-tui"
revision = 1
meaning = "Routine edits to the TUI."
paths = [{paths}]
profile = "delivery"
assurance = "basic"
gates = ["gate://ops.conformance.plants@1.1.0", "gate://software.repo.war-check@1.0.0"]
expires_at = "2026-12-01T00:00:00Z"
max_warrants = 10

[budget]
budget_tokens = 24000
wall_time_seconds = 1800
max_stages = 4
max_deliverables = 6
"#
        )
    }

    fn rules(r: &[Refusal]) -> Vec<&'static str> {
        r.iter().map(Refusal::rule).collect()
    }

    #[test]
    fn a_tui_class_parses_and_a_class_naming_authority_is_refused() {
        assert!(parse(&class_text(r#""crates/openwarrant-cli/src/tui/**""#)).is_ok());
        for bad in [
            "docs/authority/roles.toml",
            "docs/authority/allowed_signers",
            "openwarrant.toml",
            "crates/openwarrant-cli/src/sign.rs",
            "**",
            "docs/**",
            "crates/*/src/*.rs",
            "crates/**/Cargo.toml",
            "docs/warrants/*/generated/**",
        ] {
            let got = parse(&class_text(&format!("{bad:?}"))).expect_err(bad);
            assert!(
                got.iter()
                    .any(|r| matches!(r, Refusal::NeverCoverable { glob, .. } if glob == bad)),
                "{bad}: {got:?}"
            );
        }
    }

    #[test]
    fn an_unknown_or_missing_term_is_refused_by_name() {
        let with_resolution = class_text(r#""src/**""#).replace(
            "max_warrants = 10",
            "max_warrants = 10\nresolution = \"policy_service\"",
        );
        let got = parse(&with_resolution).unwrap_err();
        assert_eq!(rules(&got), ["standing.unknown-term"], "{got:?}");
        let without_expiry =
            class_text(r#""src/**""#).replace("expires_at = \"2026-12-01T00:00:00Z\"\n", "");
        let got = parse(&without_expiry).unwrap_err();
        assert_eq!(rules(&got), ["standing.missing-term"], "{got:?}");
    }

    #[test]
    fn the_q002_bounds_hold() {
        let over = class_text(r#""src/**""#)
            .replace("max_warrants = 10", "max_warrants = 51")
            .replace("budget_tokens = 24000", "budget_tokens = 24001")
            .replace("wall_time_seconds = 1800", "wall_time_seconds = 1801");
        let got = parse(&over).unwrap_err();
        assert_eq!(got.len(), 3, "{got:?}");
        let c = parse(&class_text(r#""src/**""#)).unwrap();
        assert!(expiry_refusal(&c, "2026-09-25T00:00:00Z").is_none());
        assert!(expiry_refusal(&c, "2026-08-01T00:00:00Z").is_some());
        assert!(expiry_refusal(&c, "2026-12-02T00:00:00Z").is_some());
    }

    #[test]
    fn globs_match_and_overlap_as_documented() {
        assert!(glob_match("src/**", "src/a/b.rs"));
        assert!(glob_match("src/**", "src"));
        assert!(glob_match("src/*.rs", "src/a.rs"));
        assert!(!glob_match("src/*.rs", "src/a/b.rs"));
        assert!(glob_match("**/Cargo.toml", "Cargo.toml"));
        assert!(glob_match("**/Cargo.toml", "crates/x/Cargo.toml"));
        assert!(globs_overlap("**", "docs/authority/**"));
        assert!(globs_overlap("docs/*/roles.toml", "docs/authority/**"));
        assert!(!globs_overlap(
            "crates/openwarrant-cli/src/tui/**",
            "docs/**"
        ));
        assert!(!globs_overlap(
            "crates/openwarrant-cli/src/tui/**",
            "crates/openwarrant-cli/src/sign.rs"
        ));
        assert!(globs_overlap(
            "crates/*/src/s*.rs",
            "crates/openwarrant-cli/src/sign.rs"
        ));
    }

    fn contract() -> Contract {
        Contract {
            profile: "delivery".into(),
            assurance: "basic".into(),
            declared_paths: vec!["src/app.rs".into()],
            cited_gates: vec!["gate://software.repo.war-check@1.0.0".into()],
            stages: vec![StageBudget {
                id: "STAGE-001".into(),
                executor_kind: "agent".into(),
                budget_tokens: Some(1000),
                wall_time_seconds: None,
            }],
            ..Contract::default()
        }
    }

    #[test]
    fn a_contract_inside_is_covered_and_each_term_refuses_on_its_own() {
        let c = parse(&class_text(r#""src/**""#)).unwrap();
        assert_eq!(covers(&c, &contract()), Ok(()));
        type Mutation = Box<dyn Fn(&mut Contract)>;
        let cases: Vec<(Mutation, &str)> = vec![
            (
                Box::new(|k| k.declared_paths.push("lib/x.rs".into())),
                "standing.outside-class",
            ),
            (
                Box::new(|k| k.assurance = "controlled".into()),
                "standing.assurance",
            ),
            (
                Box::new(|k| k.profile = "decision".into()),
                "standing.profile",
            ),
            (Box::new(|k| k.has_adr_atom = true), "standing.adr"),
            (
                Box::new(|k| k.residual_risks.push("A-001".into())),
                "standing.residual-risk",
            ),
            (
                Box::new(|k| k.cited_gates.push("gate://other@1.0.0".into())),
                "standing.gate",
            ),
            (
                Box::new(|k| k.stages[0].budget_tokens = Some(24_001)),
                "standing.budget",
            ),
            (
                Box::new(|k| k.stages[0].wall_time_seconds = Some(1801)),
                "standing.budget",
            ),
            (
                Box::new(|k| k.stages[0].budget_tokens = None),
                "standing.budget",
            ),
            (
                Box::new(|k| k.declared_paths.push("Cargo.toml".into())),
                "standing.never-coverable",
            ),
        ];
        for (mutate, rule) in cases {
            let mut k = contract();
            mutate(&mut k);
            let got = covers(&c, &k).unwrap_err();
            assert_eq!(rules(&got), [rule], "{got:?}");
        }
    }

    #[test]
    fn in_force_refuses_after_expiry_revocation_and_the_count() {
        let c = parse(&class_text(r#""src/**""#)).unwrap();
        let signed = "2026-09-25T00:00:00Z";
        assert!(in_force(&c, "2026-10-01T00:00:00Z", signed, None, 0).is_ok());
        assert_eq!(
            in_force(&c, "2026-12-02T00:00:00Z", signed, None, 0)
                .unwrap_err()
                .rule(),
            "standing.expired"
        );
        assert_eq!(
            in_force(&c, "2026-10-01T00:00:00Z", signed, None, 10)
                .unwrap_err()
                .rule(),
            "standing.exhausted"
        );
        assert_eq!(
            in_force(
                &c,
                "2026-10-01T00:00:00Z",
                signed,
                Some("2026-09-30T00:00:00Z"),
                0
            )
            .unwrap_err()
            .rule(),
            "standing.revoked"
        );
        assert!(
            in_force(
                &c,
                "2026-09-29T00:00:00Z",
                signed,
                Some("2026-09-30T00:00:00Z"),
                0
            )
            .is_ok()
        );
        assert_eq!(
            in_force(&c, "2026-09-24T00:00:00Z", signed, None, 0)
                .unwrap_err()
                .rule(),
            "standing.not-yet-signed"
        );
    }

    #[test]
    fn epoch_seconds_agrees_with_known_instants() {
        assert_eq!(epoch_seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(epoch_seconds("2026-09-25T00:00:00Z"), Some(1_790_294_400));
        assert_eq!(epoch_seconds("not a time"), None);
    }

    #[test]
    fn a_reference_round_trips() {
        assert_eq!(reference("routine-tui", 2), "standing://routine-tui@2");
        assert_eq!(
            parse_reference("standing://routine-tui@2"),
            Some(("routine-tui".to_owned(), 2))
        );
        assert_eq!(parse_reference("standing://Bad@1"), None);
        assert_eq!(parse_reference("standing://x@0"), None);
    }
}
