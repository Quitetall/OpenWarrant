// SPDX-License-Identifier: Apache-2.0
//! Repo presets, roles and the roster (OW-WAR-0148 M14; docs/PRESETS.md).
//!
//! How much rigour a repository asks for is data in `openwarrant.toml`, in
//! three optional tables:
//!
//! ```toml
//! [preset]
//! name = "team"                # vibe, team or regulated
//! ticks = "claimed"            # the least every tick shows
//! signing = "release"          # never, release (one batch), merge
//! pr_requires_official = true  # `war check --pr` needs an official Warrant
//!
//! [roles]                      # the least formal Warrant each GitHub role makes alone
//! admin = "vibe"
//! maintain = "vibe"
//! write = "tested"
//! triage = "formal"
//! read = "formal"
//! none = "formal"
//!
//! [roles.roster]               # a roster principal's approvals count as this role
//! brian = "admin"
//!
//! [notify]                     # run when something waits on a person; off unless set
//! argv = ["notify-send", "OpenWarrant", "{message}"]
//! ```
//!
//! A repository with none of them behaves exactly as it did before presets
//! existed: `war init` without `--vibe`, `--team` or `--regulated` writes
//! none, and nothing here changes a command's output unless one is present.
//!
//! The roster is the existing authority register: `docs/authority/roles.toml`
//! names each human and their `ssh_principal`, and `allowed_signers` holds
//! the key. `[roles.roster]` only says which GitHub-style role a principal's
//! approval counts as; it grants no signing right and holds no key.

use std::collections::BTreeMap;
use std::fmt;

use camino::Utf8Path;
use openwarrant_core::ticks::Level as Tick;
use serde::{Deserialize, Serialize};

/// The three presets, least formal first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Preset {
    /// A person and their agents: no signatures, no PR gate, and `war done`
    /// claims an unclaimed item itself.
    Vibe,
    /// A team: the PR gate on, signatures batched at release.
    Team,
    /// A regulated repository: every tick observed, signatures at merge,
    /// and a test (or a formal Warrant) from everyone.
    Regulated,
}

impl Preset {
    pub const ALL: [Self; 3] = [Self::Vibe, Self::Team, Self::Regulated];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vibe => "vibe",
            Self::Team => "team",
            Self::Regulated => "regulated",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.as_str() == s.trim())
    }

    /// What follows from the preset.
    #[must_use]
    pub const fn defaults(self) -> Defaults {
        match self {
            Self::Vibe => Defaults {
                ticks: Tick::Claimed,
                signing: Signing::Never,
                pr_requires_official: false,
                roles: DEFAULT_ROLES,
            },
            Self::Team => Defaults {
                ticks: Tick::Claimed,
                signing: Signing::Release,
                pr_requires_official: true,
                roles: DEFAULT_ROLES,
            },
            Self::Regulated => Defaults {
                ticks: Tick::Observed,
                signing: Signing::Merge,
                pr_requires_official: true,
                roles: REGULATED_ROLES,
            },
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// When signatures are asked for (decision 7). Agents never wait on one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Signing {
    /// Never: officialness is the GitHub identity in CI.
    Never,
    /// Batched at release: one sitting, one signature (`war sign release`).
    Release,
    /// At merge: the PR gate wants a signed Warrant.
    Merge,
}

impl Signing {
    pub const ALL: [Self; 3] = [Self::Never, Self::Release, Self::Merge];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Never => "never",
            Self::Release => "release",
            Self::Merge => "merge",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|v| v.as_str() == s.trim())
    }
}

/// How formal a Warrant is, least first. A role's entry in `[roles]` is the
/// least formal kind it may create alone; anything at or above it is
/// allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A title is enough.
    Vibe,
    /// A title and a test (or a KPI that decides a tick).
    Tested,
    /// A directory Warrant that passes `war check`.
    Formal,
}

impl Kind {
    pub const ALL: [Self; 3] = [Self::Vibe, Self::Tested, Self::Formal];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Vibe => "vibe",
            Self::Tested => "tested",
            Self::Formal => "formal",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == s.trim())
    }

    /// What a Warrant of this kind is, in words.
    #[must_use]
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Vibe => "a title is enough",
            Self::Tested => "a title and a test",
            Self::Formal => "a directory Warrant that passes `war check`",
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A role, mirroring GitHub's repository roles, least first. `none` is
/// someone with no role in the repository: an outsider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    None,
    Read,
    Triage,
    Write,
    Maintain,
    Admin,
}

