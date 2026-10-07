// SPDX-License-Identifier: Apache-2.0
//! Signing, probed without signing anything (M9, decision 20).
//!
//! An agent in an adopting repository hit `war sign` failing with raw
//! `ssh-keygen` stderr, could not tell why, and concluded it could not work.
//! Signing is not work: a broken signing setup blocks the sign-off and
//! nothing else. This module says what is missing, in the words of the thing
//! that is missing.
//!
//! - [`probe`] is `war admin doctor`'s signing section. It reads PATH,
//!   `SSH_AUTH_SOCK`, `ssh-add -L` (public keys only), `roles.toml` and
//!   `allowed_signers`. It signs nothing and writes nothing.
//! - [`sign_failure`] turns a failed `ssh-keygen -Y sign` into the reason:
//!   no agent, a stale socket, no key loaded, the wrong key loaded.
//! - [`finish`] is applied to every `war sign` report: a refusal that names
//!   nobody eligible says which file is missing, and every `sign.*` refusal
//!   ends with [`BLOCKS_ONLY`].
//! - [`fix`] is `war admin doctor --fix-signing`: at a terminal only, it offers to
//!   repair each finding. It never signs. It writes `roles.toml` or
//!   `allowed_signers` only when the file does not exist yet, from answers
//!   typed at the terminal, after showing the exact bytes and asking — the
//!   write-once rule `war init` already follows (OW-ADR-0021). A file that
//!   exists is never edited: the wizard prints the lines for a person to add.

use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use serde_json::json;

use crate::diagnostic::{Diagnostic, Report, Severity};
use crate::repo::{RepoError, Repository};

/// The sentence every `sign.*` refusal ends with.
pub const BLOCKS_ONLY: &str = "This blocks only the sign-off, not your work.";

/// Where the two authority files live under the default binding.
pub const ROLES: &str = "docs/authority/roles.toml";
pub const ALLOWED: &str = "docs/authority/allowed_signers";

/// What the ssh agent behind `SSH_AUTH_SOCK` says, as `ssh-add -L` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Agent {
    /// `SSH_AUTH_SOCK` is not set: no agent is reachable from this process.
    NoSocket,
    /// It names a path that does not exist.
    SocketMissing(String),
    /// `ssh-add` could not be run at all.
    NoSshAdd(String),
    /// The agent did not answer (`ssh-add` exit 2).
    Unreachable { sock: String, why: String },
    /// The agent answered and holds no key (exit 1).
    NoKeys { sock: String },
    /// The agent holds these keys, one `<keytype> <base64> [comment]` each.
    Keys { sock: String, keys: Vec<String> },
    /// Anything else: what the agent holds is UNKNOWN.
    Unknown { sock: String, why: String },
}

impl Agent {
    /// Whether the agent is known to hold `pubkey` (`<keytype> <base64>`).
    #[must_use]
    pub fn holds(&self, pubkey: &str) -> Option<bool> {
        match self {
            Self::Keys { keys, .. } => Some(keys.iter().any(|k| same_key(k, pubkey))),
            Self::NoKeys { .. } | Self::NoSocket | Self::SocketMissing(_) => Some(false),
            Self::Unreachable { .. } | Self::NoSshAdd(_) | Self::Unknown { .. } => None,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::NoSocket => "no-socket",
            Self::SocketMissing(_) => "socket-missing",
            Self::NoSshAdd(_) => "no-ssh-add",
            Self::Unreachable { .. } => "unreachable",
            Self::NoKeys { .. } => "no-keys",
            Self::Keys { .. } => "keys",
            Self::Unknown { .. } => "unknown",
        }
    }
}

/// Read the agent: `SSH_AUTH_SOCK`, then `ssh-add -L`, which lists public
/// keys and signs nothing.
#[must_use]
pub fn agent() -> Agent {
    let sock = std::env::var("SSH_AUTH_SOCK")
        .ok()
        .filter(|s| !s.is_empty());
    let Some(sock) = sock else {
        return Agent::NoSocket;
    };
    if !Utf8Path::new(&sock).exists() {
        return Agent::SocketMissing(sock);
    }
    match std::process::Command::new("ssh-add")
        .arg("-L")
        .stdin(std::process::Stdio::null())
        .output()
    {
        Ok(out) => agent_from(
            &sock,
            out.status.code(),
            &String::from_utf8_lossy(&out.stdout),
            &String::from_utf8_lossy(&out.stderr),
        ),
        Err(e) => Agent::NoSshAdd(e.to_string()),
    }
}

/// `ssh-add -L`'s answer, read: exit 0 lists keys, 1 has none, 2 cannot
/// reach the agent (ssh-add(1)). Anything else is UNKNOWN, never a guess.
#[must_use]
pub fn agent_from(sock: &str, code: Option<i32>, stdout: &str, stderr: &str) -> Agent {
    let sock = sock.to_owned();
    let why = stderr.trim().to_owned();
    match code {
        Some(0) => {
            let keys: Vec<String> = stdout
                .lines()
                .map(str::trim)
                .filter(|l| {
                    l.starts_with("ssh-") || l.starts_with("sk-") || l.starts_with("ecdsa-")
                })
                .map(str::to_owned)
                .collect();
            if keys.is_empty() {
                Agent::NoKeys { sock }
            } else {
                Agent::Keys { sock, keys }
            }
        }
        Some(1) => Agent::NoKeys { sock },
        Some(2) => Agent::Unreachable { sock, why },
        other => Agent::Unknown {
            sock,
            why: format!(
                "ssh-add -L exited {}{}",
                other.map_or_else(|| "on a signal".to_owned(), |c| c.to_string()),
                if why.is_empty() {
                    String::new()
                } else {
                    format!(": {why}")
                }
            ),
        },
    }
}

