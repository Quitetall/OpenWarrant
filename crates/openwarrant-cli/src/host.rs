// SPDX-License-Identifier: Apache-2.0
//! `war host` — the standalone half of Liminal hosting, `oh.war/liminal-v1`
//! (OW-WAR-0148 M8; SAS §11.3, §82.2–82.4; OW-ADR-0031; docs/LIMINAL_HOST.md).
//!
//! One request on stdin, one response on stdout:
//!
//! ```text
//! request  = { protocol: "oh.war/liminal-v1", version: 1,
//!              basis: { id, digest, members[{path, digest, utf8|hex}] },
//!              profiles[{name, digest, utf8}],
//!              nodes[{id, type, source, revision, jurisdiction?}],
//!              relations[{from, kind, to, to_revision?}],
//!              observations?: { signatures[], git[] },
//!              options?: { projections[{kind, subject}], limits{…} } }
//! response = { protocol, version, outcome, basis{id, digest}, model,
//!              model_digest, diagnostics[], projections[], observations,
//!              limits, refusal }
//! ```
//!
//! **One implementation.** The model is built by [`crate::model::build`]
//! over a [`Corpus`] whose files are the request's basis, held in memory by
//! [`crate::vfs`] — the same code `war model` runs over the disk. Nothing
//! here interprets a record.
//!
//! **Pure.** A hosted run reads no repository and no other file, writes
//! nothing, opens no network connection and spawns no process. The two facts
//! the readers otherwise observe outside the bytes — an `ssh-keygen -Y
//! verify` verdict and a `git` read of history — are answered from the
//! request's `observations`, keyed by exactly the bytes or arguments they
//! depend on. The host is trusted for them (owner decision, 2026-10-07; see
//! docs/LIMINAL_HOST.md): they are used as given, never re-checked here, and
//! the response says which were used; absent one, the reader fails closed, exactly as a
//! missing `ssh-keygen` or `git` does, and the response names each as an
//! unknown.
//!
//! **Refusals before any work, by name:** `host.limit`, `host.malformed`,
//! `host.protocol`, `host.version`, `host.path-not-bytes`,
//! `host.member-path`, `host.member-duplicate`, `host.member-digest`,
//! `host.basis-digest`, `host.profile-name`, `host.profile-in-basis`,
//! `host.observation`, `host.projection-kind`.
//!
//! **Exit codes:** 0 compiled, and the request's Nodes and Relations are the
//! compiled ones, every requested projection rendered and every observation
//! the run needed supplied; 1 refused (no model); 2 compiled, but something
//! is not established — a Node or Relation differs, a projection is
//! unavailable, an observation was missing, or the basis is no repository.
//!
//! `war host --export` writes the request that reproduces a repository's
//! model: every file the standalone build read, listed or found, the
//! observations it made, and its records and relations as Nodes and
//! Relations. Standalone and hosted runs of the same compiler over it give
//! the same model, byte for byte (`conformance/plants.d/79-host.sh`).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::rc::Rc;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::corpus::Corpus;
use crate::model::{self, Model};
use crate::repo::{RepoError, Repository};
use crate::sdk::wire;
use crate::vfs;

pub const PROTOCOL: &str = "oh.war/liminal-v1";
pub const VERSION: u32 = 1;
/// Where a hosted basis is mounted. No file is ever read there: every read
/// under it is answered from memory, and every read elsewhere finds nothing.
pub const VIRTUAL_ROOT: &str = "/liminal-basis";

/// The declared limits. A request may lower any of them, never raise one.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// Bytes of the request.
    pub input_bytes: u64,
    /// JSON values in the request.
    pub json_nodes: u64,
    /// Basis members plus profiles.
    pub members: u64,
    /// Bytes of one member or profile.
    pub member_bytes: u64,
    /// Requested projections.
    pub projections: u64,
    /// Bytes of the response.
    pub output_bytes: u64,
}

pub const LIMITS: Limits = Limits {
    input_bytes: 256 * 1024 * 1024,
    json_nodes: 1_000_000,
    members: 50_000,
    member_bytes: 32 * 1024 * 1024,
    projections: 10_000,
    output_bytes: 256 * 1024 * 1024,
};

/// Lowered limits a request may declare.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_nodes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projections: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_bytes: Option<u64>,
}

impl Limits {
    fn lowered(self, d: &DeclaredLimits) -> Self {
        let low = |hard: u64, ask: Option<u64>| ask.map_or(hard, |a| a.min(hard));
        Self {
            input_bytes: low(self.input_bytes, d.input_bytes),
            json_nodes: low(self.json_nodes, d.json_nodes),
            members: low(self.members, d.members),
            member_bytes: low(self.member_bytes, d.member_bytes),
            projections: low(self.projections, d.projections),
            output_bytes: low(self.output_bytes, d.output_bytes),
        }
    }
}

// ---------------------------------------------------------------------------
// The request
// ---------------------------------------------------------------------------

/// `oh.war/liminal-v1` request.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// `oh.war/liminal-v1`.
    pub protocol: String,
    /// `1`.
    pub version: u32,
    /// The Workspace Basis: every file of the WAR records, by its exact bytes.
    pub basis: Basis,
    /// Profile data (`oh.war/profile/v1` TOML), mounted at
    /// `profiles/<name>.toml`.
    pub profiles: Vec<Profile>,
    /// The WAR records as Liminal holds them. Compared with the compiled
    /// records; a difference is reported, never repaired.
    pub nodes: Vec<Node>,
    /// The WAR relations as Liminal holds them, compared likewise.
    pub relations: Vec<model::Relation>,
    /// Facts observed outside the bytes, taken on the host's trust and never
    /// re-checked here.
    #[serde(default, skip_serializing_if = "Observations::is_empty")]
    pub observations: Observations,
    #[serde(default, skip_serializing_if = "Options::is_empty")]
    pub options: Options,
}