impl Role {
    pub const ALL: [Self; 6] = [
        Self::Admin,
        Self::Maintain,
        Self::Write,
        Self::Triage,
        Self::Read,
        Self::None,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Read => "read",
            Self::Triage => "triage",
            Self::Write => "write",
            Self::Maintain => "maintain",
            Self::Admin => "admin",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.as_str() == s.trim())
    }

    /// GitHub's answer for one user (`GET /repos/{o}/{r}/collaborators/{u}/
    /// permission`): `role_name` when it is one of the five built-in roles,
    /// else the legacy `permission` (`admin`, `write`, `read`, `none`). A
    /// custom role with no legacy permission GitHub names is `None`: not
    /// known, never guessed.
    #[must_use]
    pub fn from_github(role_name: Option<&str>, permission: Option<&str>) -> Option<Self> {
        role_name
            .and_then(Self::parse)
            .or_else(|| permission.and_then(Self::parse))
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The roles table decision 5 sets: admin and maintain vibe, write needs a
/// test, everyone else writes a formal Warrant.
pub const DEFAULT_ROLES: [(Role, Kind); 6] = [
    (Role::Admin, Kind::Vibe),
    (Role::Maintain, Kind::Vibe),
    (Role::Write, Kind::Tested),
    (Role::Triage, Kind::Formal),
    (Role::Read, Kind::Formal),
    (Role::None, Kind::Formal),
];

/// The regulated preset's table: nobody's plan is a title alone.
pub const REGULATED_ROLES: [(Role, Kind); 6] = [
    (Role::Admin, Kind::Tested),
    (Role::Maintain, Kind::Tested),
    (Role::Write, Kind::Formal),
    (Role::Triage, Kind::Formal),
    (Role::Read, Kind::Formal),
    (Role::None, Kind::Formal),
];

/// What a preset sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Defaults {
    pub ticks: Tick,
    pub signing: Signing,
    pub pr_requires_official: bool,
    pub roles: [(Role, Kind); 6],
}

/// `[notify]`: a command run when something waits on a person. Off unless
/// `argv` is set.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NotifyPolicy {
    /// The command, an argv (never a shell string). `{event}`, `{subject}`,
    /// `{message}` and `{command}` are filled in each element.
    #[serde(default)]
    pub argv: Vec<String>,
    /// How long it may run before it is killed (default 10 seconds).
    #[serde(default)]
    pub timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    name: String,
    #[serde(default)]
    ticks: Option<String>,
    #[serde(default)]
    signing: Option<String>,
    #[serde(default)]
    pr_requires_official: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RolesFile {
    #[serde(default)]
    admin: Option<String>,
    #[serde(default)]
    maintain: Option<String>,
    #[serde(default)]
    write: Option<String>,
    #[serde(default)]
    triage: Option<String>,
    #[serde(default)]
    read: Option<String>,
    #[serde(default)]
    none: Option<String>,
    #[serde(default)]
    roster: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct File {
    #[serde(default)]
    preset: Option<PresetFile>,
    #[serde(default)]
    roles: Option<RolesFile>,
    #[serde(default)]
    notify: Option<NotifyPolicy>,
}

/// The rigour this repository asks for, every default filled in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Policy {
    /// `[preset] name`; `None` when the table is absent.
    pub preset: Option<Preset>,
    /// The least every tick shows (`[preset] ticks`).
    pub ticks: Tick,
    pub signing: Signing,
    pub pr_requires_official: bool,
    /// Each role's least formal kind, every role present.
    pub roles: BTreeMap<Role, Kind>,
    /// Whether `[roles]` is in the file (otherwise `roles` is the preset's,
    /// or decision 5's).
    pub roles_declared: bool,
    /// `[roles.roster]`: a principal's approvals count as this role.
    pub roster: BTreeMap<String, Role>,
    /// `[notify]`, when it names a command.
    pub notify: Option<NotifyPolicy>,
}