/// `ssh-keygen` on PATH, found by looking, never by running it: run bare it
/// would start generating a key.
#[must_use]
pub fn keygen_on_path() -> Option<Utf8PathBuf> {
    let path = std::env::var("PATH").ok()?;
    path.split(':')
        .filter(|d| !d.is_empty())
        .map(|d| Utf8PathBuf::from(d).join("ssh-keygen"))
        .find(|p| p.is_file())
}

/// `<keytype> <base64>` of a key line, ignoring the comment.
fn key_id(line: &str) -> (String, String) {
    let mut f = line.split_whitespace();
    (
        f.next().unwrap_or_default().to_owned(),
        f.next().unwrap_or_default().to_owned(),
    )
}

fn same_key(a: &str, b: &str) -> bool {
    let (ta, ba) = key_id(a);
    let (tb, bb) = key_id(b);
    !ba.is_empty() && ta == tb && ba == bb
}

/// A key as a person recognises it: type and the last characters.
#[must_use]
pub fn short_key(line: &str) -> String {
    let (kt, b64) = key_id(line);
    let tail: String = b64
        .chars()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{kt} …{tail}")
}

/// One human the register lets sign off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signer {
    pub actor: String,
    pub principal: Option<String>,
}

/// What `roles.toml` says about who signs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Roles {
    Absent,
    Unreadable(String),
    /// Every human holding `authorizer` or `resolver`, and how many
    /// assignments the file holds in all.
    Read {
        signers: Vec<Signer>,
        total: usize,
    },
}

impl Roles {
    /// Read `roles.toml` through the same loader every authority check uses.
    #[must_use]
    pub fn read(repo: &Repository) -> Self {
        if !repo.root.join(ROLES).is_file() {
            return Self::Absent;
        }
        match repo.load_authority_register() {
            Err(e) => Self::Unreadable(e.to_string()),
            Ok(r) => Self::Read {
                total: r.assignments.len(),
                signers: r
                    .assignments
                    .iter()
                    .filter(|a| a.actor_kind.to_string() == "human")
                    .filter(|a| {
                        a.roles.iter().any(|role| {
                            matches!(
                                role,
                                openwarrant_core::authority::ActorRole::Authorizer
                                    | openwarrant_core::authority::ActorRole::Resolver
                            )
                        })
                    })
                    .map(|a| Signer {
                        actor: a.actor.clone(),
                        principal: a.ssh_principal.clone(),
                    })
                    .collect(),
            },
        }
    }
}

/// Everything the signing probes look at, gathered first so the judgement
/// over it is pure and testable without an agent or a key.
#[derive(Debug, Clone)]
pub struct Inputs {
    pub keygen: Option<Utf8PathBuf>,
    pub agent: Agent,
    /// `Some(description)` when `[authority] store` binds signers instead of
    /// the two files.
    pub store: Option<String>,
    pub roles: Roles,
    /// `allowed_signers`: `None` when absent, `Err` when unreadable.
    pub allowed: Option<Result<String, String>>,
}

impl Inputs {
    #[must_use]
    pub fn gather(repo: &Repository) -> Self {
        let store = match crate::authority_check::binding(repo) {
            crate::authority_check::Binding::Register => None,
            crate::authority_check::Binding::Store(s) => Some(s.describe()),
            crate::authority_check::Binding::Failed { rule, why } => Some(format!(
                "a configured store that cannot be used ({rule}: {why})"
            )),
        };
        let path = repo.root.join(ALLOWED);
        let allowed = path
            .is_file()
            .then(|| std::fs::read_to_string(&path).map_err(|e| e.to_string()));
        Self {
            keygen: keygen_on_path(),
            agent: agent(),
            store,
            roles: Roles::read(repo),
            allowed,
        }
    }
}

const FIX: &str =
    "`war admin doctor --fix-signing` at a terminal offers to write it from your answers";

fn line_for(principal: &str, key: &str) -> String {
    format!("{principal} namespaces=\"oh.war/response,oh.war/dsse\" {key}")
}

