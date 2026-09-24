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
}

impl PerformPolicy {
    #[must_use]
    pub fn concurrency(&self) -> u32 {
        if self.max_concurrent == 0 {
            1
        } else {
            self.max_concurrent
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.performer_argv.is_empty()
            && self.performer_timeout_secs == 0
            && self.max_concurrent == 0
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
        }
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

    #[test]
    fn empty_path_is_refused() {
        let mut config = valid();
        config.paths.adrs = "  ".to_owned();
        assert_eq!(
            config.validate(),
            Err(ConfigError::PathEmpty { field: "adrs" })
        );
    }
}
