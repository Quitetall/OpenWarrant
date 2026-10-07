// SPDX-License-Identifier: Apache-2.0
//! Repository configuration — `openwarrant.toml` (SAS §60).
//!
//! This crate defines and validates the shape; the CLI reads the bytes (§79.1).

use serde::{Deserialize, Serialize};

/// The only repository-config schema this build understands.
pub const REPOSITORY_CONFIG_SCHEMA: &str = "oh.war/repository-config/v1";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error(
        "unknown repository-config schema {found:?}; this build understands \
         {expected:?} (SAS §69.3 — a breaking protocol change is not silently accepted)"
    )]
    UnknownSchema { found: String, expected: String },
    #[error("project.namespace is empty")]
    NamespaceEmpty,
    #[error(
        "project.namespace {found:?} is malformed; expected uppercase ASCII \
         letters, digits, or hyphens (it prefixes every local alias)"
    )]
    NamespaceMalformed { found: String },
    #[error("project.name is empty")]
    ProjectNameEmpty,
    #[error("paths.{field} is empty")]
    PathEmpty { field: &'static str },
    #[error("paths.{field} is {value:?}; configured paths must be relative to the repository root")]
    PathNotRelative { field: &'static str, value: String },
    #[error(
        "sign.preset {key:?} offers itself for a correction but names no `kind`; \
         a correction records whether it is a behaviour-change or an added-refusal, \
         and no tool may decide that for the signer"
    )]
    PresetKindMissing { key: String },
    #[error(
        "project.requires_war {found:?} is not a version requirement: {why}. Write one or \
         more comma-separated comparators such as \">=1.0.0\", \">=1.2, <2\" or \"^1.1\" \
         (OW-WAR-0130)"
    )]
    RequiresWarMalformed { found: String, why: String },
    #[error(
        "authority.store is {found:?}; a protected store is named by an absolute path with no \
         `..` (OW-WAR-0138)"
    )]
    AuthorityStoreNotAbsolute { found: String },
}

/// A validated project namespace, e.g. `OW`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Namespace(String);

impl Namespace {
    /// Parse a namespace.
    ///
    /// Surrounding whitespace is **trimmed, not rejected**: `" OW "` is accepted
    /// and stored as `"OW"`. This matches [`crate::LocalAlias::parse`], and the
    /// two must agree — a namespace that normalised differently from the alias
    /// prefix it is compared against would make
    /// [`crate::LocalAlias::parse_in`] reject correct input.
    pub fn parse(raw: &str) -> Result<Self, ConfigError> {
        let value = raw.trim();
        if value.is_empty() {
            return Err(ConfigError::NamespaceEmpty);
        }
        let well_formed = value
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
            && !value.starts_with('-')
            && !value.ends_with('-');
        if !well_formed {
            return Err(ConfigError::NamespaceMalformed {
                found: value.to_owned(),
            });
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub namespace: Namespace,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knowledge_fabric_project_ref: Option<String>,
    /// The actor every self-* check compares against (self-verification
    /// §46, self-authorization §27.2, self-resolution §27.3). Set in this
    /// human-written, committed file and nowhere else: a flag that renamed
    /// the performer would let a caller walk out of all three. Default
    /// `claude`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub performer: Option<String>,
    /// Where the repository lives, for the projections that link back to
    /// it. Optional; a tracked input, unlike a git remote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_url: Option<String>,
    /// The `war` versions this repository needs (OW-WAR-0130, option B of
    /// its U-001): a requirement such as `">=1.2.0"`, checked once when a
    /// repository is discovered, before any record is read. A `war` it does
    /// not admit refuses with `compat.war-too-old` rather than misreading
    /// records a newer one wrote. Absent: any `war` reads the repository.
    ///
    /// A `war` older than this key does not know it and ignores it; the key
    /// protects from the first release that reads it onwards
    /// (docs/COMPATIBILITY.md, "Reading backward").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_war: Option<String>,
}

/// Where the controlled document trees live (§59, §60).
///
/// Paths are configurable and semantics are NOT inferred from them (§59):
/// a directory is the warrant tree because this field says so, not because it
/// happens to be named `warrants`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paths {
    #[serde(default = "Paths::default_sas")]
    pub sas: String,
    #[serde(default = "Paths::default_roadmap")]
    pub roadmap: String,
    #[serde(default = "Paths::default_adrs")]
    pub adrs: String,
    #[serde(default = "Paths::default_warrants")]
    pub warrants: String,
    /// §43.1 — local gate candidates and cached registry projections. The
    /// authoritative institutional registry is Knowledge Fabric's; this
    /// directory never holds it (OW-ADR-0005).
    #[serde(default = "Paths::default_gates")]
    pub gates: String,
    /// §44.6 gate-run receipts. Evidence, so it is preserved rather than
    /// written to a temporary directory and lost.
    #[serde(default = "Paths::default_receipts")]
    pub receipts: String,
}

impl Paths {
    fn default_sas() -> String {
        "docs/sas".to_owned()
    }
    fn default_roadmap() -> String {
        "docs/roadmap".to_owned()
    }
    fn default_adrs() -> String {
        "docs/adr".to_owned()
    }
    fn default_warrants() -> String {
        "docs/warrants".to_owned()
    }
    fn default_gates() -> String {
        "docs/gates".to_owned()
    }
    fn default_receipts() -> String {
        "docs/receipts".to_owned()
    }

