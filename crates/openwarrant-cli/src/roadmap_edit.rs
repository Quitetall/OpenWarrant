// SPDX-License-Identifier: Apache-2.0
//! Editing the roadmap by keystroke (OW-WAR-0114 M4, OW-ADR-0023).
//!
//! The owner's condition on roadmap signatures was that they cost almost
//! nothing: "if a human wants it, they don't need to trudge through
//! documentation hell to do so." So a human never opens the phases atom.
//! `war roadmap edit` shows the phases, takes one-letter commands, and ends
//! in exactly one act: the atom is written, the revision proposed, and one
//! `war sign roadmap --ssh-sign` child raises the key's dialog.
//!
//! [`Editor`] is the pure part — no I/O in an edit, so it is driven by canned
//! answers in its tests. [`run`] is the line front end, behind the same
//! terminal gate `war sign` uses. The app's Roadmap pane runs this command
//! in the terminal it hands over; it is one editor with two doors.
//!
//! What the editor refuses, by name: a duplicate id, a dependency on a phase
//! that does not exist, a cycle, an undeclared tier, and removing a phase
//! that Warrants still name (they would all read `roadmap.unknown-phase`;
//! move them first).

use std::collections::BTreeMap;

use camino::Utf8Path;
use openwarrant_core::roadmap::{Phase, Phases, parse_phases};

use crate::repo::{RepoError, Repository};

/// One edit a human asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Add {
        number: u8,
        title: String,
        exit: String,
    },
    Rename {
        id: String,
        title: String,
    },
    SetExit {
        id: String,
        exit: String,
    },
    SetOutcome {
        id: String,
        outcome: String,
    },
    SetPriority {
        id: String,
        tier: Option<String>,
    },
    SetDependsOn {
        id: String,
        depends_on: Vec<String>,
    },
    Remove {
        id: String,
    },
}

/// The phases being edited, the atom's header kept verbatim, and how many
/// Warrants name each phase (so a phase with members is not removed).
#[derive(Debug, Clone)]
pub struct Editor {
    prefix: String,
    header: String,
    pub phases: Phases,
    members: BTreeMap<String, usize>,
    pub dirty: bool,
}

impl Editor {
    /// From the atom's text. `members` is phase id → Warrant count.
    pub fn new(
        source: &str,
        prefix: &str,
        members: BTreeMap<String, usize>,
    ) -> Result<Self, String> {
        let phases = parse_phases(source, prefix).map_err(|e| e.to_string())?;
        // Everything before the first `tiers:` / `phases:` key: the schema
        // line and the atom's comment, which a human wrote and a rewrite must
        // not lose.
        let cut = source
            .find("\ntiers:")
            .or_else(|| source.find("\nphases:"))
            .map_or(source.len(), |i| i + 1);
        Ok(Self {
            prefix: prefix.to_owned(),
            header: source[..cut].to_owned(),
            phases,
            members,
            dirty: false,
        })
    }