/// A Workspace Basis (Liminal): an immutable set of members.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Basis {
    /// Liminal's name for it (`liminal-basis://…`); echoed, never interpreted.
    pub id: String,
    /// `sha256:` over the JCS of the sorted `[path, digest]` pairs.
    pub digest: String,
    pub members: Vec<Member>,
}

/// One member of a basis: a path and its exact bytes, never a path alone.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    /// Basis-relative, `/`-separated, no `.`/`..`, no leading `/`.
    pub path: String,
    /// `sha256:<hex>` of the bytes.
    pub digest: String,
    /// The bytes, when they are UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utf8: Option<String>,
    /// The bytes as lowercase hex, otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hex: Option<String>,
    /// Held by digest only, deliberately: the member is listed and found,
    /// and a reader that needs its bytes is reported (`host.member-withheld`).
    /// `war host --export` withholds what the standalone build listed and
    /// never read.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub withheld: bool,
}

/// One profile file.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// `ticket` for `profiles/ticket.toml`.
    pub name: String,
    pub digest: String,
    pub utf8: String,
}

/// A record as a Liminal Node (OW-ADR-0031).
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// The basis member that holds it.
    pub source: String,
    /// Its revision: a member of the Workspace Basis.
    pub revision: String,
    /// Who governs it: its Jurisdiction (`governed_by` in the model).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jurisdiction: Option<String>,
}

impl From<&model::Record> for Node {
    fn from(r: &model::Record) -> Self {
        Self {
            id: r.id.clone(),
            kind: r.kind.clone(),
            source: r.source.clone(),
            revision: r.revision.clone(),
            jurisdiction: r.governed_by.clone(),
        }
    }
}

/// Facts the readers observe outside the bytes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observations {
    #[serde(default)]
    pub signatures: Vec<SignatureObservation>,
    #[serde(default)]
    pub git: Vec<GitObservation>,
}

impl Observations {
    fn is_empty(&self) -> bool {
        self.signatures.is_empty() && self.git.is_empty()
    }
}

/// What `ssh-keygen -Y verify` answered for one set of bytes.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureObservation {
    /// sha256 hex over the length-prefixed allowed signers, principal,
    /// namespace, response, signature and key-list label.
    pub key: String,
    /// `verified` or `rejected`.
    pub verdict: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// What one `git` read of the tree's history answered.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitObservation {
    /// Its arguments, the repository left out.
    pub args: Vec<String>,
    /// Its stdout as lowercase hex, when it succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_hex: Option<String>,
    /// Why it failed, when it did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// What the request asks for beyond the model.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projections: Vec<ProjectionRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<DeclaredLimits>,
}

impl Options {
    fn is_empty(&self) -> bool {
        self.projections.is_empty() && self.limits.is_none()
    }
}

/// One requested projection.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionRequest {
    /// `warrant` (a Warrant's committed views). Kinds the projection
    /// targets of OW-WAR-0148 M6 add are answered through its render seam.
    pub kind: String,
    /// A Warrant alias, or `*` for every Warrant.
    pub subject: String,
}

// ---------------------------------------------------------------------------
// The response
// ---------------------------------------------------------------------------

/// `oh.war/liminal-v1` response.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: String,
    pub version: u32,
    /// `compiled`, `not_established` or `refused`.
    pub outcome: String,
    /// The basis echoed: its id and the digest the host recomputed.
    pub basis: Option<BasisEcho>,
    /// The compiled `oh.war/model/v1`; null when refused or not compiled.
    pub model: Option<Model>,
    /// `sha256:` of the model's compact JSON bytes, as serialized here.
    pub model_digest: Option<String>,
    /// Sorted by (rule, subject, message).
    pub diagnostics: Vec<HostDiagnostic>,
    /// In request order, `*` expanded in alias order.
    pub projections: Vec<Projection>,
    pub observations: ObservationUse,
    /// The limits this run held to.
    pub limits: Limits,
    pub refusal: Option<Refusal>,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BasisEcho {
    pub id: String,
    pub digest: String,
}

/// A finding of the host about the request, beside the model's own.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostDiagnostic {
    pub rule: String,
    pub subject: String,
    pub message: String,
    /// `error` or `unknown`.
    pub severity: String,
}

/// One rendered projection, or why it is not.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    pub kind: String,
    pub subject: String,
    /// Basis-relative path where the standalone compiler commits it.
    pub path: Option<String>,
    /// `rendered` or `unavailable`.
    pub status: String,
    pub digest: Option<String>,
    pub utf8: Option<String>,
    pub reason: Option<String>,
}

/// Which observations the run used. Every one is taken on the host's trust.
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationUse {
    pub signatures_supplied: usize,
    pub signatures_used: usize,
    pub signatures_missing: usize,
    pub git_supplied: usize,
    pub git_used: usize,
    pub git_missing: usize,
    /// Always false: `war` itself authenticates no observation; the host is
    /// trusted for them (docs/LIMINAL_HOST.md).
    pub authenticated: bool,
}

#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    pub rule: String,
    pub message: String,
}