    fn validate(&self) -> Result<(), ConfigError> {
        for (field, value) in [
            ("sas", &self.sas),
            ("roadmap", &self.roadmap),
            ("adrs", &self.adrs),
            ("warrants", &self.warrants),
            ("gates", &self.gates),
            ("receipts", &self.receipts),
        ] {
            if value.trim().is_empty() {
                return Err(ConfigError::PathEmpty { field });
            }
            // An absolute path in repository configuration makes the repository
            // unclonable: it would resolve to one machine's layout.
            //
            // The `:` test catches a Windows drive prefix (`C:\...`). It is
            // deliberately coarse and will also reject a *relative* POSIX path
            // containing a colon, such as `docs:v2` — legal on Unix, and refused
            // here anyway. That trade is taken knowingly: a colon in a
            // configured document path is far more likely to be a drive letter
            // or a URI fragment than an intended directory name, and being
            // wrong in this direction costs a rename, while being wrong in the
            // other direction produces a repository that only builds on one
            // machine.
            if value.starts_with('/') || value.contains(':') {
                return Err(ConfigError::PathNotRelative {
                    field,
                    value: value.clone(),
                });
            }
        }
        Ok(())
    }
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            sas: Self::default_sas(),
            roadmap: Self::default_roadmap(),
            adrs: Self::default_adrs(),
            warrants: Self::default_warrants(),
            gates: Self::default_gates(),
            receipts: Self::default_receipts(),
        }
    }
}

/// Generated-view policy (§59.2).
///
/// `commit = true, verify_drift = true` is what makes §17.3 enforceable: the
/// compiled parents are in Git, so a fresh compilation can be compared against
/// them. Setting `commit = false` is permitted by the SAS and removes that
/// comparison — the authority of the sources is unchanged either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedPolicy {
    #[serde(default = "GeneratedPolicy::yes")]
    pub commit: bool,
    #[serde(default = "GeneratedPolicy::yes")]
    pub verify_drift: bool,
    /// OW-ADR-0022 — write `docs/generated/HISTORY.md`, the optional
    /// projection of everything that ever existed, beside the master
    /// document. Off unless a repository asks: a new program has no history
    /// to keep, and `CURRENT.md` already names every replaced subject by one
    /// line of lineage.
    #[serde(default)]
    pub history: bool,
}

impl GeneratedPolicy {
    fn yes() -> bool {
        true
    }
}

impl Default for GeneratedPolicy {
    fn default() -> Self {
        Self {
            commit: true,
            verify_drift: true,
            history: false,
        }
    }
}

/// §27.3 condition 1 — whether a policy service may resolve automatically.
///
/// A separate block from everything else in the config because of what it is:
/// the one switch that lets a machine close work. It defaults to `false` and
/// there is no code path that writes it, so turning it on is an act a human
/// performs in an editor. An agent that could set it would be granting itself
/// the authority §27.2 withholds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityPolicy {
    /// §27.3 — `false` unless a human wrote otherwise. The other four
    /// conditions are properties of the work and are checked per Warrant; this
    /// one is the standing permission without which none of them are reached.
    #[serde(default)]
    pub allow_automated_resolution: bool,
    /// OW-WAR-0138 — every signature `war sign` makes (one act or a batch)
    /// must show a person at the key: a security key's signature with its
    /// user-presence flag set. An ordinary key, a security key signing
    /// without a touch, and the terminal path (no signature at all) are
    /// refused `sign.presence-required` before anything is renamed into
    /// place. `false` unless a human wrote otherwise, and omitted from a
    /// written config while false, so every existing file keeps its bytes.
    ///
    /// Without a protected store it lives here, in a file the performer can
    /// edit: turning it off is a commit a reviewer sees, not a thing `war` can
    /// stop. With `[authority] store` set, the store's value governs
    /// ([`RepositoryConfig::govern_from_store`]).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub require_user_presence: bool,
}

/// §75.2's configured drafter: the process `war plan --draft` hands the
/// request to. Absent means `war plan` emits the request and stops, which is
/// the honest default — a seam with nothing on the other side says so.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanPolicy {
    /// argv, not a shell string: an executable and its arguments, no shell
    /// between them. Empty means no drafter is configured.
    #[serde(default)]
    pub drafter_argv: Vec<String>,
    /// Wall-clock bound on one drafting run. 0 means the default (300).
    #[serde(default)]
    pub drafter_timeout_secs: u64,
    /// Recorded in the journal as the proposing actor, e.g. `claude-code 2.1`.
    #[serde(default)]
    pub drafter_name: String,
}

impl PlanPolicy {
    #[must_use]
    pub fn timeout_secs(&self) -> u64 {
        if self.drafter_timeout_secs == 0 {
            300
        } else {
            self.drafter_timeout_secs
        }
    }
}

/// One named reason a signer reaches for often (`[[sign.preset]]`).
///
/// The friction a signing queue creates is not the key, it is the prose: ten
/// acts with one sentence each is twenty minutes of typing about changes the
/// repository already describes. A preset is that sentence, written once, by
/// the human, in the configuration they own. Picking one is a keystroke; the
/// tool still records who signed, when, and over which bytes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignPreset {
    /// The key pressed to choose it, usually one character.
    pub key: String,
    /// What it says on the menu.
    pub label: String,
    /// The reason recorded, verbatim, appended to the drafted meaning.
    pub meaning: String,
    /// Which acts it applies to: `authorize`, `resolve`, `accept`, `correct`.
    /// Empty means every act.
    #[serde(default)]
    pub acts: Vec<String>,
    /// For a correction: `behaviour-change` or `added-refusal`. Ignored by
    /// the other acts, required by that one, so a preset can carry it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignPolicy {
    /// The presets offered in `war console` and by `war sign --preset`.
    #[serde(default, rename = "preset", skip_serializing_if = "Vec::is_empty")]
    pub presets: Vec<SignPreset>,
}