impl Default for Policy {
    /// No preset and no `[roles]`: today's behaviour. Where a command needs
    /// a value anyway (the PR gate someone chose to run, `war sign
    /// release`), it is the team preset's.
    fn default() -> Self {
        let d = Preset::Team.defaults();
        Self {
            preset: None,
            ticks: Tick::Claimed,
            signing: d.signing,
            pr_requires_official: d.pr_requires_official,
            roles: d.roles.into_iter().collect(),
            roles_declared: false,
            roster: BTreeMap::new(),
            notify: None,
        }
    }
}

/// A refusal of the configuration, with the rule it is reported under.
pub const CONFIG_RULE: &str = "preset.config";

impl Policy {
    /// Read the three tables from `openwarrant.toml`'s text. `Err` names the
    /// key that is wrong; nothing is guessed.
    pub fn from_text(text: &str) -> Result<Self, String> {
        let file: File = toml::from_str(text).map_err(|e| {
            format!("openwarrant.toml: the [preset], [roles] or [notify] table: {e}")
        })?;
        let mut policy = Self::default();
        if let Some(p) = file.preset {
            let preset = Preset::parse(&p.name).ok_or_else(|| {
                format!(
                    "[preset] name = {:?} is not a preset; it is one of vibe, team, regulated",
                    p.name
                )
            })?;
            let d = preset.defaults();
            policy.preset = Some(preset);
            policy.ticks = match p.ticks.as_deref() {
                None => d.ticks,
                Some(s) => Tick::parse(s).ok_or_else(|| {
                    format!(
                        "[preset] ticks = {s:?} is not a level; it is one of claimed, observed, \
                         independent, signed"
                    )
                })?,
            };
            policy.signing = match p.signing.as_deref() {
                None => d.signing,
                Some(s) => Signing::parse(s).ok_or_else(|| {
                    format!(
                        "[preset] signing = {s:?} is not a timing; it is one of never, release, \
                         merge"
                    )
                })?,
            };
            policy.pr_requires_official = p.pr_requires_official.unwrap_or(d.pr_requires_official);
            policy.roles = d.roles.into_iter().collect();
        }
        if let Some(r) = file.roles {
            policy.roles_declared = true;
            for (role, given) in [
                (Role::Admin, &r.admin),
                (Role::Maintain, &r.maintain),
                (Role::Write, &r.write),
                (Role::Triage, &r.triage),
                (Role::Read, &r.read),
                (Role::None, &r.none),
            ] {
                if let Some(s) = given {
                    let kind = Kind::parse(s).ok_or_else(|| {
                        format!(
                            "[roles] {role} = {s:?} is not a kind; it is one of vibe, tested, \
                             formal"
                        )
                    })?;
                    policy.roles.insert(role, kind);
                }
            }
            for (principal, role) in r.roster {
                let parsed = Role::parse(&role).ok_or_else(|| {
                    format!(
                        "[roles.roster] {principal} = {role:?} is not a role; it is one of admin, \
                         maintain, write, triage, read, none"
                    )
                })?;
                policy.roster.insert(principal, parsed);
            }
        }
        if let Some(n) = file.notify {
            if n.argv.first().is_some_and(|a| !a.trim().is_empty()) {
                policy.notify = Some(n);
            } else if !n.argv.is_empty() {
                return Err("[notify] argv starts with an empty program name".to_owned());
            }
        }
        Ok(policy)
    }

    /// Read `openwarrant.toml` under `root`.
    pub fn read(root: &Utf8Path) -> Result<Self, String> {
        let path = root.join(crate::init::CONFIG_FILE);
        let text =
            crate::vfs::read_to_string(&path).map_err(|e| format!("could not read {path}: {e}"))?;
        Self::from_text(&text)
    }

    /// The policy, or the default when the file cannot be read or is wrong.
    /// For the reads that must never fail a command over it; `war check`
    /// reports the fault.
    #[must_use]
    pub fn read_or_default(root: &Utf8Path) -> Self {
        Self::read(root).unwrap_or_default()
    }

    /// Whether this repository asked for any of it: a `[preset]` or a
    /// `[roles]` table. Officialness is shown, and new human acts listed,
    /// only then.
    #[must_use]
    pub fn configured(&self) -> bool {
        self.preset.is_some() || self.roles_declared
    }

    /// The least formal kind `role` creates alone.
    #[must_use]
    pub fn least_kind(&self, role: Role) -> Kind {
        self.roles.get(&role).copied().unwrap_or(Kind::Formal)
    }

    /// Whether `role` may create a Warrant of `kind` alone.
    #[must_use]
    pub fn allows(&self, role: Role, kind: Kind) -> bool {
        kind >= self.least_kind(role)
    }