/// The signing findings. Warn for what is missing (signing is optional
/// until someone signs), Pass for what is in place, Unknown only for what
/// could not be observed.
#[must_use]
pub fn assess(i: &Inputs) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let warn = |rule: &str, file: Option<&str>, msg: String| {
        Diagnostic::new(Severity::Warn, rule, file.map(str::to_owned), msg)
    };
    let pass = |rule: &str, msg: String| Diagnostic::new(Severity::Pass, rule, None, msg);
    match &i.keygen {
        Some(p) => out.push(pass(
            "doctor.signing-keygen",
            format!("ssh-keygen is at {p}"),
        )),
        None => out.push(warn(
            "doctor.signing-keygen",
            None,
            "ssh-keygen is not on PATH; `war sign --ssh-sign` uses it (OpenSSH 8.2 or later). \
             Install OpenSSH, or sign at a terminal with plain `war sign`"
                .to_owned(),
        )),
    }
    let start = "start one with `eval \"$(ssh-agent -s)\"`, then load your key with \
                 `ssh-add -c <key>`";
    match &i.agent {
        Agent::NoSocket => out.push(warn(
            "doctor.signing-agent",
            None,
            format!(
                "SSH_AUTH_SOCK is not set, so no ssh-agent is reachable from this shell: {start}. \
                 Only `war sign --ssh-sign` needs it"
            ),
        )),
        Agent::SocketMissing(p) => out.push(warn(
            "doctor.signing-agent",
            None,
            format!(
                "SSH_AUTH_SOCK names {p}, which does not exist (the agent has exited, or this \
                 shell kept an old value): {start}"
            ),
        )),
        Agent::NoSshAdd(why) => out.push(Diagnostic::new(
            Severity::Unknown,
            "doctor.signing-agent",
            None,
            format!("ssh-add could not be run ({why}), so what the agent holds is UNKNOWN"),
        )),
        Agent::Unreachable { sock, why } => out.push(warn(
            "doctor.signing-agent",
            None,
            format!("the ssh-agent at {sock} did not answer ({why}): {start}"),
        )),
        Agent::NoKeys { sock } => out.push(warn(
            "doctor.signing-keys",
            None,
            format!(
                "no key loaded: the ssh-agent at {sock} holds none. Run `ssh-add -c <key>` with \
                 the key allowed_signers names for you"
            ),
        )),
        Agent::Keys { sock, keys } => out.push(pass(
            "doctor.signing-keys",
            format!("the ssh-agent at {sock} holds {} key(s)", keys.len()),
        )),
        Agent::Unknown { sock, why } => out.push(Diagnostic::new(
            Severity::Unknown,
            "doctor.signing-keys",
            None,
            format!("the ssh-agent at {sock}: {why}; what it holds is UNKNOWN"),
        )),
    }
    if let Some(store) = &i.store {
        out.push(pass(
            "doctor.signing-store",
            format!(
                "signers bind through {store}; roles.toml and allowed_signers are not what \
                 signing reads, so they are not probed here"
            ),
        ));
        return out;
    }
    let signers = match &i.roles {
        Roles::Absent => {
            out.push(warn(
                "doctor.signing-roles",
                Some(ROLES),
                format!(
                    "{ROLES} does not exist, so no one is registered to sign off. {FIX}, or \
                     a person writes it by hand, naming themselves with the authorizer and \
                     resolver roles (the format is OpenWarrant's \
                     docs/authority/roles.toml.example)"
                ),
            ));
            return out;
        }
        Roles::Unreadable(why) => {
            out.push(warn(
                "doctor.signing-roles",
                Some(ROLES),
                format!("{ROLES} could not be read, so no signer is known: {why}"),
            ));
            return out;
        }
        Roles::Read { signers, total } => {
            if signers.is_empty() {
                out.push(warn(
                    "doctor.signing-roles",
                    Some(ROLES),
                    format!(
                        "{ROLES} holds {total} assignment(s) and gives no human the authorizer \
                         or resolver role, so no one can sign off. A person adds an \
                         [[assignment]] with actor_kind = \"human\", roles = [\"authorizer\", \
                         \"resolver\"] and ssh_principal = \"<name, no spaces>\""
                    ),
                ));
                return out;
            }
            signers
        }
    };
    let mut principals = Vec::new();
    for s in signers {
        match &s.principal {
            Some(p) => {
                out.push(pass(
                    "doctor.signing-roles",
                    format!("{} may sign off, as ssh principal {p}", s.actor),
                ));
                principals.push(p.clone());
            }
            None => out.push(warn(
                "doctor.signing-roles",
                Some(ROLES),
                format!(
                    "{} may sign off but has no ssh_principal in {ROLES}, so `war sign \
                     --ssh-sign` cannot sign as them; a person adds `ssh_principal = \
                     \"<name, no spaces>\"` to that [[assignment]] (plain `war sign` at a \
                     terminal works without it)",
                    s.actor
                ),
            )),
        }
    }
    if principals.is_empty() {
        return out;
    }
    let text = match &i.allowed {
        None => {
            out.push(warn(
                "doctor.signing-allowed",
                Some(ALLOWED),
                format!(
                    "{ALLOWED} does not exist; `war sign --ssh-sign` verifies every signature \
                     against it. It needs one line per signer, e.g. `{}`; {FIX}",
                    line_for(&principals[0], "<key from ssh-add -L>")
                ),
            ));
            return out;
        }
        Some(Err(why)) => {
            out.push(warn(
                "doctor.signing-allowed",
                Some(ALLOWED),
                format!("{ALLOWED} could not be read: {why}"),
            ));
            return out;
        }
        Some(Ok(text)) => text,
    };
    for p in &principals {
        let key = match crate::sign::pubkey_for_principal(text, p) {
            Ok(k) => {
                out.push(pass(
                    "doctor.signing-allowed",
                    format!("{ALLOWED} names a {} key for {p}", short_key(&k)),
                ));
                k
            }
            Err(why) => {
                out.push(warn(
                    "doctor.signing-allowed",
                    Some(ALLOWED),
                    format!("{ALLOWED}: {why}"),
                ));
                continue;
            }
        };
        match i.agent.holds(&key) {
            Some(true) => out.push(pass(
                "doctor.signing-key-loaded",
                format!("the key {ALLOWED} names for {p} is loaded in the agent"),
            )),
            Some(false) => {
                if let Agent::Keys { keys, .. } = &i.agent {
                    out.push(warn(
                        "doctor.signing-key-loaded",
                        None,
                        format!(
                            "the agent holds {} key(s), none of them the {} key {ALLOWED} \
                             names for {p}: load it with `ssh-add -c <its private key file>`",
                            keys.len(),
                            short_key(&key)
                        ),
                    ));
                }
            }
            None => {}
        }
    }
    out
}