impl SignPolicy {
    /// No presets declared: the table is omitted from a written config.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty()
    }

    /// A preset that offers itself for a correction must say which kind of
    /// correction, because `war sign --kind` has no default a tool may pick:
    /// "the behaviour changed" and "a refusal was added" are different claims
    /// about the same bytes, and only the signer knows which one is true.
    pub fn validate(&self) -> Result<(), ConfigError> {
        for p in &self.presets {
            if p.acts.iter().any(|a| a == "correct") && p.kind.trim().is_empty() {
                return Err(ConfigError::PresetKindMissing { key: p.key.clone() });
            }
        }
        Ok(())
    }

    /// The presets that apply to one act kind, in declared order.
    #[must_use]
    pub fn for_act(&self, act: &str) -> Vec<&SignPreset> {
        self.presets
            .iter()
            .filter(|p| p.acts.is_empty() || p.acts.iter().any(|a| a == act))
            .collect()
    }
}

/// `[perform]` — the agent that performs an agent stage (OW-WAR-0069).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PerformPolicy {
    /// argv, not a shell string: the agent that performs an agent stage, given
    /// the Dispatch on stdin and expected to answer with a Stage Submission on
    /// stdout. Empty means no performer is configured.
    #[serde(default)]
    pub performer_argv: Vec<String>,
    /// Wall-clock bound on one performance. 0 means the stage's own
    /// `wall_time_seconds`, and failing that `[run] default_wall_time_seconds`.
    #[serde(default)]
    pub performer_timeout_secs: u64,
    /// How many agent stages may run at once. 0 means 1.
    ///
    /// One, in 1.0. Nothing here contains a performer — no cgroups, no sandbox
    /// — so concurrency would mean several unbounded processes writing one
    /// tree. Raising it is a deliberate act, and `war perform` says so.
    #[serde(default)]
    pub max_concurrent: u32,
    /// Whether `war perform` may run a performer that reports no spend
    /// (OW-WAR-0132). No adapter meters spend today, so every performance is
    /// unmetered and journals `spend: "unknown"`, never 0.
    ///
    /// Absent is not consent: `war perform` refuses
    /// `perform.unmetered-not-allowed` until this is `true` (U-001, settled at
    /// authorization). Setting it says "run it, and I know the cost is
    /// unknown"; it does not make the cost known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_unmetered: Option<bool>,
    /// A mandatory spend ceiling, such as `"1.00 USD"`. Nothing meters spend,
    /// so no cap can be enforced, and a cap that cannot be enforced refuses the
    /// run (`perform.spend-unenforceable`) rather than be claimed. The value is
    /// not parsed: any cap, of any amount, is one this tool cannot keep.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_spend_cap: Option<String>,
    /// How many repairs a stage gets: performances after its first accepted
    /// submission, counted by the ones that were themselves accepted. 0 means
    /// the default, [`DEFAULT_MAX_REPAIRS`] (3, the product spec's fallback).
    #[serde(default)]
    pub max_repairs: u32,
    /// How many recoveries a stage gets in a row: performances after an
    /// ending that was not an accepted submission (refused, timeout,
    /// cancelled, failed), counted since its last accepted one. 0 means the
    /// default, [`DEFAULT_MAX_RECOVERIES`] (2).
    #[serde(default)]
    pub max_recoveries: u32,
}

/// `[perform] max_repairs` when unset (OW-WAR-0132 U-001).
pub const DEFAULT_MAX_REPAIRS: u32 = 3;
/// `[perform] max_recoveries` when unset (OW-WAR-0132 U-001).
pub const DEFAULT_MAX_RECOVERIES: u32 = 2;

impl PerformPolicy {
    #[must_use]
    pub fn concurrency(&self) -> u32 {
        if self.max_concurrent == 0 {
            1
        } else {
            self.max_concurrent
        }
    }

    /// The repair limit in force: [`Self::max_repairs`], or the default.
    #[must_use]
    pub fn repairs(&self) -> u32 {
        if self.max_repairs == 0 {
            DEFAULT_MAX_REPAIRS
        } else {
            self.max_repairs
        }
    }

    /// The recovery limit in force: [`Self::max_recoveries`], or the default.
    #[must_use]
    pub fn recoveries(&self) -> u32 {
        if self.max_recoveries == 0 {
            DEFAULT_MAX_RECOVERIES
        } else {
            self.max_recoveries
        }
    }

    /// Only an explicit `true` admits an unmetered performer.
    #[must_use]
    pub fn unmetered_allowed(&self) -> bool {
        self.allow_unmetered == Some(true)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.performer_argv.is_empty()
            && self.performer_timeout_secs == 0
            && self.max_concurrent == 0
            && self.allow_unmetered.is_none()
            && self.hard_spend_cap.is_none()
            && self.max_repairs == 0
            && self.max_recoveries == 0
    }
}

/// `[run]` — `war run`'s bounds for a service stage (slice C4b).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPolicy {
    /// Wall-clock bound for a stage that declares none; 0 means 600.
    #[serde(default)]
    pub default_wall_time_seconds: u64,
}

impl RunPolicy {
    #[must_use]
    pub fn wall_time_seconds(&self) -> u64 {
        if self.default_wall_time_seconds == 0 {
            600
        } else {
            self.default_wall_time_seconds
        }
    }
}