    fn index(&self, id: &str) -> Result<usize, String> {
        self.phases
            .phases
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| format!("{id} is not a phase"))
    }

    /// Apply one edit, then re-validate the whole atom through the parser
    /// `war check` uses. A refused edit leaves the editor as it was.
    pub fn apply(&mut self, edit: Edit) -> Result<(), String> {
        let text: Vec<&str> = match &edit {
            Edit::Add { title, exit, .. } => vec![title, exit],
            Edit::Rename { title, .. } => vec![title],
            Edit::SetExit { exit, .. } => vec![exit],
            Edit::SetOutcome { outcome, .. } => vec![outcome],
            Edit::SetPriority { tier, .. } => tier.iter().map(String::as_str).collect(),
            Edit::SetDependsOn { .. } | Edit::Remove { .. } => vec![],
        };
        if let Some(bad) = text.iter().find(|s| s.contains(['"', '\n', '\r'])) {
            return Err(format!(
                "{bad:?} carries a double quote or a line break, which the phases atom cannot hold (OW-ADR-0003: no escapes); use single quotes"
            ));
        }
        let before = self.phases.clone();
        match edit {
            Edit::Add {
                number,
                title,
                exit,
            } => {
                let id = format!("{}-PHASE-{number}", self.prefix);
                if self.phases.get(&id).is_some() {
                    return Err(format!("{id} already exists"));
                }
                self.phases.phases.push(Phase {
                    id,
                    number,
                    title,
                    outcome: String::new(),
                    exit,
                    depends_on: vec![],
                    priority: None,
                    open: vec![],
                });
                self.phases.phases.sort_by_key(|p| p.number);
            }
            Edit::Rename { id, title } => {
                let i = self.index(&id)?;
                self.phases.phases[i].title = title;
            }
            Edit::SetExit { id, exit } => {
                let i = self.index(&id)?;
                self.phases.phases[i].exit = exit;
            }
            Edit::SetOutcome { id, outcome } => {
                let i = self.index(&id)?;
                self.phases.phases[i].outcome = outcome;
            }
            Edit::SetPriority { id, tier } => {
                let i = self.index(&id)?;
                self.phases.phases[i].priority = tier;
            }
            Edit::SetDependsOn { id, depends_on } => {
                let i = self.index(&id)?;
                self.phases.phases[i].depends_on = depends_on;
            }
            Edit::Remove { id } => {
                let i = self.index(&id)?;
                if let Some(n) = self.members.get(&id).filter(|n| **n > 0) {
                    return Err(format!(
                        "{id} is named by {n} Warrant(s); move them first — removing it would make each read roadmap.unknown-phase"
                    ));
                }
                if let Some(d) = self
                    .phases
                    .phases
                    .iter()
                    .find(|p| p.depends_on.contains(&id))
                {
                    return Err(format!("{} depends on {id}; change that first", d.id));
                }
                self.phases.phases.remove(i);
            }
        }
        // Round-trip through the real parser: the editor cannot write an
        // atom `war check` would refuse.
        if let Err(e) = parse_phases(&self.render(), &self.prefix) {
            self.phases = before;
            return Err(e.to_string());
        }
        self.dirty = true;
        Ok(())
    }

    /// The atom, in the restricted structured grammar (OW-ADR-0003): every
    /// value a quoted scalar or a flow list of them.
    #[must_use]
    pub fn render(&self) -> String {
        // The grammar has no escapes (OW-ADR-0003): `apply` refuses a value
        // carrying a double quote or a line break, so none reaches here.
        let q = |s: &str| format!("\"{s}\"");
        let list = |v: &[String]| {
            format!(
                "[{}]",
                v.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
            )
        };
        let mut out = self.header.clone();
        if !self.phases.tiers.is_empty() {
            out.push_str("tiers:\n");
            for t in &self.phases.tiers {
                out.push_str(&format!(
                    "  - id: {}\n    title: {}\n",
                    q(&t.id),
                    q(&t.title)
                ));
            }
            out.push('\n');
        }
        out.push_str("phases:\n");
        for (i, p) in self.phases.phases.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&format!(
                "  - id: {}\n    title: {}\n",
                q(&p.id),
                q(&p.title)
            ));
            if !p.outcome.is_empty() {
                out.push_str(&format!("    outcome: {}\n", q(&p.outcome)));
            }
            out.push_str(&format!("    exit: {}\n", q(&p.exit)));
            out.push_str(&format!("    depends_on: {}\n", list(&p.depends_on)));
            if let Some(t) = &p.priority {
                out.push_str(&format!("    priority: {}\n", q(t)));
            }
            out.push_str(&format!("    open: {}\n", list(&p.open)));
        }
        out
    }

    /// The numbered screen the line front end shows.
    #[must_use]
    pub fn screen(&self) -> String {
        let mut s = String::new();
        for (i, p) in self.phases.phases.iter().enumerate() {
            s.push_str(&format!(
                "  [{:>2}] {:<13} {}{}  ({} Warrant(s))\n        exit: {}\n",
                i + 1,
                p.id,
                p.title,
                p.priority
                    .as_ref()
                    .map_or(String::new(), |t| format!("  [tier {t}]")),
                self.members.get(&p.id).copied().unwrap_or(0),
                p.exit
            ));
            if !p.depends_on.is_empty() {
                s.push_str(&format!("        after: {}\n", p.depends_on.join(", ")));
            }
        }
        s
    }

    /// Row number (1-based) → phase id.
    pub fn id_at(&self, row: &str) -> Result<String, String> {
        let n: usize = row
            .trim()
            .parse()
            .map_err(|_| format!("{row:?} is not a row number"))?;
        self.phases
            .phases
            .get(n.wrapping_sub(1))
            .map(|p| p.id.clone())
            .ok_or_else(|| format!("no row {n}"))
    }
}

const HELP: &str = "\
  a            add a phase (number, title, exit)
  r <row>      rename            e <row>   set exit        o <row>   set outcome
  t <row>      set tier          d <row>   set depends-on (comma-separated rows, or empty)
  x <row>      remove (refused while Warrants name it)
  s            save: write the phases atom, propose the revision, sign once
  q            quit without saving
";