/// `war admin doctor`'s signing section: the findings, and what they rest on.
#[must_use]
pub fn probe(repo: &Repository) -> (Vec<Diagnostic>, serde_json::Value) {
    let inputs = Inputs::gather(repo);
    let findings = assess(&inputs);
    let keys = match &inputs.agent {
        Agent::Keys { keys, .. } => keys.len(),
        _ => 0,
    };
    let value = json!({
        "signed_anything": false,
        "ssh_keygen": inputs.keygen.as_deref().map(Utf8Path::as_str),
        "agent": inputs.agent.label(),
        "agent_keys": keys,
        "store": inputs.store,
        "roles": match &inputs.roles {
            Roles::Absent => "absent",
            Roles::Unreadable(_) => "unreadable",
            Roles::Read { .. } => "read",
        },
        "allowed_signers": match &inputs.allowed {
            None => "absent",
            Some(Err(_)) => "unreadable",
            Some(Ok(_)) => "read",
        },
    });
    (findings, value)
}

/// Why `ssh-keygen -Y sign` failed, in the terms of what is missing. The
/// agent is read again (its public keys only) to say so; raw stderr is kept
/// only where nothing better is known.
#[must_use]
pub fn sign_failure(principal: &str, pubkey: &str, status: &str, stderr: &str) -> String {
    sign_failure_with(principal, pubkey, &agent(), status, stderr)
}

#[must_use]
pub fn sign_failure_with(
    principal: &str,
    pubkey: &str,
    agent: &Agent,
    status: &str,
    stderr: &str,
) -> String {
    let start = "start one with `eval \"$(ssh-agent -s)\"` and load the key with \
                 `ssh-add -c <key>`";
    let key = short_key(pubkey);
    let said = stderr.trim();
    match agent {
        Agent::NoSocket => format!(
            "no ssh-agent is reachable: SSH_AUTH_SOCK is not set in this shell; {start} \
             ({key}, the key allowed_signers names for {principal})"
        ),
        Agent::SocketMissing(p) => {
            format!("SSH_AUTH_SOCK names {p}, which does not exist; {start}")
        }
        Agent::Unreachable { sock, why } => {
            format!("the ssh-agent at {sock} did not answer ({why}); {start}")
        }
        Agent::NoKeys { .. } => format!(
            "no key loaded: run `ssh-add -c <key>` with the {key} key allowed_signers names \
             for {principal}"
        ),
        Agent::Keys { keys, .. } if !keys.iter().any(|k| same_key(k, pubkey)) => format!(
            "the ssh-agent holds {} key(s), none of them the {key} key allowed_signers names \
             for {principal}: run `ssh-add -c <its private key file>`",
            keys.len()
        ),
        Agent::Keys { .. } => format!(
            "the {key} key for {principal} is loaded and ssh-keygen still refused ({status}): \
             the confirmation was declined, or the agent could not ask (its environment needs \
             SSH_ASKPASS and DISPLAY). ssh-keygen said: {said}"
        ),
        Agent::NoSshAdd(why) | Agent::Unknown { why, .. } => format!(
            "ssh-keygen refused ({status}); whether the agent holds the {key} key for \
             {principal} is UNKNOWN ({why}). ssh-keygen said: {said}"
        ),
    }
}

/// The start of `choose_actor`'s refusal when the register leaves nobody
/// eligible; [`finish`] replaces it with what is actually missing.
pub const NO_ELIGIBLE: &str = "no eligible signer";

/// Precise wording for "nobody eligible": the file that is missing, or the
/// role nobody holds, or the self-act rule, each with how to fix it.
fn no_signer(repo: &Repository, message: &str) -> String {
    if !matches!(
        crate::authority_check::binding(repo),
        crate::authority_check::Binding::Register
    ) {
        return message.to_owned();
    }
    match Roles::read(repo) {
        Roles::Absent => format!(
            "{ROLES} does not exist, so no one is registered to sign this. {FIX}, or a person \
             writes it by hand, naming themselves with the authorizer and resolver roles (the \
             format is OpenWarrant's docs/authority/roles.toml.example)"
        ),
        Roles::Unreadable(why) => format!("{ROLES} could not be read, so no one may sign: {why}"),
        Roles::Read { signers, .. } if signers.is_empty() => format!(
            "{ROLES} gives no human the authorizer or resolver role, so no one can sign this. A \
             person adds an [[assignment]] for themselves with actor_kind = \"human\" and those \
             roles"
        ),
        Roles::Read { .. } => message.to_owned(),
    }
}

/// Whether a `war sign` diagnostic is a refusal that blocks a sign-off.
fn is_sign_refusal(d: &Diagnostic) -> bool {
    d.rule.starts_with("sign.")
        && (matches!(d.severity, Severity::Error | Severity::Unknown)
            || matches!(d.rule.as_str(), "sign.refused" | "sign.needs-decision"))
}

/// Apply to every `war sign` report before it is shown: refine "nobody
/// eligible" into what is missing, and end each `sign.*` refusal with
/// [`BLOCKS_ONLY`]. Rules are unchanged, so nothing that reads them moves.
pub fn finish(repo: &Repository, report: &mut Report) {
    for d in &mut report.diagnostics {
        if d.rule == "sign.who" && d.message.starts_with(NO_ELIGIBLE) {
            d.message = no_signer(repo, &d.message);
        } else if d.rule == "sign.who" && d.message.ends_with("the register permits: nobody") {
            // `--as <actor>` with nobody eligible: say why the register is
            // empty when the file is what is missing.
            let why = no_signer(repo, NO_ELIGIBLE);
            if why != NO_ELIGIBLE {
                d.message = format!("{}: {why}", d.message);
            }
        }
        if is_sign_refusal(d) && !d.message.ends_with(BLOCKS_ONLY) {
            let sep = if d.message.ends_with('.') { " " } else { ". " };
            d.message = format!("{}{sep}{BLOCKS_ONLY}", d.message);
        }
    }
}