/// `[verify]` — a configured blind verifier (slice C3, §75.2).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct VerifyPolicy {
    /// The command that reads a bundle path and prints a verification
    /// response on stdout. Empty means no verifier is configured, and
    /// `war verify --run` says so.
    #[serde(default)]
    pub verifier_argv: Vec<String>,
    /// Wall-clock bound for the verifier; 0 means the default (600).
    #[serde(default)]
    pub verifier_timeout_secs: u64,
    /// Bytes of a deliverable bundled whole; larger ones are truncated to
    /// this head with their full digest. 0 means the default (65536).
    #[serde(default)]
    pub max_excerpt_bytes: usize,
    /// The reading budget of one bundle, in estimated tokens over its whole
    /// canonical JSON (t-9f7e). A Warrant whose bundle fits is sent whole; one
    /// that does not is split into one bundle per obligation, each bounded to
    /// this by excerpts carrying whole-file digests. 0 means the default
    /// (48000).
    #[serde(default)]
    pub max_bundle_tokens: u64,
}

impl VerifyPolicy {
    #[must_use]
    pub fn timeout_secs(&self) -> u64 {
        if self.verifier_timeout_secs == 0 {
            600
        } else {
            self.verifier_timeout_secs
        }
    }

    #[must_use]
    pub fn max_excerpt_bytes(&self) -> usize {
        if self.max_excerpt_bytes == 0 {
            65_536
        } else {
            self.max_excerpt_bytes
        }
    }

    #[must_use]
    pub fn max_bundle_tokens(&self) -> u64 {
        if self.max_bundle_tokens == 0 {
            48_000
        } else {
            self.max_bundle_tokens
        }
    }
}

/// `[context]` — the Dispatch token budget (slice C2, §33.7).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ContextPolicy {
    /// The budget a stage gets when it declares none. Absent means the
    /// tool's default (32 000).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_budget_tokens: Option<u64>,
}

impl ContextPolicy {
    pub const DEFAULT_BUDGET_TOKENS: u64 = 32_000;

    #[must_use]
    pub fn budget(&self) -> u64 {
        self.default_budget_tokens
            .unwrap_or(Self::DEFAULT_BUDGET_TOKENS)
    }
}

/// `[adoption]` — where governed work begins in a repository adopted with
/// history (OW-WAR-0124).
///
/// Configuration, not an authority record: it says which commit `war init`
/// started from, so `war telemetry` counts untracked work from there rather
/// than from the first commit ever made. It claims nothing about the commits
/// before it — no Warrant authorized, owns or verified them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptionPolicy {
    /// The full commit id `war init` recorded: HEAD, or `--baseline`.
    pub baseline: String,
}

/// `[authority]` — the protected store this repository adopts (OW-WAR-0138).
///
/// CONTINGENT on the owner answering OW-WAR-0138 U-001 option A: opt-in per
/// repository. Absent, every key below is read from this file and `war check`
/// warns `authority.unprotected`. Present, the store governs the actor
/// binding and the protected policy keys, with no fallback: a store that
/// cannot be read fails closed, never back to `roles.toml`.
///
/// This table is itself in a file the performer can write. Deleting it is a
/// commit a reviewer sees and turns the warning back on; it cannot make the
/// store say something else. An older `war` that does not know the table
/// ignores it, which `[project] requires_war` exists to refuse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityStoreConfig {
    /// Absolute path of the store directory `war authority bootstrap` made.
    pub store: String,
    /// The store is a same-account test store (`--unprotected-test-store`).
    /// Must match the store's own record; every diagnostic says so.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unprotected_test_store: bool,
}

/// Where the protected keys came from for this process (OW-WAR-0138).
///
/// Not part of the file: set once when the repository is opened, from the
/// store, and read by `war check`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Governance {
    /// No `[authority]` table: `openwarrant.toml` is the only source.
    #[default]
    Unprotected,
    /// A v2 store governs, at `head`.
    Store {
        store: String,
        head: String,
        test_mode: bool,
        /// Keys whose `openwarrant.toml` value differs from the store's.
        divergences: Vec<Divergence>,
    },
    /// A store is configured and gave no policy (unreadable, refused, or a v1
    /// head). The protected keys take their most restrictive values.
    FailedClosed {
        store: String,
        rule: &'static str,
        why: String,
    },
}

/// One protected key whose `openwarrant.toml` value is not the store's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub key: &'static str,
    pub file: String,
    pub store: String,
}