/// `war roadmap edit`: the line front end. Returns the exit code.
pub fn run(repo: &Repository) -> Result<u8, RepoError> {
    if !crate::sign::at_a_terminal() {
        let mut report = crate::diagnostic::Report::default();
        report.push(crate::diagnostic::Diagnostic::error(
            "roadmap.edit-no-tty",
            "-".to_owned(),
            "the roadmap is edited only from answers typed at a terminal; an agent proposes \
             with `war roadmap propose` after editing the atom, and a human signs",
        ));
        crate::check::print(&report);
        return Ok(crate::EXIT_NOT_READY);
    }
    let loaded = crate::roadmap_cmd::load(repo)
        .map_err(|e| RepoError::Message(e.to_string()))?
        .ok_or_else(|| RepoError::Message("no roadmap record to edit".to_owned()))?;
    let atom = loaded
        .manifest
        .atoms
        .iter()
        .find(|a| a.role == "phases")
        .map(|a| loaded.dir.join(&a.path))
        .ok_or_else(|| RepoError::Message("the roadmap has no phases atom".to_owned()))?;
    let source = std::fs::read_to_string(&atom).map_err(|source| RepoError::Io {
        context: format!("could not read {atom}"),
        source,
    })?;
    let members: BTreeMap<String, usize> = crate::status::build(repo)?
        .objectives
        .iter()
        .filter_map(|o| {
            o.roadmap_ref
                .as_ref()
                .map(|r| (format!("{}-PHASE-{}", r.prefix, r.phase), o.warrants.len()))
        })
        .collect();
    let mut ed =
        Editor::new(&source, &loaded.manifest.prefix, members).map_err(RepoError::Message)?;
    println!(
        "{} roadmap — edit by keystroke; nothing is written until `s`\n",
        loaded.manifest.program
    );
    loop {
        print!("{}", ed.screen());
        println!();
        let Some(cmd) = ask("  (a r e o t d x s q, ? for help) > ")? else {
            return Ok(crate::EXIT_OK);
        };
        let (verb, arg) = cmd.split_once(' ').unwrap_or((cmd.as_str(), ""));
        let result = match verb {
            "?" | "h" => {
                print!("{HELP}");
                Ok(())
            }
            "q" => {
                if ed.dirty {
                    println!("  quit: nothing written");
                }
                return Ok(crate::EXIT_OK);
            }
            "a" => (|| {
                let n = ask("  number: ")?.unwrap_or_default();
                let number: u8 = n
                    .parse()
                    .map_err(|_| RepoError::Message(format!("{n:?} is not a number")))?;
                let title = ask("  title: ")?.unwrap_or_default();
                let exit = ask("  exit criterion: ")?.unwrap_or_default();
                Ok::<_, RepoError>(Edit::Add {
                    number,
                    title,
                    exit,
                })
            })()
            .map_err(|e| e.to_string())
            .and_then(|e| ed.apply(e)),
            "r" | "e" | "o" | "t" | "d" | "x" => match ed.id_at(arg) {
                Err(e) => Err(e),
                Ok(id) => {
                    let edit = match verb {
                        "r" => ask(&format!("  new title for {id}: "))?.map(|title| Edit::Rename {
                            id: id.clone(),
                            title,
                        }),
                        "e" => ask(&format!("  exit for {id}: "))?.map(|exit| Edit::SetExit {
                            id: id.clone(),
                            exit,
                        }),
                        "o" => {
                            ask(&format!("  outcome for {id}: "))?.map(|outcome| Edit::SetOutcome {
                                id: id.clone(),
                                outcome,
                            })
                        }
                        "t" => ask(&format!("  tier for {id} (empty for none): "))?.map(|t| {
                            Edit::SetPriority {
                                id: id.clone(),
                                tier: (!t.is_empty()).then_some(t),
                            }
                        }),
                        "d" => ask(&format!("  {id} comes after rows (comma-separated): "))?.map(
                            |rows| {
                                let deps: Result<Vec<String>, String> = rows
                                    .split(',')
                                    .map(str::trim)
                                    .filter(|r| !r.is_empty())
                                    .map(|r| ed.id_at(r))
                                    .collect();
                                match deps {
                                    Ok(depends_on) => Edit::SetDependsOn {
                                        id: id.clone(),
                                        depends_on,
                                    },
                                    Err(_) => Edit::SetDependsOn {
                                        id: id.clone(),
                                        depends_on: vec!["?".into()],
                                    },
                                }
                            },
                        ),
                        _ => Some(Edit::Remove { id: id.clone() }),
                    };
                    match edit {
                        None => return Ok(crate::EXIT_OK),
                        Some(e) => ed.apply(e),
                    }
                }
            },
            "s" => {
                if !ed.dirty {
                    println!("  nothing changed");
                    continue;
                }
                save_and_sign(repo, &atom, &ed)?;
                return Ok(crate::EXIT_OK);
            }
            "" => Ok(()),
            other => Err(format!("{other:?} is not a command; ? lists them")),
        };
        if let Err(why) = result {
            println!("  refused: {why}");
        }
    }
}