    /// The roles that may create (or approve) `kind`, most trusted first,
    /// for a message: "admin or maintain".
    #[must_use]
    pub fn roles_allowing(&self, kind: Kind) -> Vec<Role> {
        Role::ALL
            .into_iter()
            .filter(|r| self.allows(*r, kind))
            .collect()
    }

    /// `roles_allowing`, in words.
    #[must_use]
    pub fn roles_allowing_text(&self, kind: Kind) -> String {
        let roles: Vec<&str> = self
            .roles_allowing(kind)
            .into_iter()
            .map(Role::as_str)
            .collect();
        match roles.as_slice() {
            [] => "no role".to_owned(),
            [one] => (*one).to_owned(),
            many => format!(
                "{} or {}",
                many[..many.len() - 1].join(", "),
                many[many.len() - 1]
            ),
        }
    }

    /// The preset's name, or `none`.
    #[must_use]
    pub fn preset_name(&self) -> &'static str {
        self.preset.map_or("none", Preset::as_str)
    }

    /// The policy as `war admin preset` shows it.
    #[must_use]
    pub fn render(&self) -> String {
        let mut s = format!(
            "preset {}{}\n  ticks: {} at least\n  signing: {}\n  PR gate: {}\n  roles (the least \
             formal Warrant each creates alone{}):\n",
            self.preset_name(),
            if self.preset.is_none() {
                " (no [preset] table: behaviour as before presets; the values below are what \
                 `war check --pr` and `war sign release` use)"
            } else {
                ""
            },
            self.ticks.as_str(),
            self.signing.as_str(),
            if self.pr_requires_official {
                "an official Warrant is required"
            } else {
                "reported, never required"
            },
            if self.roles_declared {
                ", from [roles]"
            } else {
                ", the preset's"
            }
        );
        for role in Role::ALL {
            let kind = self.least_kind(role);
            s.push_str(&format!(
                "    {:<9} {:<7} ({})\n",
                role.as_str(),
                kind.as_str(),
                kind.describe()
            ));
        }
        if self.roster.is_empty() {
            s.push_str("  roster: no principal is mapped to a role ([roles.roster])\n");
        } else {
            s.push_str("  roster ([roles.roster]):\n");
            for (p, r) in &self.roster {
                s.push_str(&format!("    {p:<20} {r}\n"));
            }
        }
        s.push_str(&format!(
            "  notify: {}",
            self.notify
                .as_ref()
                .map_or_else(|| "off".to_owned(), |n| format!("{:?}", n.argv))
        ));
        s
    }
}

// ---- writing a preset into openwarrant.toml ------------------------------------

/// The `[preset]` table a preset writes, comments inside it so that
/// switching presets replaces them with the table.
#[must_use]
pub fn preset_table(preset: Preset) -> String {
    let d = preset.defaults();
    format!(
        "[preset]\n\
         # How much rigour this repository asks for (docs/PRESETS.md).\n\
         # `war admin preset <vibe|team|regulated|none>` switches it.\n\
         name = \"{name}\"\n\
         # The least every tick shows: claimed, observed, independent or signed.\n\
         ticks = \"{ticks}\"\n\
         # When signatures are asked for: never, release (one batch per release),\n\
         # or merge. Agents never wait on one.\n\
         signing = \"{signing}\"\n\
         # Whether `war check --pr` needs the PR to cite an official Warrant.\n\
         pr_requires_official = {pr}\n",
        name = preset.as_str(),
        ticks = d.ticks.as_str(),
        signing = d.signing.as_str(),
        pr = d.pr_requires_official,
    )
}

/// The `[roles]` table a preset writes.
#[must_use]
pub fn roles_table(preset: Preset) -> String {
    let mut s = String::from(
        "[roles]\n\
         # The least formal Warrant each GitHub role may create alone: vibe (a\n\
         # title is enough), tested (a title and a test), formal (a directory\n\
         # Warrant that passes `war check`). A less formal one is official once\n\
         # someone allowed approves it: an approving review on the PR, or\n\
         # `war sign approve <id> --ssh-sign` by a roster key. A roster\n\
         # principal (docs/authority/roles.toml `ssh_principal`) is given a\n\
         # role in [roles.roster]: `<principal> = \"maintain\"`.\n",
    );
    for (role, kind) in preset.defaults().roles {
        s.push_str(&format!("{} = \"{}\"\n", role.as_str(), kind.as_str()));
    }
    s
}