#[derive(Debug)]
struct Refused(&'static str, String);

fn refuse<T>(rule: &'static str, message: impl Into<String>) -> Result<T, Refused> {
    Err(Refused(rule, message.into()))
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", openwarrant_compiler::sha256_hex(bytes))
}

fn hex_bytes(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let d = |b: u8| match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
    };
    hex.as_bytes()
        .chunks_exact(2)
        .map(|p| Some(d(p[0])? * 16 + d(p[1])?))
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// `sha256:` over the JCS of the sorted `[path, digest]` pairs.
#[must_use]
pub fn basis_digest(pairs: &[(String, String)]) -> String {
    let mut pairs: Vec<&(String, String)> = pairs.iter().collect();
    pairs.sort();
    let canonical = serde_jcs::to_string(&pairs).unwrap_or_default();
    digest(canonical.as_bytes())
}

/// Keys that locate bytes elsewhere instead of carrying them.
const LOCATORS: &[&str] = &[
    "file",
    "uri",
    "url",
    "root",
    "dir",
    "directory",
    "location",
    "filename",
];

fn has_locator(v: &Value) -> Option<&'static str> {
    let o = v.as_object()?;
    LOCATORS.iter().copied().find(|k| o.contains_key(*k))
}

fn member_path_ok(p: &str) -> bool {
    !p.is_empty()
        && !p.starts_with('/')
        && !p.contains('\\')
        && !p.contains('\0')
        && p.split('/').all(|s| !s.is_empty() && s != "." && s != "..")
}

/// The request, checked in full before any work.
struct Checked {
    request: Request,
    limits: Limits,
    files: BTreeMap<String, Vec<u8>>,
    withheld: BTreeSet<String>,
    basis_digest: String,
    ssh: BTreeMap<String, vfs::SshVerdict>,
    git: BTreeMap<Vec<String>, vfs::GitOutcome>,
}