/// Write the atom, propose the revision, and hand the one signature to a
/// child `war sign roadmap --ssh-sign` that owns the terminal.
fn save_and_sign(repo: &Repository, atom: &Utf8Path, ed: &Editor) -> Result<(), RepoError> {
    std::fs::write(atom, ed.render()).map_err(|source| RepoError::Io {
        context: format!("could not write {atom}"),
        source,
    })?;
    let report = crate::roadmap_cmd::propose(repo)?;
    for d in &report.diagnostics {
        println!("  {}", d.message);
    }
    println!("\n  one signature accepts it: war sign roadmap --ssh-sign");
    if ask("  sign now? [Y/n] ")?.is_some_and(|a| a.is_empty() || a.eq_ignore_ascii_case("y")) {
        let exe = std::env::current_exe().map_err(|source| RepoError::Io {
            context: "could not find this executable".to_owned(),
            source,
        })?;
        let _ = std::process::Command::new(exe)
            .arg("--root")
            .arg(repo.root.as_str())
            .args(["sign", "roadmap", "--ssh-sign"])
            .status();
    } else {
        println!("  later: war sign roadmap --ssh-sign  (the queue and the app list it)");
    }
    Ok(())
}

fn ask(text: &str) -> Result<Option<String>, RepoError> {
    use std::io::Write;
    let mut out = std::io::stdout();
    out.write_all(text.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;

    const ATOM: &str = r#"schema: "oh.war/roadmap-phases/v1"

# a human's comment, kept

tiers:
  - id: "1"
    title: "Library"

phases:
  - id: "OW-PHASE-0"
    title: "Start"
    exit: "it starts"
    depends_on: []
    priority: "1"
    open: []
  - id: "OW-PHASE-1"
    title: "Adopt"
    exit: "it is adopted"
    depends_on: ["OW-PHASE-0"]
    open: ["later"]
"#;

    fn ed() -> Editor {
        Editor::new(ATOM, "OW", [("OW-PHASE-1".to_owned(), 3)].into()).unwrap()
    }

    #[test]
    fn canned_edits_round_trip_through_the_real_parser() {
        let mut e = ed();
        e.apply(Edit::Add {
            number: 2,
            title: "Grow".into(),
            exit: "it grows".into(),
        })
        .unwrap();
        e.apply(Edit::SetDependsOn {
            id: "OW-PHASE-2".into(),
            depends_on: vec!["OW-PHASE-1".into()],
        })
        .unwrap();
        e.apply(Edit::Rename {
            id: "OW-PHASE-0".into(),
            title: "Begin 'now'".into(),
        })
        .unwrap();
        assert!(
            e.apply(Edit::Rename {
                id: "OW-PHASE-0".into(),
                title: "a \"quoted\" title".into(),
            })
            .unwrap_err()
            .contains("no escapes")
        );
        let text = e.render();
        assert!(text.contains("# a human's comment, kept"), "{text}");
        let back = parse_phases(&text, "OW").unwrap();
        assert_eq!(back.phases.len(), 3);
        assert_eq!(back.get("OW-PHASE-0").unwrap().title, "Begin 'now'");
        assert_eq!(back.get("OW-PHASE-1").unwrap().open, ["later"]);
        assert!(e.dirty);
    }

    #[test]
    fn it_refuses_what_check_would_refuse_and_leaves_the_editor_as_it_was() {
        let mut e = ed();
        let before = e.render();
        assert!(
            e.apply(Edit::Remove {
                id: "OW-PHASE-1".into()
            })
            .unwrap_err()
            .contains("3 Warrant")
        );
        assert!(
            e.apply(Edit::Remove {
                id: "OW-PHASE-0".into()
            })
            .unwrap_err()
            .contains("depends on")
        );
        assert!(
            e.apply(Edit::SetDependsOn {
                id: "OW-PHASE-0".into(),
                depends_on: vec!["OW-PHASE-1".into()]
            })
            .unwrap_err()
            .contains("cycle")
        );
        assert!(
            e.apply(Edit::SetPriority {
                id: "OW-PHASE-0".into(),
                tier: Some("9".into())
            })
            .is_err()
        );
        assert!(
            e.apply(Edit::Add {
                number: 1,
                title: "dup".into(),
                exit: String::new()
            })
            .is_err()
        );
        assert_eq!(e.render(), before);
        assert!(!e.dirty);
    }

    #[test]
    fn rows_are_one_based() {
        let e = ed();
        assert_eq!(e.id_at("2").unwrap(), "OW-PHASE-1");
        assert!(e.id_at("0").is_err());
        assert!(e.id_at("x").is_err());
    }
}