/// The header of a TOML table line: `[preset]` gives `preset`; an array of
/// tables (`[[x]]`) gives `[x]` so it never equals a plain table's name.
fn header(line: &str) -> Option<String> {
    let t = line.trim();
    if let Some(inner) = t.strip_prefix("[[") {
        let end = inner.find("]]")?;
        return Some(format!("[{}]", inner[..end].trim()));
    }
    let inner = t.strip_prefix('[')?;
    let end = inner.find(']')?;
    Some(inner[..end].trim().to_owned())
}

/// Remove every table `keep_out` matches, header through its last line of
/// content. Comment and blank lines just before the next table are left: a
/// person's comment above `[tickets]` belongs to `[tickets]`.
fn remove_tables(text: &str, matches: impl Fn(&str) -> bool) -> (String, bool) {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let headers: Vec<(usize, String)> = lines
        .iter()
        .enumerate()
        .filter_map(|(i, l)| header(l).map(|h| (i, h)))
        .collect();
    let mut drop = vec![false; lines.len()];
    let mut removed = false;
    for (n, (start, name)) in headers.iter().enumerate() {
        if !matches(name) {
            continue;
        }
        removed = true;
        let next = headers.get(n + 1).map_or(lines.len(), |(i, _)| *i);
        let mut end = next;
        if next < lines.len() {
            while end > start + 1 {
                let l = lines[end - 1].trim();
                if l.is_empty() || l.starts_with('#') {
                    end -= 1;
                } else {
                    break;
                }
            }
        }
        for d in drop.iter_mut().take(end).skip(*start) {
            *d = true;
        }
    }
    let kept: String = lines
        .iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(l, _)| *l)
        .collect();
    (kept, removed)
}

/// Append `block` after `text`, one blank line between.
fn append(text: &str, block: &str) -> String {
    let trimmed = text.trim_end_matches('\n');
    if trimmed.is_empty() {
        return block.to_owned();
    }
    format!("{trimmed}\n\n{block}")
}

/// What `war admin preset` did to the file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Switched {
    pub from: Option<Preset>,
    pub to: Option<Preset>,
    /// `written`, `kept` (a `[roles]` table was there and not reset), or
    /// `none` (no table, none written).
    pub roles: &'static str,
}