fn check(bytes: &[u8]) -> Result<Checked, Refused> {
    if bytes.len() as u64 > LIMITS.input_bytes {
        return refuse(
            "host.limit",
            format!(
                "the request exceeds {} bytes (input_bytes)",
                LIMITS.input_bytes
            ),
        );
    }
    let value = wire::decode_value_within(
        bytes,
        usize::try_from(LIMITS.json_nodes).unwrap_or(usize::MAX),
    )
    .map_err(|e| {
        let m = e.to_string();
        if m.contains("resource-limit") {
            Refused(
                "host.limit",
                format!(
                    "the request exceeds {} JSON values or 64 levels (json_nodes)",
                    LIMITS.json_nodes
                ),
            )
        } else {
            Refused("host.malformed", format!("the request is not JSON: {m}"))
        }
    })?;
    let Some(top) = value.as_object() else {
        return refuse("host.malformed", "the request must be a JSON object");
    };
    match top.get("protocol").and_then(Value::as_str) {
        Some(PROTOCOL) => {}
        Some(other) => {
            return refuse(
                "host.protocol",
                format!("protocol {other:?} is not spoken here; this host speaks {PROTOCOL}"),
            );
        }
        None => return refuse("host.malformed", "the request names no `protocol`"),
    }
    match top.get("version") {
        Some(v) if v.as_u64() == Some(u64::from(VERSION)) => {}
        Some(v) => {
            return refuse(
                "host.version",
                format!(
                    "{PROTOCOL} version {v} is not supported; this host speaks version {VERSION}"
                ),
            );
        }
        None => return refuse("host.malformed", "the request names no `version`"),
    }
    // A path instead of bytes, wherever bytes are owed.
    if let Some(basis) = top.get("basis") {
        if let Some(k) = has_locator(basis) {
            return refuse(
                "host.path-not-bytes",
                format!(
                    "the basis names a `{k}` instead of carrying its members' bytes; a hosted run reads no file"
                ),
            );
        }
        for m in basis
            .get("members")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let path = m.get("path").and_then(Value::as_str).unwrap_or("?");
            if let Some(k) = has_locator(m) {
                return refuse(
                    "host.path-not-bytes",
                    format!(
                        "member {path} names a `{k}` instead of carrying its bytes; a hosted run reads no file"
                    ),
                );
            }
            if m.is_object()
                && m.get("utf8").is_none()
                && m.get("hex").is_none()
                && m.get("withheld") != Some(&Value::Bool(true))
            {
                return refuse(
                    "host.path-not-bytes",
                    format!(
                        "member {path} carries no bytes (`utf8` or `hex`) and is not `withheld`; a hosted run reads no file"
                    ),
                );
            }
        }
    }
    for p in top
        .get("profiles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = p.get("name").and_then(Value::as_str).unwrap_or("?");
        if let Some(k) = has_locator(p) {
            return refuse(
                "host.path-not-bytes",
                format!("profile {name} names a `{k}` instead of carrying its bytes"),
            );
        }
        if p.is_object() && p.get("utf8").is_none() {
            return refuse(
                "host.path-not-bytes",
                format!("profile {name} carries no bytes (`utf8`)"),
            );
        }
    }
    let request: Request = serde_json::from_value(value.clone())
        .map_err(|e| Refused("host.malformed", format!("the request is malformed: {e}")))?;
    wire::shape(
        &value,
        &serde_json::to_value(&request).map_err(|e| Refused("host.malformed", e.to_string()))?,
    )
    .map_err(|e| Refused("host.malformed", format!("the request is malformed: {e}")))?;

    // Declared limits, lowered only.
    let limits = LIMITS.lowered(&request.options.limits.unwrap_or_default());
    let over = |what: &str, n: u64, limit: u64| -> Result<(), Refused> {
        if n > limit {
            return refuse(
                "host.limit",
                format!("{what}: {n} exceeds the limit of {limit}"),
            );
        }
        Ok(())
    };
    over("input_bytes", bytes.len() as u64, limits.input_bytes)?;
    if limits.json_nodes < LIMITS.json_nodes
        && wire::decode_value_within(
            bytes,
            usize::try_from(limits.json_nodes).unwrap_or(usize::MAX),
        )
        .is_err()
    {
        return refuse(
            "host.limit",
            format!(
                "json_nodes: the request exceeds the limit of {} JSON values",
                limits.json_nodes
            ),
        );
    }
    over(
        "members",
        (request.basis.members.len() + request.profiles.len()) as u64,
        limits.members,
    )?;
    over(
        "projections",
        request.options.projections.len() as u64,
        limits.projections,
    )?;

    // Members: exact bytes under a safe path, each matching its digest.
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut withheld: BTreeSet<String> = BTreeSet::new();
    let mut pairs = Vec::new();
    for m in &request.basis.members {
        if !member_path_ok(&m.path) {
            return refuse(
                "host.member-path",
                format!(
                    "member path {:?} is not basis-relative (no leading `/`, `.`, `..` or `\\`)",
                    m.path
                ),
            );
        }
        if m.withheld {
            if m.utf8.is_some() || m.hex.is_some() {
                return refuse(
                    "host.malformed",
                    format!("member {} is withheld and carries bytes", m.path),
                );
            }
            if files.contains_key(&m.path) || !withheld.insert(m.path.clone()) {
                return refuse(
                    "host.member-duplicate",
                    format!("member {} is declared more than once", m.path),
                );
            }
            pairs.push((m.path.clone(), m.digest.clone()));
            continue;
        }
        let bytes = match (&m.utf8, &m.hex) {
            (Some(t), None) => t.as_bytes().to_vec(),
            (None, Some(h)) => hex_bytes(h).ok_or_else(|| {
                Refused(
                    "host.malformed",
                    format!("member {}: `hex` is not lowercase hex", m.path),
                )
            })?,
            _ => {
                return refuse(
                    "host.malformed",
                    format!("member {} carries both `utf8` and `hex`", m.path),
                );
            }
        };
        over(
            &format!("member {} bytes", m.path),
            bytes.len() as u64,
            limits.member_bytes,
        )?;
        if digest(&bytes) != m.digest {
            return refuse(
                "host.member-digest",
                format!(
                    "member {}: its bytes digest to {}, not the declared {}",
                    m.path,
                    digest(&bytes),
                    m.digest
                ),
            );
        }
        if withheld.contains(&m.path) || files.insert(m.path.clone(), bytes).is_some() {
            return refuse(
                "host.member-duplicate",
                format!("member {} is declared more than once", m.path),
            );
        }
        pairs.push((m.path.clone(), m.digest.clone()));
    }
    let recomputed = basis_digest(&pairs);
    if recomputed != request.basis.digest {
        return refuse(
            "host.basis-digest",
            format!(
                "the members digest to {recomputed}, not the declared {}",
                request.basis.digest
            ),
        );
    }
    for p in &request.profiles {
        if p.name.is_empty()
            || !p
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        {
            return refuse(
                "host.profile-name",
                format!("profile name {:?} is not [a-z0-9_-]+", p.name),
            );
        }
        over(
            &format!("profile {} bytes", p.name),
            p.utf8.len() as u64,
            limits.member_bytes,
        )?;
        if digest(p.utf8.as_bytes()) != p.digest {
            return refuse(
                "host.member-digest",
                format!(
                    "profile {}: its bytes do not digest to {}",
                    p.name, p.digest
                ),
            );
        }
        let path = format!("profiles/{}.toml", p.name);
        if withheld.contains(&path)
            || files
                .insert(path.clone(), p.utf8.as_bytes().to_vec())
                .is_some()
        {
            return refuse(
                "host.profile-in-basis",
                format!(
                    "{path} is both a basis member and a profile; profile data goes in `profiles` only"
                ),
            );
        }
    }
    let mut ssh = BTreeMap::new();
    for s in &request.observations.signatures {
        let v = match (s.verdict.as_str(), &s.reason) {
            ("verified", None) => Ok(()),
            ("rejected", Some(r)) => Err(r.clone()),
            _ => {
                return refuse(
                    "host.observation",
                    format!(
                        "signature observation {}: verdict is `verified` (no reason) or `rejected` (with one)",
                        s.key
                    ),
                );
            }
        };
        if ssh.insert(s.key.clone(), v).is_some() {
            return refuse(
                "host.observation",
                format!("signature observation {} is supplied twice", s.key),
            );
        }
    }
    let mut git = BTreeMap::new();
    for g in &request.observations.git {
        let v = match (&g.stdout_hex, &g.error) {
            (Some(h), None) => Ok(hex_bytes(h).ok_or_else(|| {
                Refused(
                    "host.observation",
                    format!(
                        "git observation {:?}: stdout_hex is not lowercase hex",
                        g.args
                    ),
                )
            })?),
            (None, Some(e)) => Err(e.clone()),
            _ => {
                return refuse(
                    "host.observation",
                    format!(
                        "git observation {:?}: exactly one of stdout_hex and error",
                        g.args
                    ),
                );
            }
        };
        if git.insert(g.args.clone(), v).is_some() {
            return refuse(
                "host.observation",
                format!("git observation {:?} is supplied twice", g.args),
            );
        }
    }
    for p in &request.options.projections {
        if !projection_kind_known(&p.kind) {
            return refuse(
                "host.projection-kind",
                format!(
                    "projection kind {:?} is not one this compiler renders ({})",
                    p.kind,
                    PROJECTION_KINDS.join(", ")
                ),
            );
        }
    }
    Ok(Checked {
        request,
        limits,
        files,
        withheld,
        basis_digest: recomputed,
        ssh,
        git,
    })
}