fn shown<T: Serialize>(v: &T) -> String {
    serde_json::to_string(v).unwrap_or_else(|_| "<unprintable>".to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub schema: String,
    pub project: Project,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub generated: GeneratedPolicy,
    /// §27.3's standing permission. Absent means absent, which means no.
    #[serde(default)]
    pub policy: AuthorityPolicy,
    /// §75.2 — the configured drafting process, if any.
    #[serde(default)]
    pub plan: PlanPolicy,
    #[serde(default)]
    pub context: ContextPolicy,
    #[serde(default)]
    pub verify: VerifyPolicy,
    #[serde(default)]
    pub run: RunPolicy,
    /// `[perform]` — the agent that performs an agent stage (OW-WAR-0069).
    #[serde(default, skip_serializing_if = "PerformPolicy::is_empty")]
    pub perform: PerformPolicy,
    #[serde(default, skip_serializing_if = "SignPolicy::is_empty")]
    pub sign: SignPolicy,
    /// §46.1's nine independence dimensions, for verification performed in this
    /// repository.
    ///
    /// `Option`, and deliberately NOT defaulted to [`crate::Independence::default`].
    /// §46.1's own defaults are mostly `true`, so a repository that never
    /// declared independence would inherit a claim that its verification is
    /// blind and separately workspaced. Absent means UNDECLARED, and `war check`
    /// reports it as such rather than assuming the flattering answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub independence: Option<crate::independence::Independence>,
    /// `[adoption]` — absent for a repository initialized with no history,
    /// and never written when absent, so every existing `openwarrant.toml`
    /// loads and writes unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adoption: Option<AdoptionPolicy>,
    /// `[authority]` — the protected store, if this repository adopted one
    /// (OW-WAR-0138). Absent in every existing file, and never written absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<AuthorityStoreConfig>,
    /// Where the protected keys came from; set when the repository is
    /// opened, never read from or written to the file.
    #[serde(skip)]
    pub governance: Governance,
}

impl RepositoryConfig {
    /// Construct the configuration `war init` writes for a new repository.
    #[must_use]
    pub fn new(name: impl Into<String>, namespace: Namespace) -> Self {
        Self {
            schema: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            project: Project {
                name: name.into(),
                namespace,
                knowledge_fabric_project_ref: None,
                performer: None,
                repository_url: None,
                requires_war: None,
            },
            paths: Paths::default(),
            generated: GeneratedPolicy::default(),
            // A new repository does not permit automated resolution. `war init`
            // must never hand a fresh project the one setting that lets a
            // machine close its work.
            policy: AuthorityPolicy::default(),
            plan: PlanPolicy::default(),
            context: ContextPolicy::default(),
            verify: VerifyPolicy::default(),
            run: RunPolicy::default(),
            perform: PerformPolicy::default(),
            sign: SignPolicy::default(),
            independence: None,
            adoption: None,
            authority: None,
            governance: Governance::Unprotected,
        }
    }

    /// The protected keys as the store holds them, with each file value that
    /// differs recorded as a [`Divergence`] (OW-WAR-0138 D-006). The store's
    /// value governs: after this call every consumer reading
    /// `self.policy`, `self.verify.verifier_argv` or `self.independence`
    /// reads the store's.
    pub fn govern_from_store(
        &mut self,
        store: &str,
        head: &str,
        test_mode: bool,
        policy: &crate::authority_transition::Policy,
    ) {
        let mut divergences = Vec::new();
        let mut differ = |key: &'static str, file: String, store: String| {
            if file != store {
                divergences.push(Divergence { key, file, store });
            }
        };
        differ(
            "[policy] allow_automated_resolution",
            shown(&self.policy.allow_automated_resolution),
            shown(&policy.allow_automated_resolution),
        );
        differ(
            "[policy] require_user_presence",
            shown(&self.policy.require_user_presence),
            shown(&policy.require_user_presence),
        );
        differ(
            "[verify] verifier_argv",
            shown(&self.verify.verifier_argv),
            shown(&policy.verifier_argv),
        );
        differ(
            "[independence]",
            shown(&self.independence),
            shown(&policy.independence),
        );
        self.policy.allow_automated_resolution = policy.allow_automated_resolution;
        self.policy.require_user_presence = policy.require_user_presence;
        self.verify.verifier_argv.clone_from(&policy.verifier_argv);
        self.independence = policy.independence;
        self.governance = Governance::Store {
            store: store.to_owned(),
            head: head.to_owned(),
            test_mode,
            divergences,
        };
    }

    /// A store is configured and gave no policy: every protected key takes
    /// its most restrictive value — no automated resolution, presence
    /// required, no verifier, independence undeclared. Never the file's.
    pub fn govern_fail_closed(&mut self, store: &str, rule: &'static str, why: String) {
        self.policy.allow_automated_resolution = false;
        self.policy.require_user_presence = true;
        self.verify.verifier_argv.clear();
        self.independence = None;
        self.governance = Governance::FailedClosed {
            store: store.to_owned(),
            rule,
            why,
        };
    }

    /// Fail-closed validation (§91.1 test 4).
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema != REPOSITORY_CONFIG_SCHEMA {
            return Err(ConfigError::UnknownSchema {
                found: self.schema.clone(),
                expected: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            });
        }
        if self.project.name.trim().is_empty() {
            return Err(ConfigError::ProjectNameEmpty);
        }
        if let Some(req) = &self.project.requires_war {
            VersionReq::parse(req).map_err(|why| ConfigError::RequiresWarMalformed {
                found: req.clone(),
                why,
            })?;
        }
        if let Some(a) = &self.authority
            && (!a.store.starts_with('/') || a.store.split('/').any(|c| c == ".."))
        {
            return Err(ConfigError::AuthorityStoreNotAbsolute {
                found: a.store.clone(),
            });
        }
        self.sign.validate()?;
        self.paths.validate()
    }
}

/// A `war` version requirement, `[project] requires_war` (OW-WAR-0130).
///
/// Cargo's comparator syntax, without the dependency: one or more
/// comma-separated comparators, each `>=`, `>`, `<=`, `<`, `=`, `^`, `~` or
/// bare (which is `^`), over a version of one to three numeric parts, or of
/// three with a pre-release (`=1.0.0-alpha.2`). Versions order as semver
/// orders them — a pre-release below its release, so `>=1.0.0` does not
/// admit `1.0.0-alpha.2` — and wildcards are refused rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionReq {
    bounds: Vec<Bounds>,
}

/// One comparator: a lower and an upper bound, each `(key, inclusive)`.
type Bounds = (Option<(VersionKey, bool)>, Option<(VersionKey, bool)>);

/// A version as semver orders it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct VersionKey(u64, u64, u64, Pre);

/// The pre-release part. `Pre(vec![])` is the least version of its triple;
/// a release is greater than every pre-release of it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Pre {
    Pre(Vec<PreId>),
    Release,
}

/// Numeric identifiers order below alphanumeric ones, as semver says.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PreId {
    Num(u64),
    Alnum(String),
}