/// `openwarrant.toml` with `[preset]` set to `to` (or removed, `None`).
/// `[roles]` is written from the preset when absent, or when `reset_roles`;
/// otherwise a person's table is kept. The result is parsed again before it
/// is returned, so a file this would break is refused, not written.
pub fn switch(
    text: &str,
    to: Option<Preset>,
    reset_roles: bool,
) -> Result<(String, Switched), String> {
    let before = Policy::from_text(text)?;
    let (mut out, _) = remove_tables(text, |h| h == "preset");
    let roles = match to {
        None => {
            if before.roles_declared {
                "kept"
            } else {
                "none"
            }
        }
        Some(p) => {
            out = append(&out, &preset_table(p));
            if before.roles_declared && !reset_roles {
                "kept"
            } else {
                if before.roles_declared {
                    out = remove_tables(&out, |h| h == "roles").0;
                }
                out = append(&out, &roles_table(p));
                "written"
            }
        }
    };
    let after = Policy::from_text(&out)
        .map_err(|e| format!("the rewritten file would not read ({e}); nothing was written"))?;
    if after.preset != to {
        return Err(format!(
            "the rewritten file reads as preset {} rather than {}; nothing was written (a \
             hand-written `preset` key outside a [preset] table?)",
            after.preset_name(),
            to.map_or("none", Preset::as_str)
        ));
    }
    Ok((
        out,
        Switched {
            from: before.preset,
            to,
            roles,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str =
        "schema = \"oh.war/config/v1\"\n\n[project]\nname = \"x\"\nnamespace = \"X\"\n";

    #[test]
    fn no_tables_is_todays_behaviour() {
        let p = Policy::from_text(BASE).unwrap();
        assert!(!p.configured());
        assert_eq!(p.preset, None);
        assert_eq!(p.ticks, Tick::Claimed);
        assert!(p.notify.is_none());
        assert_eq!(p.least_kind(Role::Admin), Kind::Vibe);
        assert_eq!(p.least_kind(Role::Write), Kind::Tested);
        assert_eq!(p.least_kind(Role::None), Kind::Formal);
    }

    #[test]
    fn each_preset_writes_its_defaults_and_reads_back() {
        for preset in Preset::ALL {
            let (text, s) = switch(BASE, Some(preset), false).unwrap();
            assert_eq!(s.roles, "written");
            let p = Policy::from_text(&text).unwrap();
            let d = preset.defaults();
            assert_eq!(p.preset, Some(preset));
            assert_eq!(p.ticks, d.ticks);
            assert_eq!(p.signing, d.signing);
            assert_eq!(p.pr_requires_official, d.pr_requires_official);
            assert!(p.roles_declared);
            for (r, k) in d.roles {
                assert_eq!(p.least_kind(r), k, "{preset} {r}");
            }
        }
    }

    #[test]
    fn switching_replaces_the_preset_and_keeps_a_persons_roles() {
        let (team, _) = switch(BASE, Some(Preset::Team), false).unwrap();
        let custom = team.replace("write = \"tested\"", "write = \"vibe\"");
        let custom = format!("{custom}\n# my tickets\n[tickets]\ndir = \"t\"\n");
        let (reg, s) = switch(&custom, Some(Preset::Regulated), false).unwrap();
        assert_eq!(s.roles, "kept");
        assert_eq!(s.from, Some(Preset::Team));
        let p = Policy::from_text(&reg).unwrap();
        assert_eq!(p.preset, Some(Preset::Regulated));
        assert_eq!(p.least_kind(Role::Write), Kind::Vibe, "kept");
        assert_eq!(reg.matches("[preset]").count(), 1);
        assert!(reg.contains("# my tickets\n[tickets]"), "{reg}");
        let (reset, s) = switch(&reg, Some(Preset::Regulated), true).unwrap();
        assert_eq!(s.roles, "written");
        assert_eq!(
            Policy::from_text(&reset).unwrap().least_kind(Role::Write),
            Kind::Formal
        );
        let (none, s) = switch(&reset, None, false).unwrap();
        assert_eq!(s.to, None);
        let p = Policy::from_text(&none).unwrap();
        assert_eq!(p.preset, None);
        assert!(p.roles_declared, "roles stay");
        assert!(none.contains("[tickets]\ndir = \"t\""));
    }

    #[test]
    fn a_wrong_value_is_refused_by_key() {
        for (bad, key) in [
            ("[preset]\nname = \"loose\"\n", "[preset] name"),
            (
                "[preset]\nname = \"team\"\nticks = \"seen\"\n",
                "[preset] ticks",
            ),
            (
                "[preset]\nname = \"team\"\nsigning = \"sometimes\"\n",
                "[preset] signing",
            ),
            ("[roles]\nwrite = \"casual\"\n", "[roles] write"),
            (
                "[roles.roster]\nbrian = \"owner\"\n",
                "[roles.roster] brian",
            ),
            ("[roles]\nwriter = \"vibe\"\n", "unknown field"),
            ("[notify]\nargv = [\"\"]\n", "[notify] argv"),
        ] {
            let err = Policy::from_text(&format!("{BASE}{bad}")).unwrap_err();
            assert!(err.contains(key), "{bad:?}: {err}");
        }
    }

    #[test]
    fn github_roles_read_role_name_then_permission() {
        assert_eq!(
            Role::from_github(Some("maintain"), Some("write")),
            Some(Role::Maintain)
        );
        assert_eq!(
            Role::from_github(Some("custom-reviewer"), Some("read")),
            Some(Role::Read)
        );
        assert_eq!(Role::from_github(None, Some("none")), Some(Role::None));
        assert_eq!(Role::from_github(Some("custom"), None), None);
    }

    #[test]
    fn allows_reads_the_least_formal_kind() {
        let p = Policy::default();
        assert!(p.allows(Role::Admin, Kind::Vibe));
        assert!(!p.allows(Role::Write, Kind::Vibe));
        assert!(p.allows(Role::Write, Kind::Tested));
        assert!(!p.allows(Role::None, Kind::Tested));
        assert!(p.allows(Role::None, Kind::Formal));
        assert_eq!(p.roles_allowing_text(Kind::Vibe), "admin or maintain");
    }
}