// ---------------------------------------------------------------------------
// Projections
// ---------------------------------------------------------------------------

/// The projection kinds this build renders.
pub const PROJECTION_KINDS: &[&str] = &["warrant"];

fn projection_kind_known(kind: &str) -> bool {
    PROJECTION_KINDS.contains(&kind) || m6_render_seam_kinds().contains(&kind)
}

/// The projection kinds M6's pure renderer answers (OW-WAR-0148 M6):
/// `document`, a declared document's projections (`docs/records/<area>/
/// documents.toml`), by subject `<area>/<name>` or `*` for every one.
fn m6_render_seam_kinds() -> &'static [&'static str] {
    &["document"]
}

/// Render `kind` of `subject` with the same call `war compile` makes
/// (`render_cmd::compile_all`, bounded by each document's budget), over this
/// corpus and model, as `(basis-relative path, bytes)`. One implementation:
/// a hosted rendering cannot differ from a standalone one.
fn m6_render_seam(
    corpus: &Corpus,
    model: &Model,
    kind: &str,
    subject: &str,
) -> Option<Result<Vec<(String, String)>, String>> {
    if kind != "document" {
        return None;
    }
    let repo = corpus.repo();
    let (declared, faults) = crate::render_cmd::documents(repo);
    if let Some(f) = faults.first() {
        return Some(Err(format!("the documents declaration is refused: {f:?}")));
    }
    let chosen: Vec<_> = declared
        .into_iter()
        .filter(|d| subject == "*" || d.id == subject)
        .collect();
    if chosen.is_empty() {
        return Some(Err(format!("{subject} is no declared document of this basis")));
    }
    let input = crate::render_cmd::input(corpus, model);
    let mut views = Vec::new();
    for c in crate::render_cmd::compile_all(corpus, &chosen, &input, true) {
        match c.rendered {
            Ok(p) => views.push((repo.relative(&c.path).to_string(), p.content)),
            Err(e) => return Some(Err(format!("{}: {}: {e}", e.rule(), c.document.id))),
        }
    }
    Some(Ok(views))
}

/// A Warrant's committed views, exactly as `war compile` renders them.
fn warrant_views(corpus: &Corpus, alias: &str) -> Result<Vec<(String, String)>, String> {
    let repo = corpus.repo();
    let entry = corpus
        .entry(alias)
        .ok_or_else(|| format!("{alias} is no Warrant of this basis"))?;
    let one = entry.loaded().map_err(|e| e.to_string())?;
    let (Some(basis), Some(validated)) = (&one.basis, &one.validated) else {
        return Err(format!(
            "{alias}: the manifest did not validate, so it has no projection"
        ));
    };
    let loaded: Vec<&crate::repo::Loaded> = corpus
        .entries()
        .map_err(|e| e.to_string())?
        .iter()
        .filter_map(crate::corpus::Entry::ok)
        .collect();
    let children =
        crate::compile::children_of_with(&validated.raw.uuid, &loaded, corpus.currencies());
    let views = match entry.ir() {
        Ok(ir) => crate::compile::projections_of(ir, basis, &children),
        Err(_) => crate::compile::projections(basis, validated, &children),
    }
    .map_err(|e| format!("{alias}: {e}"))?;
    Ok(views
        .into_iter()
        .map(|(view, text)| {
            (
                repo.relative(&one.dir.join(view.committed_filename())),
                text,
            )
        })
        .collect())
}

fn render(corpus: &Corpus, model: &Model, asked: &[ProjectionRequest]) -> Vec<Projection> {
    let mut out = Vec::new();
    for p in asked {
        let subjects: Vec<String> = if p.kind == "warrant" && p.subject == "*" {
            corpus
                .entries()
                .map(|es| es.iter().map(|e| e.name().to_owned()).collect())
                .unwrap_or_default()
        } else {
            vec![p.subject.clone()]
        };
        for subject in subjects {
            let rendered = if p.kind == "warrant" {
                Some(warrant_views(corpus, &subject))
            } else {
                m6_render_seam(corpus, model, &p.kind, &subject)
            };
            match rendered {
                Some(Ok(views)) => {
                    for (path, text) in views {
                        out.push(Projection {
                            kind: p.kind.clone(),
                            subject: subject.clone(),
                            path: Some(path),
                            status: "rendered".to_owned(),
                            digest: Some(digest(text.as_bytes())),
                            utf8: Some(text),
                            reason: None,
                        });
                    }
                }
                Some(Err(why)) => out.push(unavailable(&p.kind, &subject, why)),
                None => out.push(unavailable(
                    &p.kind,
                    &subject,
                    format!("no render function for kind {} in this build", p.kind),
                )),
            }
        }
    }
    out
}

fn unavailable(kind: &str, subject: &str, why: String) -> Projection {
    Projection {
        kind: kind.to_owned(),
        subject: subject.to_owned(),
        path: None,
        status: "unavailable".to_owned(),
        digest: None,
        utf8: None,
        reason: Some(why),
    }
}

// ---------------------------------------------------------------------------
// The run
// ---------------------------------------------------------------------------