impl VersionKey {
    /// `1.2.3`, `1.2.3-alpha.2`, `1.2.3+build`; `None` for anything else.
    fn parse(v: &str) -> Option<Self> {
        let v = v.trim();
        let v = v.split_once('+').map_or(v, |(c, _)| c);
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c, Some(p)),
            None => (v, None),
        };
        let n: Vec<u64> = core
            .split('.')
            .map(str::parse)
            .collect::<Result<_, _>>()
            .ok()?;
        let [a, b, c] = n.as_slice() else {
            return None;
        };
        let pre = match pre {
            None => Pre::Release,
            Some(p) => Pre::Pre(Self::pre(p)?),
        };
        Some(Self(*a, *b, *c, pre))
    }

    fn pre(p: &str) -> Option<Vec<PreId>> {
        p.split('.')
            .map(|id| {
                if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                    None
                } else if let Ok(n) = id.parse() {
                    Some(PreId::Num(n))
                } else {
                    Some(PreId::Alnum(id.to_owned()))
                }
            })
            .collect()
    }
}

impl VersionReq {
    /// Parse a requirement; the error says which comparator and why.
    pub fn parse(raw: &str) -> Result<Self, String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err("it is empty".to_owned());
        }
        let mut bounds = Vec::new();
        for part in raw.split(',') {
            bounds.push(Self::comparator(part.trim())?);
        }
        Ok(Self { bounds })
    }

    fn comparator(c: &str) -> Result<Bounds, String> {
        let (op, rest) = ["<=", ">=", "<", ">", "=", "^", "~"]
            .iter()
            .find_map(|op| c.strip_prefix(op).map(|r| (*op, r.trim())))
            .unwrap_or(("^", c));
        if rest.is_empty() {
            return Err(format!("{c:?} names no version"));
        }
        let (core, pre) = match rest.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (rest, None),
        };
        let parts: Vec<&str> = core.split('.').collect();
        if parts.len() > 3 {
            return Err(format!("{rest:?} has more than three parts"));
        }
        let mut n = [0u64; 3];
        for (i, p) in parts.iter().enumerate() {
            n[i] = p
                .parse()
                .map_err(|_| format!("{rest:?}: {p:?} is not a number (wildcards are not read)"))?;
        }
        let given = parts.len();
        let [ma, mi, pa] = n;
        let pre = match pre {
            None => Pre::Release,
            Some(_) if given != 3 => {
                return Err(format!("{rest:?}: a pre-release needs all three parts"));
            }
            Some(p) => Pre::Pre(
                VersionKey::pre(p)
                    .ok_or_else(|| format!("{rest:?}: {p:?} is not a pre-release"))?,
            ),
        };
        let exact = VersionKey(ma, mi, pa, pre);
        // The least version of a triple: its lowest pre-release.
        let least = |a, b, c| VersionKey(a, b, c, Pre::Pre(vec![]));
        let at = |k: VersionKey| Some((k, true));
        let below = |k: VersionKey| Some((k, false));
        // The first version past the given precision: 1.2 → 1.3.0, 1 → 2.0.0.
        let next = match given {
            1 => least(ma + 1, 0, 0),
            2 => least(ma, mi + 1, 0),
            _ => least(ma, mi, pa + 1),
        };
        let full = given == 3;
        Ok(match op {
            ">=" => (at(exact), None),
            ">" if full => (Some((exact, false)), None),
            ">" => (at(next), None),
            "<" => (None, below(exact)),
            "<=" if full => (None, at(exact)),
            "<=" => (None, below(next)),
            "=" if full => (at(exact.clone()), at(exact)),
            "=" => (at(exact), below(next)),
            "~" => {
                let upper = if given == 1 {
                    least(ma + 1, 0, 0)
                } else {
                    least(ma, mi + 1, 0)
                };
                (at(exact), below(upper))
            }
            _ => {
                // Caret: the leftmost non-zero part given may not move.
                let upper = if ma > 0 || given == 1 {
                    least(ma + 1, 0, 0)
                } else if mi > 0 || given == 2 {
                    least(0, mi + 1, 0)
                } else {
                    least(0, 0, pa + 1)
                };
                (at(exact), below(upper))
            }
        })
    }

    /// Whether `version` satisfies every comparator. A version that does not
    /// parse satisfies nothing.
    #[must_use]
    pub fn matches(&self, version: &str) -> bool {
        let Some(key) = VersionKey::parse(version) else {
            return false;
        };
        self.bounds.iter().all(|(lo, hi)| {
            lo.as_ref()
                .is_none_or(|(k, incl)| if *incl { key >= *k } else { key > *k })
                && hi
                    .as_ref()
                    .is_none_or(|(k, incl)| if *incl { key <= *k } else { key < *k })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_version_requirement_admits_and_refuses_as_cargo_would() {
        let admits = |req: &str, v: &str| VersionReq::parse(req).expect(req).matches(v);
        assert!(admits(">=1.0.0", "1.0.0"));
        assert!(!admits(">=99", "1.0.0"));
        assert!(
            !admits(">=1.0.0", "1.0.0-alpha.2"),
            "a pre-release is below its release"
        );
        assert!(admits("^1.1", "1.9.0") && !admits("^1.1", "2.0.0") && !admits("^1.1", "1.0.9"));
        assert!(admits("^0.2.3", "0.2.9") && !admits("^0.2.3", "0.3.0"));
        assert!(admits("~1.2", "1.2.7") && !admits("~1.2", "1.3.0"));
        assert!(admits("=1.2", "1.2.5") && !admits("=1.2.3", "1.2.4"));
        assert!(admits(">1.2", "1.3.0") && !admits(">1.2", "1.2.9"));
        assert!(admits("<=1.2", "1.2.9") && !admits("<=1.2", "1.3.0"));
        assert!(admits(">=1.2, <2", "1.5.0") && !admits(">=1.2, <2", "2.0.0"));
        assert!(!admits(">=1.0.0", "not a version"));
        assert!(admits("=1.0.0-alpha.2", "1.0.0-alpha.2"));
        assert!(!admits("=1.0.0-alpha.2", "1.0.0-alpha.3") && !admits("=1.0.0-alpha.2", "1.0.0"));
        assert!(
            admits(">=1.0.0-alpha.2", "1.0.0-alpha.10"),
            "numeric identifiers order as numbers"
        );
        assert!(admits(">=1.0.0-alpha.2", "1.0.0") && !admits(">=1.0.0-alpha.2", "1.0.0-alpha.1"));
        assert!(admits("<1.0.0", "0.9.9") && !admits("<1.0.0", "1.0.0"));
        for bad in ["", ">=", "1.x", ">=1.0-alpha", "1.2.3.4", "*", "=1.0.0-"] {
            assert!(VersionReq::parse(bad).is_err(), "{bad:?} must be refused");
        }
    }

    #[test]
    fn a_malformed_requires_war_is_refused_by_validation() {
        let mut c = valid();
        c.project.requires_war = Some(">=one".to_owned());
        assert!(matches!(
            c.validate(),
            Err(ConfigError::RequiresWarMalformed { .. })
        ));
        c.project.requires_war = Some(">=1.0".to_owned());
        assert_eq!(c.validate(), Ok(()));
    }

    fn valid() -> RepositoryConfig {
        RepositoryConfig::new("OpenWarrant", Namespace::parse("OW").expect("valid"))
    }

    /// OW-WAR-0138 OBL-003: with the store saying no, a file saying yes is a
    /// divergence, and a policy-service resolution is still refused — the
    /// value consumers read is the store's.
    #[test]
    fn the_store_governs_and_a_differing_file_value_is_a_divergence() {
        use crate::authority::{ActorRole, PolicyResolutionContext, RoleAssignment};
        let mut c = valid();
        c.policy.allow_automated_resolution = true;
        c.independence = Some(crate::independence::Independence::default());
        c.verify.verifier_argv = vec!["mine.sh".to_owned()];
        let store = crate::authority_transition::Policy {
            allow_automated_resolution: false,
            require_user_presence: true,
            verifier_argv: vec!["theirs.sh".to_owned()],
            independence: None,
        };
        c.govern_from_store("/s", "sha256:h", true, &store);
        assert!(!c.policy.allow_automated_resolution);
        assert!(c.policy.require_user_presence);
        assert_eq!(c.verify.verifier_argv, vec!["theirs.sh".to_owned()]);
        assert_eq!(c.independence, None);
        let Governance::Store { divergences, .. } = &c.governance else {
            panic!("{:?}", c.governance);
        };
        let keys: Vec<&str> = divergences.iter().map(|d| d.key).collect();
        assert_eq!(
            keys,
            [
                "[policy] allow_automated_resolution",
                "[policy] require_user_presence",
                "[verify] verifier_argv",
                "[independence]"
            ]
        );
        assert_eq!(divergences[0].file, "true");
        assert_eq!(divergences[0].store, "false");
        let service = RoleAssignment {
            actor: "closer".to_owned(),
            actor_kind: crate::authority::ActorKind::PolicyService,
            roles: [ActorRole::Resolver].into_iter().collect(),
            assigned_by: "test".to_owned(),
            effective_time: "2026-01-01T00:00:00Z".to_owned(),
            note: None,
            ssh_principal: None,
        };
        let ctx = |allows| PolicyResolutionContext {
            policy_allows: allows,
            assurance_level: "basic",
            all_obligations_mechanical: true,
            residual_risk_judgment_required: false,
        };
        assert!(
            service
                .may_resolve("claude", ctx(c.policy.allow_automated_resolution))
                .is_err(),
            "the store's false governs"
        );
        assert!(
            service.may_resolve("claude", ctx(true)).is_ok(),
            "the refusal is the policy's, not something else's"
        );
    }

    #[test]
    fn equal_values_are_no_divergence_and_a_failed_store_closes_every_key() {
        let mut c = valid();
        c.govern_from_store(
            "/s",
            "sha256:h",
            false,
            &crate::authority_transition::Policy::default(),
        );
        assert!(
            matches!(&c.governance, Governance::Store { divergences, .. } if divergences.is_empty())
        );
        let mut c = valid();
        c.policy.allow_automated_resolution = true;
        c.verify.verifier_argv = vec!["v".to_owned()];
        c.independence = Some(crate::independence::Independence::default());
        c.govern_fail_closed("/s", "authority.verify-unavailable", "gone".to_owned());
        assert!(!c.policy.allow_automated_resolution);
        assert!(c.policy.require_user_presence);
        assert!(c.verify.verifier_argv.is_empty());
        assert!(c.independence.is_none());
    }

    #[test]
    fn an_authority_store_is_absolute_and_absent_by_default() {
        let mut c = valid();
        assert!(c.authority.is_none());
        let text = toml::to_string(&c).expect("serializes");
        assert!(!text.contains("[authority]"), "{text}");
        c.authority = Some(AuthorityStoreConfig {
            store: "relative/store".to_owned(),
            unprotected_test_store: false,
        });
        assert!(matches!(
            c.validate(),
            Err(ConfigError::AuthorityStoreNotAbsolute { .. })
        ));
        c.authority = Some(AuthorityStoreConfig {
            store: "/var/lib/ow/../x".to_owned(),
            unprotected_test_store: false,
        });
        assert!(c.validate().is_err());
        c.authority = Some(AuthorityStoreConfig {
            store: "/var/lib/openwarrant/example".to_owned(),
            unprotected_test_store: false,
        });
        assert_eq!(c.validate(), Ok(()));
    }

    #[test]
    fn default_config_validates() {
        assert_eq!(valid().validate(), Ok(()));
    }

    #[test]
    fn default_generated_policy_commits_and_verifies() {
        let config = valid();
        assert!(config.generated.commit);
        assert!(config.generated.verify_drift, "drift check must default on");
        assert!(
            !config.generated.history,
            "the history projection is opt-in"
        );
    }

    /// A preset that offers itself for a correction and names no kind is a
    /// preset that would have the tool decide what the signer is claiming.
    #[test]
    fn a_correction_preset_without_a_kind_is_refused() {
        let mut config = valid();
        config.sign.presets.push(SignPreset {
            key: "1".to_owned(),
            label: "the bytes moved".to_owned(),
            meaning: "They moved with the plan.".to_owned(),
            acts: vec!["correct".to_owned()],
            kind: String::new(),
        });
        assert_eq!(
            config.validate(),
            Err(ConfigError::PresetKindMissing {
                key: "1".to_owned()
            })
        );
        config.sign.presets[0].kind = "behaviour-change".to_owned();
        assert_eq!(config.validate(), Ok(()));
        // An authorize preset needs no kind: there is nothing to choose.
        config.sign.presets[0].acts = vec!["authorize".to_owned()];
        config.sign.presets[0].kind = String::new();
        assert_eq!(config.validate(), Ok(()));
    }

    /// §69.3 / §91.1 test 4: an unrecognised schema is refused, not ignored.
    #[test]
    fn unknown_schema_fails_closed() {
        let mut config = valid();
        config.schema = "oh.war/repository-config/v2".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::UnknownSchema {
                found: "oh.war/repository-config/v2".to_owned(),
                expected: REPOSITORY_CONFIG_SCHEMA.to_owned(),
            })
        );
    }

    #[test]
    fn namespace_rules() {
        assert!(Namespace::parse("OW").is_ok());
        assert!(Namespace::parse("OPEN-HUMAN").is_ok());
        assert!(Namespace::parse("OW2").is_ok());
        for bad in ["", "  ", "ow", "-OW", "OW-", "O W", "OW_X"] {
            assert!(Namespace::parse(bad).is_err(), "expected {bad:?} refused");
        }
    }

    #[test]
    fn absolute_paths_are_refused() {
        let mut config = valid();
        config.paths.warrants = "/absolute/elsewhere/docs/warrants".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::PathNotRelative {
                field: "warrants",
                value: "/absolute/elsewhere/docs/warrants".to_owned(),
            })
        );
    }

    /// OW-WAR-0124: a config with no `[adoption]` writes no such table, and
    /// one that records a baseline reads it back.
    #[test]
    fn adoption_is_absent_unless_recorded() {
        let config = valid();
        let text = toml::to_string_pretty(&config).expect("serializes");
        assert!(!text.contains("adoption"), "{text}");
        let back: RepositoryConfig = toml::from_str(&text).expect("parses");
        assert_eq!(back.adoption, None);
        let mut adopted = valid();
        adopted.adoption = Some(AdoptionPolicy {
            baseline: "0123456789abcdef0123456789abcdef01234567".to_owned(),
        });
        let text = toml::to_string_pretty(&adopted).expect("serializes");
        assert!(text.contains("[adoption]"), "{text}");
        let back: RepositoryConfig = toml::from_str(&text).expect("parses");
        assert_eq!(back, adopted);
    }

    /// OW-WAR-0138: absent means no, a written config omits it while false,
    /// and a human's `true` reads back.
    #[test]
    fn user_presence_is_off_unless_written() {
        let config = valid();
        assert!(!config.policy.require_user_presence);
        let text = toml::to_string_pretty(&config).expect("serializes");
        assert!(!text.contains("require_user_presence"), "{text}");
        let on: AuthorityPolicy = toml::from_str("require_user_presence = true\n").expect("parses");
        assert!(on.require_user_presence);
        assert!(!on.allow_automated_resolution);
        let text = toml::to_string_pretty(&on).expect("serializes");
        assert!(text.contains("require_user_presence = true"), "{text}");
    }

    #[test]
    fn empty_path_is_refused() {
        let mut config = valid();
        config.paths.adrs = "  ".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::PathEmpty { field: "adrs" })
        );
    }

    /// OW-WAR-0132 U-001: absent is refusal, not consent; 0 is the default.
    #[test]
    fn perform_spend_and_attempt_defaults() {
        let p = PerformPolicy::default();
        assert!(
            !p.unmetered_allowed(),
            "absent allow_unmetered is not consent"
        );
        assert_eq!((p.repairs(), p.recoveries()), (3, 2));
        let set: PerformPolicy = toml::from_str(
            "allow_unmetered = true\nhard_spend_cap = \"1.00 USD\"\nmax_repairs = 1\nmax_recoveries = 5\n",
        )
        .expect("parses");
        assert!(set.unmetered_allowed());
        assert_eq!(set.hard_spend_cap.as_deref(), Some("1.00 USD"));
        assert_eq!((set.repairs(), set.recoveries()), (1, 5));
        assert!(!set.is_empty());
        let no: PerformPolicy = toml::from_str("allow_unmetered = false\n").expect("parses");
        assert!(!no.unmetered_allowed());
        assert!(
            !no.is_empty(),
            "an explicit false is written back, not dropped"
        );
    }
}