// ---------------------------------------------------------------------------
// `war doctor --fix-signing`: a terminal-only wizard that never signs.

/// What the wizard can do about one finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repair {
    /// Write a file that does not exist yet, after showing these exact bytes.
    WriteOnce { path: String, text: String },
    /// Something only a person can do: say exactly what.
    Tell(String),
}

/// The repairs for what [`assess`] found, given the answers the wizard
/// collected: who signs (`name`, `principal`) and which key (`key`, a line as
/// `ssh-add -L` prints it). Pure, so the never-edit and never-sign rules are
/// unit-tested.
#[must_use]
pub fn repairs(
    i: &Inputs,
    name: &str,
    principal: &str,
    key: Option<&str>,
    now: &str,
) -> Vec<Repair> {
    let mut out = Vec::new();
    if i.keygen.is_none() {
        out.push(Repair::Tell(
            "install OpenSSH (8.2 or later) so `ssh-keygen` is on PATH".to_owned(),
        ));
    }
    match &i.agent {
        Agent::NoSocket | Agent::SocketMissing(_) | Agent::Unreachable { .. } => {
            out.push(Repair::Tell(
                "start an agent in your shell, then run `war admin doctor --fix-signing` again from \
                 it:\n  eval \"$(ssh-agent -s)\""
                    .to_owned(),
            ));
        }
        _ => {}
    }
    if i.store.is_some() {
        out.push(Repair::Tell(
            "this repository binds signers through its authority store; the wizard leaves that \
             to the store's own commands (`war sign authority --help`)"
                .to_owned(),
        ));
        return out;
    }
    match &i.roles {
        Roles::Absent => out.push(Repair::WriteOnce {
            path: ROLES.to_owned(),
            text: render_roles(name, principal, now),
        }),
        Roles::Unreadable(why) => out.push(Repair::Tell(format!(
            "{ROLES} does not parse ({why}); fix it by hand. The wizard never edits it"
        ))),
        Roles::Read { signers, .. } => {
            let me = signers.iter().find(|s| s.actor == name);
            match me {
                None => out.push(Repair::Tell(format!(
                    "{ROLES} exists, so the wizard leaves it as it is; add yourself by hand:\n{}",
                    assignment(name, principal, now)
                ))),
                Some(s) if s.principal.is_none() => out.push(Repair::Tell(format!(
                    "{ROLES} exists, so the wizard leaves it as it is; add this line to {name}'s \
                     [[assignment]] by hand:\n  ssh_principal = {principal:?}"
                ))),
                Some(_) => {}
            }
        }
    }
    let Some(key) = key else {
        return out;
    };
    match &i.allowed {
        None => out.push(Repair::WriteOnce {
            path: ALLOWED.to_owned(),
            text: render_allowed(name, principal, key, now),
        }),
        Some(Err(why)) => out.push(Repair::Tell(format!(
            "{ALLOWED} could not be read ({why}); fix it by hand"
        ))),
        Some(Ok(text)) => {
            if crate::sign::pubkey_for_principal(text, principal).is_err() {
                out.push(Repair::Tell(format!(
                    "{ALLOWED} exists, so the wizard leaves it as it is; append this line by \
                     hand (and remove any other line for {principal}):\n  {}",
                    line_for(principal, key)
                )));
            }
        }
    }
    if i.agent.holds(key) == Some(false)
        && matches!(i.agent, Agent::Keys { .. } | Agent::NoKeys { .. })
    {
        out.push(Repair::Tell(format!(
            "load the {} key with confirmation on: `ssh-add -c <its private key file>`",
            short_key(key)
        )));
    }
    out
}

fn assignment(name: &str, principal: &str, now: &str) -> String {
    format!(
        "[[assignment]]\nactor = {name:?}\nactor_kind = \"human\"\nroles = [\"authorizer\", \
         \"resolver\", \"risk_acceptor\", \"judge\"]\nassigned_by = {name:?}\neffective_time = \
         {now:?}\nnote = \"Repository owner, from `war admin doctor --fix-signing`.\"\nssh_principal = \
         {principal:?}\n"
    )
}

fn header(name: &str, now: &str) -> String {
    format!(
        "# Written by `war doctor --fix-signing` on {now} from answers typed at a terminal by \
         {name:?}.\n#\n# THE RULE FOR THIS FILE: a tool writes it only from a human's answers at \
         a\n# terminal, once. No command edits it afterwards; every tool in this workspace\n# \
         reads it, and both `war init` and `war admin doctor --fix-signing` refuse to touch\n# one that \
         exists (OW-ADR-0021, Consequences).\n"
    )
}

/// `roles.toml` from the wizard: the human with every human role, the agent
/// as `performer` and nothing else. No answer reaches the agent entry.
#[must_use]
pub fn render_roles(name: &str, principal: &str, now: &str) -> String {
    format!(
        "# Actor role assignments (SAS §27.4).\n{}\n{}\n[[assignment]]\nactor = \"claude\"\n\
         actor_kind = \"agent\"\nroles = [\"performer\"]\nassigned_by = {name:?}\n\
         effective_time = {now:?}\nnote = \"Drafts and executes. Authorizes nothing, resolves \
         nothing.\"\n",
        header(name, now),
        assignment(name, principal, now)
    )
}