fn refused_response(r: &Refused, limits: Limits) -> Response {
    Response {
        protocol: PROTOCOL.to_owned(),
        version: VERSION,
        outcome: "refused".to_owned(),
        basis: None,
        model: None,
        model_digest: None,
        diagnostics: vec![],
        projections: vec![],
        observations: ObservationUse::default(),
        limits,
        refusal: Some(Refusal {
            rule: r.0.to_owned(),
            message: r.1.clone(),
        }),
    }
}

/// The compact JSON bytes the model digest is taken over.
#[must_use]
pub fn model_bytes(model: &Model) -> Vec<u8> {
    serde_json::to_vec(model).unwrap_or_default()
}

/// Answer one request. Pure: see the module documentation.
#[must_use]
pub fn answer(bytes: &[u8]) -> (u8, Response) {
    let checked = match check(bytes) {
        Ok(c) => c,
        Err(r) => return (1, refused_response(&r, LIMITS)),
    };
    let Checked {
        request,
        limits,
        files,
        withheld,
        basis_digest,
        ssh,
        git,
    } = checked;
    let supplied = (ssh.len(), git.len());
    let tree = Rc::new(
        vfs::Tree::new(Utf8PathBuf::from(VIRTUAL_ROOT), files, withheld)
            .with_observations(ssh, git),
    );
    let mut diagnostics: BTreeSet<HostDiagnostic> = BTreeSet::new();
    let built = vfs::hosted(Rc::clone(&tree), || -> Result<_, RepoError> {
        let repo = Repository::open(Utf8PathBuf::from(VIRTUAL_ROOT))?;
        crate::gate_cmd::source::remember_tree_reads();
        let corpus = Corpus::new(&repo);
        let model = model::build(&corpus)?;
        let projections = render(&corpus, &model, &request.options.projections);
        Ok((model, projections))
    });
    let consulted = tree.consulted();
    for path in &consulted.withheld_read {
        diagnostics.insert(HostDiagnostic {
            rule: "host.member-withheld".to_owned(),
            subject: path.clone(),
            message: "a reader needed the bytes of a member the basis holds by digest only"
                .to_owned(),
            severity: "error".to_owned(),
        });
    }
    for k in &consulted.ssh_missing {
        diagnostics.insert(HostDiagnostic {
            rule: "host.observation-missing".to_owned(),
            subject: format!("signature:{k}"),
            message: "a signature the readers would verify has no observation in the request, so it reads unchecked, as with no ssh-keygen".to_owned(),
            severity: "unknown".to_owned(),
        });
    }
    for a in &consulted.git_missing {
        diagnostics.insert(HostDiagnostic {
            rule: "host.observation-missing".to_owned(),
            subject: format!("git:{}", a.join(" ")),
            message: "a git read the readers would make has no observation in the request, so it fails, as with no git".to_owned(),
            severity: "unknown".to_owned(),
        });
    }
    let observations = ObservationUse {
        signatures_supplied: supplied.0,
        signatures_used: consulted.ssh_supplied.len(),
        signatures_missing: consulted.ssh_missing.len(),
        git_supplied: supplied.1,
        git_used: consulted.git_supplied.len(),
        git_missing: consulted.git_missing.len(),
        authenticated: false,
    };
    let echo = BasisEcho {
        id: request.basis.id.clone(),
        digest: basis_digest,
    };
    let (model, projections) = match built {
        Ok(x) => x,
        Err(e) => {
            diagnostics.insert(HostDiagnostic {
                rule: "host.not-compiled".to_owned(),
                subject: "basis".to_owned(),
                message: format!("the basis did not compile to a model: {e}"),
                severity: "error".to_owned(),
            });
            let response = Response {
                protocol: PROTOCOL.to_owned(),
                version: VERSION,
                outcome: "not_established".to_owned(),
                basis: Some(echo),
                model: None,
                model_digest: None,
                diagnostics: diagnostics.into_iter().collect(),
                projections: vec![],
                observations,
                limits,
                refusal: None,
            };
            return (2, response);
        }
    };
    compare(&model, &request, &mut diagnostics);
    for p in projections.iter().filter(|p| p.status != "rendered") {
        diagnostics.insert(HostDiagnostic {
            rule: "host.projection-unavailable".to_owned(),
            subject: format!("{}:{}", p.kind, p.subject),
            message: p.reason.clone().unwrap_or_default(),
            severity: "unknown".to_owned(),
        });
    }
    let established = diagnostics.is_empty();
    let response = Response {
        protocol: PROTOCOL.to_owned(),
        version: VERSION,
        outcome: if established {
            "compiled"
        } else {
            "not_established"
        }
        .to_owned(),
        basis: Some(echo),
        model_digest: Some(digest(&model_bytes(&model))),
        model: Some(model),
        diagnostics: diagnostics.into_iter().collect(),
        projections,
        observations,
        limits,
        refusal: None,
    };
    (if established { 0 } else { 2 }, response)
}

/// The request's Nodes and Relations against the compiled ones.
fn compare(model: &Model, request: &Request, out: &mut BTreeSet<HostDiagnostic>) {
    let compiled: BTreeMap<&str, Node> = model
        .records
        .iter()
        .map(|r| (r.id.as_str(), Node::from(r)))
        .collect();
    let mut held: BTreeMap<&str, &Node> = BTreeMap::new();
    let mut d = |rule: &str, subject: String, message: String| {
        out.insert(HostDiagnostic {
            rule: rule.to_owned(),
            subject,
            message,
            severity: "error".to_owned(),
        });
    };
    for n in &request.nodes {
        if held.insert(n.id.as_str(), n).is_some() {
            d(
                "host.node-duplicate",
                n.id.clone(),
                format!("Node {} is supplied more than once", n.id),
            );
        }
    }
    for (id, n) in &compiled {
        match held.get(id) {
            None => d(
                "host.node-missing",
                (*id).to_owned(),
                format!("the basis compiles record {id}, and the request holds no Node for it"),
            ),
            Some(h) if *h != n => d(
                "host.node-differs",
                (*id).to_owned(),
                format!(
                    "Node {id} is held as {} and compiles to {}",
                    serde_json::to_string(h).unwrap_or_default(),
                    serde_json::to_string(n).unwrap_or_default()
                ),
            ),
            Some(_) => {}
        }
    }
    for id in held.keys().filter(|id| !compiled.contains_key(*id)) {
        d(
            "host.node-unknown",
            (*id).to_owned(),
            format!("Node {id} is no record the basis compiles to"),
        );
    }
    let compiled_rel: BTreeSet<&model::Relation> = model.relations.iter().collect();
    let held_rel: BTreeSet<&model::Relation> = request.relations.iter().collect();
    let name = |r: &model::Relation| format!("{} {} {}", r.from, r.kind, r.to);
    for r in compiled_rel.difference(&held_rel) {
        d(
            "host.relation-missing",
            name(r),
            "the basis compiles this relation, and the request holds no Relation for it".to_owned(),
        );
    }
    for r in held_rel.difference(&compiled_rel) {
        d(
            "host.relation-unknown",
            name(r),
            "this Relation is no relation the basis compiles to".to_owned(),
        );
    }
    if held_rel.len() != request.relations.len() {
        d(
            "host.relation-duplicate",
            "relations".to_owned(),
            "a Relation is supplied more than once".to_owned(),
        );
    }
}

/// `war host`: one request on stdin, one response on stdout.
#[must_use]
pub fn run_stdin() -> u8 {
    let mut bytes = Vec::new();
    let read = std::io::stdin()
        .lock()
        .take(LIMITS.input_bytes + 1)
        .read_to_end(&mut bytes);
    let (mut code, response) = match read {
        Ok(_) => answer(&bytes),
        Err(e) => (
            1,
            refused_response(
                &Refused(
                    "host.malformed",
                    format!("the request could not be read: {e}"),
                ),
                LIMITS,
            ),
        ),
    };
    let limit = usize::try_from(response.limits.output_bytes).unwrap_or(usize::MAX);
    let mut data = match wire::encode(&crate::output::value(&response), limit.saturating_sub(1)) {
        Ok(d) => d,
        Err(_) => {
            code = 1;
            let r = Refused(
                "host.limit",
                format!(
                    "the response exceeds {} bytes (output_bytes)",
                    response.limits.output_bytes
                ),
            );
            serde_json::to_vec(&refused_response(&r, response.limits)).unwrap_or_default()
        }
    };
    data.push(b'\n');
    if std::io::stdout().write_all(&data).is_err() {
        return 1;
    }
    code
}

// ---------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------

/// Parse `kind:subject` (`warrant:OW-WAR-0001`, `warrant:*`).
pub fn parse_projection(s: &str) -> Result<ProjectionRequest, String> {
    let (kind, subject) = s.split_once(':').ok_or_else(|| {
        format!("--projection {s:?}: use KIND:SUBJECT (warrant:OW-WAR-0001, warrant:*)")
    })?;
    if !projection_kind_known(kind) || subject.is_empty() {
        return Err(format!(
            "--projection {s:?}: kind is one of {}",
            PROJECTION_KINDS.join(", ")
        ));
    }
    Ok(ProjectionRequest {
        kind: kind.to_owned(),
        subject: subject.to_owned(),
    })
}