/// `allowed_signers` from the wizard: one line, both namespaces.
#[must_use]
pub fn render_allowed(name: &str, principal: &str, key: &str, now: &str) -> String {
    format!(
        "# Who may sign a response with an ssh key (`war sign --ssh-sign`).\n{}#\n# Format: \
         <principal> namespaces=\"oh.war/response,oh.war/dsse\" <keytype> <base64> [comment]\n\
         \n{}\n",
        header(name, now),
        line_for(principal, key.trim())
    )
}

fn ask(prompt: &str) -> Result<Option<String>, RepoError> {
    let mut out = std::io::stdout();
    out.write_all(prompt.as_bytes())
        .and_then(|()| out.flush())
        .map_err(|source| RepoError::Io {
            context: "could not write the prompt".to_owned(),
            source,
        })?;
    let mut line = String::new();
    let n = std::io::stdin()
        .read_line(&mut line)
        .map_err(|source| RepoError::Io {
            context: "could not read the answer".to_owned(),
            source,
        })?;
    Ok((n > 0).then(|| line.trim().to_owned()))
}

fn yes(a: &str) -> bool {
    matches!(a.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

/// `war admin doctor --fix-signing`. Refused without a terminal, before anything
/// is read; never signs; writes a file only when it does not exist, after
/// the exact bytes were shown and confirmed.
pub fn fix(repo: &Repository) -> Result<Report, RepoError> {
    let mut report = Report::default();
    if !crate::sign::at_a_terminal() {
        report.push(Diagnostic::new(
            Severity::Error,
            "doctor.fix-needs-tty",
            None,
            "`war admin doctor --fix-signing` asks questions, so it runs only at a terminal; \
             `war admin doctor` alone reports the same findings without asking"
                .to_owned(),
        ));
        return Ok(report);
    }
    let inputs = Inputs::gather(repo);
    for d in assess(&inputs) {
        println!("{d}");
    }
    let git_name = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    println!("\nWho signs off in this repository? (nothing is signed by this wizard)");
    let Some(name) = ask(&format!("  your name [{git_name}]: "))? else {
        return Ok(report);
    };
    let name = if name.is_empty() { git_name } else { name };
    if name.is_empty() || name.contains(['\n', '\r', '"']) {
        report.push(Diagnostic::new(
            Severity::Warn,
            "doctor.fix-stopped",
            None,
            "no usable name was given; nothing was written".to_owned(),
        ));
        return Ok(report);
    }
    let default_principal: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let Some(principal) = ask(&format!(
        "  ssh principal, no spaces [{default_principal}]: "
    ))?
    else {
        return Ok(report);
    };
    let principal = if principal.is_empty() {
        default_principal
    } else {
        principal
    };
    if principal.contains(char::is_whitespace) {
        report.push(Diagnostic::new(
            Severity::Warn,
            "doctor.fix-stopped",
            None,
            "a principal has no spaces (OpenSSH's rule); nothing was written".to_owned(),
        ));
        return Ok(report);
    }
    let keys = match &inputs.agent {
        Agent::Keys { keys, .. } => keys.clone(),
        _ => Vec::new(),
    };
    let key = if keys.is_empty() {
        let Some(k) = ask("  your public key, as `<keytype> <base64>` (empty to skip): ")? else {
            return Ok(report);
        };
        (!k.is_empty()).then_some(k)
    } else {
        for (n, k) in keys.iter().enumerate() {
            println!("  [{}] {}", n + 1, short_key(k));
        }
        let Some(pick) = ask("  which key signs (number, or paste a line): ")? else {
            return Ok(report);
        };
        match pick.parse::<usize>() {
            Ok(n) if n >= 1 && n <= keys.len() => Some(keys[n - 1].clone()),
            _ if pick.is_empty() => None,
            _ => Some(pick),
        }
    };
    if let Some(k) = &key {
        let (kt, b64) = key_id(k);
        if b64.is_empty()
            || !(kt.starts_with("ssh-") || kt.starts_with("sk-") || kt.starts_with("ecdsa-"))
        {
            report.push(Diagnostic::new(
                Severity::Warn,
                "doctor.fix-stopped",
                None,
                "that is not a key line as `ssh-add -L` prints it; nothing was written".to_owned(),
            ));
            return Ok(report);
        }
    }
    let now = crate::gate_cmd::receipt::now_rfc3339_public();
    for r in repairs(&inputs, &name, &principal, key.as_deref(), &now) {
        match r {
            Repair::Tell(what) => {
                println!("\n- {what}");
                report.push(Diagnostic::new(
                    Severity::Warn,
                    "doctor.fix-by-hand",
                    None,
                    what,
                ));
            }
            Repair::WriteOnce { path, text } => {
                let full = repo.root.join(&path);
                // Checked again at the write: the tree may have moved while
                // the questions were asked.
                if full.exists() {
                    report.push(Diagnostic::new(
                        Severity::Warn,
                        "doctor.fix-exists",
                        Some(path.clone()),
                        format!("{path} appeared while the wizard ran; it is left as it is"),
                    ));
                    continue;
                }
                println!("\n--- {path} (exact bytes) ---\n{text}--- end ---");
                let Some(ok) = ask(&format!("  write {path}? [y/N]: "))? else {
                    return Ok(report);
                };
                if !yes(&ok) {
                    report.push(Diagnostic::new(
                        Severity::Warn,
                        "doctor.fix-declined",
                        Some(path.clone()),
                        format!("{path} not written"),
                    ));
                    continue;
                }
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|source| RepoError::Io {
                        context: format!("could not create {parent}"),
                        source,
                    })?;
                }
                // `create_new`: never over a file, even one that appeared
                // between the check above and this write.
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&full)
                    .map_err(|source| RepoError::Io {
                        context: format!("could not create {full}"),
                        source,
                    })?;
                f.write_all(text.as_bytes())
                    .map_err(|source| RepoError::Io {
                        context: format!("could not write {full}"),
                        source,
                    })?;
                report.push(Diagnostic::new(
                    Severity::Pass,
                    "doctor.fix-wrote",
                    Some(path.clone()),
                    format!("wrote {path} from your answers; nothing was signed"),
                ));
            }
        }
    }
    report.note(
        "The wizard signed nothing. Run `war admin doctor` to see the signing setup now; a missing \
         piece blocks only the sign-off, not anyone's work.",
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIK0exampleexampleexample brian@host";

    fn inputs(agent: Agent, roles: Roles, allowed: Option<Result<String, String>>) -> Inputs {
        Inputs {
            keygen: Some(Utf8PathBuf::from("/usr/bin/ssh-keygen")),
            agent,
            store: None,
            roles,
            allowed,
        }
    }

    fn me() -> Roles {
        Roles::Read {
            signers: vec![Signer {
                actor: "Brian".to_owned(),
                principal: Some("brian".to_owned()),
            }],
            total: 2,
        }
    }

    fn rules(d: &[Diagnostic], severity: Severity) -> Vec<String> {
        d.iter()
            .filter(|x| x.severity == severity)
            .map(|x| x.rule.clone())
            .collect()
    }

    #[test]
    fn ssh_add_exit_codes_are_read_never_guessed() {
        assert_eq!(
            agent_from("/s", Some(0), &format!("{KEY}\n"), ""),
            Agent::Keys {
                sock: "/s".into(),
                keys: vec![KEY.into()]
            }
        );
        assert_eq!(
            agent_from("/s", Some(1), "The agent has no identities.\n", ""),
            Agent::NoKeys { sock: "/s".into() }
        );
        assert!(matches!(
            agent_from("/s", Some(2), "", "Could not open a connection"),
            Agent::Unreachable { .. }
        ));
        // Refusal side: an exit nobody documented is UNKNOWN, not "no keys".
        assert!(matches!(
            agent_from("/s", Some(7), "", "odd"),
            Agent::Unknown { .. }
        ));
        assert!(matches!(
            agent_from("/s", None, "", ""),
            Agent::Unknown { .. }
        ));
    }

    #[test]
    fn a_complete_setup_passes_and_each_missing_piece_is_named() {
        let allowed = Some(Ok(format!("brian {KEY}\n")));
        let ok = assess(&inputs(
            Agent::Keys {
                sock: "/s".into(),
                keys: vec![KEY.into()],
            },
            me(),
            allowed.clone(),
        ));
        assert!(rules(&ok, Severity::Warn).is_empty(), "{ok:?}");
        assert!(rules(&ok, Severity::Pass).contains(&"doctor.signing-key-loaded".to_owned()));

        // No socket: named, with the command that starts one.
        let d = assess(&inputs(Agent::NoSocket, me(), allowed.clone()));
        let agent = d.iter().find(|x| x.rule == "doctor.signing-agent").unwrap();
        assert_eq!(agent.severity, Severity::Warn);
        assert!(
            agent.message.contains("SSH_AUTH_SOCK is not set"),
            "{}",
            agent.message
        );
        // No key loaded.
        let d = assess(&inputs(
            Agent::NoKeys { sock: "/s".into() },
            me(),
            allowed.clone(),
        ));
        assert!(d.iter().any(|x| x.message.starts_with("no key loaded")));
        // The wrong key loaded.
        let other = "ssh-ed25519 AAAAotherotherotherother other@host";
        let d = assess(&inputs(
            Agent::Keys {
                sock: "/s".into(),
                keys: vec![other.into()],
            },
            me(),
            allowed.clone(),
        ));
        let m = d
            .iter()
            .find(|x| x.rule == "doctor.signing-key-loaded")
            .unwrap();
        assert_eq!(m.severity, Severity::Warn);
        assert!(m.message.contains("none of them"), "{}", m.message);
        // roles.toml missing: the file is named, and how to make it.
        let d = assess(&inputs(Agent::NoSocket, Roles::Absent, None));
        let r = d.iter().find(|x| x.rule == "doctor.signing-roles").unwrap();
        assert!(
            r.message
                .starts_with("docs/authority/roles.toml does not exist"),
            "{}",
            r.message
        );
        // allowed_signers missing, or without the principal's line.
        let d = assess(&inputs(Agent::NoSocket, me(), None));
        assert!(d.iter().any(|x| x.rule == "doctor.signing-allowed"
            && x.message.contains("does not exist")));
        let d = assess(&inputs(
            Agent::NoSocket,
            me(),
            Some(Ok("someone-else ".to_owned() + KEY)),
        ));
        assert!(
            d.iter().any(|x| x.rule == "doctor.signing-allowed"
                && x.message.contains("no key for principal"))
        );
        // Nothing here is an error: signing is optional until someone signs.
        assert!(rules(&d, Severity::Error).is_empty());
    }

    #[test]
    fn a_failed_signature_says_what_is_missing() {
        let m = sign_failure_with("brian", KEY, &Agent::NoKeys { sock: "/s".into() }, "1", "x");
        assert!(
            m.starts_with("no key loaded: run `ssh-add -c <key>`"),
            "{m}"
        );
        let m = sign_failure_with("brian", KEY, &Agent::NoSocket, "1", "x");
        assert!(m.contains("SSH_AUTH_SOCK is not set"), "{m}");
        let m = sign_failure_with(
            "brian",
            KEY,
            &Agent::Keys {
                sock: "/s".into(),
                keys: vec!["ssh-ed25519 AAAAother".into()],
            },
            "1",
            "x",
        );
        assert!(m.contains("none of them"), "{m}");
        // The key IS loaded: the raw stderr is kept, because nothing better
        // is known, and the likely cause is said.
        let m = sign_failure_with(
            "brian",
            KEY,
            &Agent::Keys {
                sock: "/s".into(),
                keys: vec![KEY.into()],
            },
            "exit status: 255",
            "agent refused operation",
        );
        assert!(
            m.contains("agent refused operation") && m.contains("declined"),
            "{m}"
        );
    }

    #[test]
    fn the_wizard_writes_only_absent_files_and_never_signs() {
        let none = inputs(
            Agent::Keys {
                sock: "/s".into(),
                keys: vec![KEY.into()],
            },
            Roles::Absent,
            None,
        );
        let r = repairs(&none, "Brian", "brian", Some(KEY), "2026-10-07T00:00:00Z");
        let writes: Vec<&str> = r
            .iter()
            .filter_map(|x| match x {
                Repair::WriteOnce { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(writes, [ROLES, ALLOWED]);
        // The roles it writes give the agent `performer` and nothing more.
        let Repair::WriteOnce { text, .. } = &r[0] else {
            panic!()
        };
        let v: toml::Value = toml::from_str(text).unwrap();
        let agent = v["assignment"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["actor_kind"].as_str() == Some("agent"))
            .unwrap();
        assert_eq!(agent["roles"].as_array().unwrap().len(), 1);
        // Refusal side: both files exist: nothing is written, the lines to
        // add are printed instead.
        let exist = inputs(
            Agent::Keys {
                sock: "/s".into(),
                keys: vec![KEY.into()],
            },
            Roles::Read {
                signers: vec![],
                total: 1,
            },
            Some(Ok("other ssh-ed25519 AAAAx\n".to_owned())),
        );
        let r = repairs(&exist, "Brian", "brian", Some(KEY), "2026-10-07T00:00:00Z");
        assert!(
            r.iter().all(|x| !matches!(x, Repair::WriteOnce { .. })),
            "{r:?}"
        );
        assert!(
            r.iter()
                .any(|x| matches!(x, Repair::Tell(t) if t.contains("ssh_principal = \"brian\"")))
        );
        assert!(
            r.iter()
                .any(|x| matches!(x, Repair::Tell(t) if t.contains("brian namespaces=")))
        );
        // And no repair is a command at all: the wizard writes an absent
        // file or says what a person adds. Signing is not among them.
        assert!(
            r.iter()
                .all(|x| !matches!(x, Repair::Tell(t) if t.contains("war sign")))
        );
    }

    /// With no roles.toml, both spellings of "nobody eligible" name the
    /// missing file; the refusal side is the repository above, whose
    /// register exists, where the message is left as it was.
    #[test]
    fn nobody_eligible_names_a_missing_roles_toml() {
        let root = Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .unwrap()
            .join(format!("war-no-roles-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        crate::init::run("NR", None, Some(root.clone())).unwrap();
        let repo = Repository::discover(Some(root.clone())).unwrap();
        let mut r = Report::default();
        r.push(Diagnostic::error(
            "sign.who",
            "x",
            format!("{NO_ELIGIBLE}: docs/authority/roles.toml gives the role to nobody"),
        ));
        r.push(Diagnostic::error(
            "sign.who",
            "x",
            "Ada is not eligible to sign this; the register permits: nobody",
        ));
        finish(&repo, &mut r);
        assert!(
            r.diagnostics[0]
                .message
                .starts_with("docs/authority/roles.toml does not exist"),
            "{}",
            r.diagnostics[0].message
        );
        assert!(
            r.diagnostics[1]
                .message
                .contains(": docs/authority/roles.toml does not exist"),
            "{}",
            r.diagnostics[1].message
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn every_sign_refusal_ends_with_the_sentence_and_others_do_not() {
        let mut r = Report::default();
        r.push(Diagnostic::error(
            "sign.no-tty",
            "war sign",
            "needs a terminal",
        ));
        r.push(Diagnostic::warn("sign.refused", "x", "ingest refused."));
        r.push(Diagnostic::pass("sign.would-record", "fine"));
        r.push(Diagnostic::error("authorize.self", "x", "self"));
        let root = Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let repo = Repository::discover(Some(root)).unwrap();
        finish(&repo, &mut r);
        assert!(r.diagnostics[0].message.ends_with(BLOCKS_ONLY));
        assert_eq!(
            r.diagnostics[1].message,
            format!("ingest refused. {BLOCKS_ONLY}")
        );
        assert_eq!(r.diagnostics[2].message, "fine");
        assert_eq!(r.diagnostics[3].message, "self");
        // Applied twice, said once.
        finish(&repo, &mut r);
        assert_eq!(r.diagnostics[0].message.matches(BLOCKS_ONLY).count(), 1);
    }
}