/// The request that reproduces `root`'s model: built by running the
/// standalone build once, recording what it read and observed.
pub fn export(
    root: &Utf8PathBuf,
    projections: Vec<ProjectionRequest>,
) -> Result<Request, RepoError> {
    let recorder = Rc::new(RefCell::new(vfs::Recorder {
        root: root.clone(),
        ..vfs::Recorder::default()
    }));
    let model = vfs::recording(Rc::clone(&recorder), || -> Result<Model, RepoError> {
        let repo = Repository::open(root.clone())?;
        crate::gate_cmd::source::remember_tree_reads();
        let corpus = Corpus::new(&repo);
        let model = model::build(&corpus)?;
        let _ = render(&corpus, &model, &projections);
        Ok(model)
    })?;
    let rec = recorder.borrow();
    let mut members = Vec::new();
    let mut profiles = Vec::new();
    for path in &rec.files {
        let full = root.join(path);
        let bytes = std::fs::read(&full).map_err(|source| RepoError::Io {
            context: format!("could not read {full}"),
            source,
        })?;
        if let Some(name) = path
            .strip_prefix("profiles/")
            .and_then(|n| n.strip_suffix(".toml"))
            .filter(|n| !n.contains('/'))
            && let Ok(text) = String::from_utf8(bytes.clone())
        {
            profiles.push(Profile {
                name: name.to_owned(),
                digest: digest(&bytes),
                utf8: text,
            });
            continue;
        }
        let d = digest(&bytes);
        let (utf8, hex) = match String::from_utf8(bytes) {
            Ok(t) => (Some(t), None),
            Err(e) => (None, Some(to_hex(e.as_bytes()))),
        };
        members.push(Member {
            path: path.clone(),
            digest: d,
            utf8,
            hex,
            withheld: false,
        });
    }
    for path in &rec.seen {
        let full = root.join(path);
        let bytes = std::fs::read(&full).map_err(|source| RepoError::Io {
            context: format!("could not read {full}"),
            source,
        })?;
        members.push(Member {
            path: path.clone(),
            digest: digest(&bytes),
            utf8: None,
            hex: None,
            withheld: true,
        });
    }
    members.sort_by(|a, b| a.path.cmp(&b.path));
    let pairs: Vec<(String, String)> = members
        .iter()
        .map(|m| (m.path.clone(), m.digest.clone()))
        .collect();
    let bd = basis_digest(&pairs);
    let observations = Observations {
        signatures: rec
            .ssh
            .iter()
            .map(|(k, v)| SignatureObservation {
                key: k.clone(),
                verdict: if v.is_ok() { "verified" } else { "rejected" }.to_owned(),
                reason: v.clone().err(),
            })
            .collect(),
        git: rec
            .git
            .iter()
            .map(|(a, v)| GitObservation {
                args: a.clone(),
                stdout_hex: v.as_ref().ok().map(|b| to_hex(b)),
                error: v.clone().err(),
            })
            .collect(),
    };
    Ok(Request {
        protocol: PROTOCOL.to_owned(),
        version: VERSION,
        basis: Basis {
            id: format!("liminal-basis://{bd}"),
            digest: bd,
            members,
        },
        profiles,
        nodes: model.records.iter().map(Node::from).collect(),
        relations: model.relations.clone(),
        observations,
        options: Options {
            projections,
            limits: None,
        },
    })
}

/// `war host --export`: the request, one line of JSON on stdout.
pub fn run_export(repo: &Repository, projections: &[String]) -> Result<u8, RepoError> {
    let asked = projections
        .iter()
        .map(|p| parse_projection(p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(RepoError::Message)?;
    let request = export(&repo.root, asked)?;
    let mut data = serde_json::to_vec(&request)
        .map_err(|e| RepoError::Message(format!("could not encode the request: {e}")))?;
    data.push(b'\n');
    std::io::stdout()
        .write_all(&data)
        .map_err(|source| RepoError::Io {
            context: "could not write the request".to_owned(),
            source,
        })?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(path: &str, text: &str) -> Member {
        Member {
            path: path.to_owned(),
            digest: digest(text.as_bytes()),
            utf8: Some(text.to_owned()),
            hex: None,
            withheld: false,
        }
    }

    fn request(members: Vec<Member>) -> Request {
        let pairs: Vec<_> = members
            .iter()
            .map(|m| (m.path.clone(), m.digest.clone()))
            .collect();
        Request {
            protocol: PROTOCOL.to_owned(),
            version: VERSION,
            basis: Basis {
                id: "liminal-basis://test".to_owned(),
                digest: basis_digest(&pairs),
                members,
            },
            profiles: vec![],
            nodes: vec![],
            relations: vec![],
            observations: Observations::default(),
            options: Options::default(),
        }
    }

    fn refused_by(v: &Value) -> String {
        let (code, r) = answer(&serde_json::to_vec(v).unwrap());
        assert_eq!(code, 1, "{r:?}");
        assert!(r.model.is_none());
        r.refusal.unwrap().rule
    }

    #[test]
    fn refusals_are_named_before_any_work() {
        let ok = serde_json::to_value(request(vec![member("a.md", "x")])).unwrap();
        let mut v = ok.clone();
        v["version"] = 2.into();
        assert_eq!(refused_by(&v), "host.version");
        let mut v = ok.clone();
        v["protocol"] = "oh.war/liminal-v0".into();
        assert_eq!(refused_by(&v), "host.protocol");
        let mut v = ok.clone();
        v["basis"]["members"][0] =
            serde_json::json!({"path":"a.md","digest":"sha256:00","file":"/etc/passwd"});
        assert_eq!(refused_by(&v), "host.path-not-bytes");
        let mut v = ok.clone();
        v["basis"]["members"][0]["utf8"] = "y".into();
        assert_eq!(refused_by(&v), "host.member-digest");
        let mut v = ok.clone();
        v["basis"]["digest"] = "sha256:00".into();
        assert_eq!(refused_by(&v), "host.basis-digest");
        let mut v = ok.clone();
        v["options"] = serde_json::json!({"limits":{"members":0}});
        assert_eq!(refused_by(&v), "host.limit");
        let mut v = ok.clone();
        v["options"] = serde_json::json!({"limits":{"json_nodes":5}});
        assert_eq!(refused_by(&v), "host.limit");
        let mut v = ok.clone();
        v["extra"] = 1.into();
        assert_eq!(refused_by(&v), "host.malformed");
        let v = serde_json::to_value(request(vec![member("../a.md", "x")])).unwrap();
        assert_eq!(refused_by(&v), "host.member-path");
        let (code, r) = answer(b"[1,2");
        assert_eq!(
            (code, r.refusal.unwrap().rule.as_str()),
            (1, "host.malformed")
        );
    }

    #[test]
    fn a_basis_with_no_repository_does_not_compile() {
        let v = serde_json::to_vec(&request(vec![member("a.md", "x")])).unwrap();
        let (code, r) = answer(&v);
        assert_eq!(code, 2);
        assert_eq!(r.outcome, "not_established");
        assert_eq!(r.diagnostics[0].rule, "host.not-compiled");
    }
}
