//! `war init` as a conversation: the state machine (OW-WAR-0112 M4).
//!
//! Setup was ten commands and a flag nobody remembers. This module is the
//! part of the conversation that can be tested without a terminal: given
//! what the tree already holds, which question comes next; given an answer,
//! what should happen. It performs no I/O in a transition. Two front ends
//! drive it — the line prompts in `init::guided` and the app's Setup tab —
//! and both apply the `Effect`s it returns, then hand back fresh `Facts`.
//!
//! # State lives in the tree, never in a file of its own
//!
//! `Facts::read` looks at `openwarrant.toml`, the two authority files, the
//! SAS revisions and the first Warrant's records, and `step_for` picks the
//! first step those facts leave undone. An interrupted setup resumes at the
//! right question because there is nothing else to resume from — the same
//! rule the release wizard follows.
//!
//! # The authority files
//!
//! `roles.toml` and `allowed_signers` are written HERE, from a human's
//! answers, and the policy that makes that acceptable is narrow and stated
//! in the file header the tool writes: only from answers typed at a
//! terminal (`sign::at_a_terminal()`, the same §27.2 gate `war sign` uses —
//! the front end holds it, this module cannot see a terminal), only once
//! (an existing file is never touched — `Effect::Write` says so and the
//! front end refuses), and the agent entry is `performer` and nothing
//! else — no answer can widen it, because no question asks. The one thing
//! `war` cannot check, that the key was loaded with `ssh-add -c`, is asked,
//! and the answer is recorded as the human's unverified statement.

use camino::Utf8Path;

/// The questions, in the order a fresh repository meets them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Name and namespace: `war init --program`.
    Program,
    /// Where governed work begins, in a repository with history
    /// (OW-WAR-0124): the `[adoption]` baseline.
    Baseline,
    /// Who signs, and with which key.
    Signer,
    /// The `-c` question: unverifiable, asked, recorded.
    KeyLoaded,
    /// `war sas propose 0.1.0`.
    Sas,
    /// A human signs the SAS.
    SignSas,
    /// The adopt Warrant: check, compile, the request.
    Authorize,
    /// A human signs the adopt Warrant.
    SignAuthorize,
    Done,
}

impl Step {
    /// One line naming the step, for the Setup tab and the resume message.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Program => "name the program",
            Self::Baseline => "say where governed work begins",
            Self::Signer => "say who signs",
            Self::KeyLoaded => "confirm the key asks you",
            Self::Sas => "record the SAS",
            Self::SignSas => "sign the SAS (human)",
            Self::Authorize => "prepare the first Warrant",
            Self::SignAuthorize => "sign the first Warrant (human)",
            Self::Done => "done",
        }
    }
}

/// What the tree already holds. Read once per turn by `Facts::read`, the
/// only function here that touches disk — and it only reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Facts {
    pub initialized: bool,
    pub program: Option<String>,
    pub namespace: Option<String>,
    pub roles_exist: bool,
    pub signers_exist: bool,
    /// Every recorded SAS revision: (version, accepted).
    pub sas: Vec<(String, bool)>,
    /// The first Warrant, when one exists: (alias, authorized).
    pub adopt: Option<(String, bool)>,
    /// Commits in HEAD's history; 0 with none, or outside git (OW-WAR-0124).
    pub commits: u64,
    /// HEAD's full id: the baseline proposed when there is history.
    pub head: Option<String>,
    /// `[adoption] baseline`, when `openwarrant.toml` records one.
    pub baseline: Option<String>,
    /// An existing ADR directory `war migrate` could import, if one is found.
    pub adr_dir: Option<String>,
}

impl Facts {
    /// Read the tree. Absent, unreadable and malformed all read as "not
    /// there yet": the conversation then asks, and the write-once rule at
    /// the front end keeps it from clobbering a file that merely failed to
    /// parse.
    #[must_use]
    pub fn read(root: &Utf8Path) -> Self {
        let config = std::fs::read_to_string(root.join(super::CONFIG_FILE)).ok();
        let (program, namespace) = config
            .as_deref()
            .and_then(|t| toml::from_str::<toml::Value>(t).ok())
            .and_then(|v| {
                let p = v.get("project")?;
                Some((
                    p.get("name")?.as_str()?.to_owned(),
                    p.get("namespace")?.as_str()?.to_owned(),
                ))
            })
            .map_or((None, None), |(p, n)| (Some(p), Some(n)));
        let baseline = config
            .as_deref()
            .and_then(|t| toml::from_str::<toml::Value>(t).ok())
            .and_then(|v| Some(v.get("adoption")?.get("baseline")?.as_str()?.to_owned()));
        let (head, commits) = match super::history(root) {
            super::History::Inside {
                head: Some((id, count)),
            } => (Some(id), count),
            _ => (None, 0),
        };
        let auth = root.join("docs/authority");
        let mut sas: Vec<(String, bool)> = std::fs::read_dir(root.join("docs/sas/revisions"))
            .map(|rd| {
                rd.filter_map(Result::ok)
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        let version = name.strip_suffix(".toml")?.to_owned();
                        let text = std::fs::read_to_string(e.path()).ok()?;
                        let accepted = toml::from_str::<toml::Value>(&text)
                            .ok()
                            .and_then(|v| v.get("state")?.as_str().map(|s| s == "accepted"))
                            .unwrap_or(false);
                        Some((version, accepted))
                    })
                    .collect()
            })
            .unwrap_or_default();
        sas.sort();
        let adopt = namespace.as_deref().and_then(|ns| {
            let alias = format!("{ns}-WAR-0001");
            let dir = root.join("docs/warrants").join(&alias);
            dir.join("manifest.toml").is_file().then(|| {
                let authorized = std::fs::read_to_string(dir.join("authorization.toml"))
                    .ok()
                    .and_then(|t| toml::from_str::<toml::Value>(&t).ok())
                    .and_then(|v| {
                        v.get("revision")?
                            .get("state")?
                            .as_str()
                            .map(|s| s.eq_ignore_ascii_case("authorized"))
                    })
                    .unwrap_or(false);
                (alias, authorized)
            })
        });
        Self {
            initialized: config.is_some(),
            program,
            namespace,
            roles_exist: auth.join("roles.toml").is_file(),
            signers_exist: auth.join("allowed_signers").is_file(),
            sas,
            adopt,
            commits,
            head,
            baseline,
            adr_dir: super::adr_dirs(root).first().map(|d| (*d).to_owned()),
        }
    }
}

/// The first step the facts leave undone.
#[must_use]
pub fn step_for(f: &Facts) -> Step {
    if !f.initialized {
        return Step::Program;
    }
    // A repository with history says where governed work begins before
    // anything else is set up. Asked while the setup is at its start — no
    // SAS revision recorded yet; a repository further along adopted before
    // this step existed, and is not asked again.
    if f.commits > 0 && f.baseline.is_none() && f.sas.is_empty() {
        return Step::Baseline;
    }
    if !(f.roles_exist && f.signers_exist) {
        return Step::Signer;
    }
    if f.sas.is_empty() {
        return Step::Sas;
    }
    if !f.sas.iter().any(|(_, accepted)| *accepted) {
        return Step::SignSas;
    }
    match &f.adopt {
        // `war init` without `--program` scaffolds no first Warrant; the
        // setup is then complete and `war new` is the next thing.
        None => Step::Done,
        Some((_, false)) => Step::Authorize,
        Some((_, true)) => Step::Done,
    }
}

/// A human's answer to the current step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    Program {
        name: String,
        namespace: String,
    },
    /// The commit governed work starts from; empty confirms the proposed one.
    Baseline(String),
    Signer {
        /// The name in `roles.toml`; spaces allowed.
        name: String,
        /// The `ssh_principal` and the first field of the allowed_signers
        /// line. No spaces (OpenSSH's rule).
        principal: String,
        /// One line as `ssh-add -L` prints it: `<keytype> <base64> [comment]`.
        key: String,
    },
    /// The `-c` question.
    KeyLoaded(bool),
    /// "go ahead" for a step that only needs consent (propose, prepare).
    Proceed,
    /// "sign it now" / "not now" for a signing step.
    SignNow(bool),
}

/// What the front end must do. Every write carries its full text, so the
/// front end can show it before it exists and refuse if the path does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// `war init --program <program> --namespace <namespace>`.
    Scaffold { program: String, namespace: String },
    /// Write once; never over an existing file.
    Write { path: String, text: String },
    /// Record `[adoption] baseline` in `openwarrant.toml` — once. The front
    /// end resolves the commit and refuses one outside the history.
    Adopt { baseline: String },
    /// `war sas propose <version>`.
    ProposeSas { version: String },
    /// Print the command and, on consent, run `war sign <target> --ssh-sign`
    /// in a child that inherits the terminal.
    Sign { target: String },
    /// `war check`, `war compile`, then `war authorize <alias>` printed.
    Prepare { alias: String },
    /// Something to tell the human.
    Say(String),
}

/// The conversation's state: the facts and the answers not yet written.
#[derive(Debug, Clone)]
pub struct Machine {
    pub facts: Facts,
    pub step: Step,
    /// Stamped by the front end at construction — the machine has no clock.
    now: String,
    /// The signer's answers, held until the `-c` question is answered so the
    /// file header can record both at once.
    pending_signer: Option<(String, String, String)>,
}

impl Machine {
    #[must_use]
    pub fn new(facts: Facts, now: impl Into<String>) -> Self {
        let step = step_for(&facts);
        Self {
            facts,
            step,
            now: now.into(),
            pending_signer: None,
        }
    }

    /// Fresh facts after the front end applied effects; the step follows.
    pub fn observe(&mut self, facts: Facts) {
        self.facts = facts;
        self.step = if self.pending_signer.is_some() && self.step == Step::KeyLoaded {
            Step::KeyLoaded
        } else {
            step_for(&self.facts)
        };
    }

    /// What this step asks, for a line front end. Multi-line; the front end
    /// prints it verbatim.
    #[must_use]
    pub fn question(&self) -> String {
        match self.step {
            Step::Program => "What is this program called, and what namespace prefixes its \
                              Warrants (uppercase letters, e.g. OW → OW-WAR-0001)?"
                .to_owned(),
            Step::Baseline => {
                let head = self.facts.head.as_deref().unwrap_or("?");
                let mut q = format!(
                    "This repository has {} commit(s). Where does governed work begin? The \
                     adoption baseline is HEAD, {}, unless you name another commit in its \
                     history. Nothing up to it is claimed, owned or verified by any Warrant; \
                     `war telemetry` counts untracked work after it.",
                    self.facts.commits,
                    head.get(..12).unwrap_or(head)
                );
                if let Some(dir) = &self.facts.adr_dir {
                    q.push_str(&format!(
                        " Existing ADRs in {dir}/ can be imported afterwards with `war migrate`; \
                         nothing is imported now."
                    ));
                }
                q
            }
            Step::Signer => "Who signs for this repository — your name as it will appear in \
                             roles.toml, the principal you sign as (no spaces), and which \
                             loaded key (`ssh-add -L`) is yours?"
                .to_owned(),
            Step::KeyLoaded => "Was that key loaded with `ssh-add -c`, so that every signature \
                                asks you through a dialog? `war` cannot check this; your answer \
                                is recorded as your statement."
                .to_owned(),
            Step::Sas => format!(
                "Record the SAS as it stands as revision 0.1.0 (`war sas propose 0.1.0`)? \
                 Namespace {}.",
                self.facts.namespace.as_deref().unwrap_or("?")
            ),
            Step::SignSas => format!(
                "SAS {} is proposed and unsigned. Sign it now? (`war sign {} --ssh-sign` — one \
                 dialog)",
                self.latest_sas().unwrap_or("0.1.0"),
                self.latest_sas().unwrap_or("0.1.0")
            ),
            Step::Authorize => format!(
                "Prepare {} for authorization — check it, compile it, and print the request?",
                self.adopt_alias().unwrap_or("the first Warrant")
            ),
            Step::SignAuthorize => format!(
                "Authorize {} now? (`war sign {} --ssh-sign` — one dialog)",
                self.adopt_alias().unwrap_or("?"),
                self.adopt_alias().unwrap_or("?")
            ),
            Step::Done => "Setup is complete. `war next` says what comes next.".to_owned(),
        }
    }

    fn latest_sas(&self) -> Option<&str> {
        self.facts.sas.last().map(|(v, _)| v.as_str())
    }

    fn adopt_alias(&self) -> Option<&str> {
        self.facts.adopt.as_ref().map(|(a, _)| a.as_str())
    }

    /// Apply one answer: the effects the front end must perform. A wrong
    /// answer is refused with the reason and the step does not move.
    pub fn answer(&mut self, answer: Answer) -> Result<Vec<Effect>, String> {
        match (self.step, answer) {
            (Step::Program, Answer::Program { name, namespace }) => {
                let name = name.trim().to_owned();
                if name.is_empty() || name.contains(['\n', '\r', '|']) {
                    return Err("the program needs a name without a newline or `|`".to_owned());
                }
                if namespace.is_empty() || !namespace.bytes().all(|b| b.is_ascii_uppercase()) {
                    return Err(format!(
                        "the namespace is uppercase ASCII letters only, A–Z (it prefixes \
                         `{namespace}-SAS-RQ-001`); got {namespace:?}"
                    ));
                }
                Ok(vec![Effect::Scaffold {
                    program: name,
                    namespace,
                }])
            }
            (Step::Baseline, Answer::Baseline(named)) => {
                let named = named.trim();
                let commit = if named.is_empty() {
                    self.facts
                        .head
                        .clone()
                        .ok_or_else(|| "there is no HEAD to propose; name a commit".to_owned())?
                } else {
                    named.to_owned()
                };
                if commit.starts_with('-') || commit.contains(char::is_whitespace) {
                    return Err(format!("{commit:?} is not a commit name"));
                }
                Ok(vec![Effect::Adopt { baseline: commit }])
            }
            (
                Step::Signer,
                Answer::Signer {
                    name,
                    principal,
                    key,
                },
            ) => {
                let name = name.trim().to_owned();
                if name.is_empty() || name.contains(['\n', '\r', '"']) {
                    return Err("a name, without a newline or a double quote".to_owned());
                }
                let principal = principal.trim().to_owned();
                if principal.is_empty() || principal.contains(char::is_whitespace) {
                    return Err("the principal has no spaces (OpenSSH's rule)".to_owned());
                }
                let key = key.trim().to_owned();
                let fields: Vec<&str> = key.split_whitespace().collect();
                if fields.len() < 2
                    || !(fields[0].starts_with("ssh-") || fields[0].starts_with("sk-"))
                {
                    return Err(
                        "a key line as `ssh-add -L` prints it: `<keytype> <base64> [comment]`"
                            .to_owned(),
                    );
                }
                self.pending_signer = Some((name, principal, key));
                self.step = Step::KeyLoaded;
                Ok(vec![])
            }
            (Step::KeyLoaded, Answer::KeyLoaded(loaded)) => {
                let Some((name, principal, key)) = self.pending_signer.take() else {
                    return Err("no signer answered yet".to_owned());
                };
                let mut effects = Vec::new();
                if !self.facts.roles_exist {
                    effects.push(Effect::Write {
                        path: "docs/authority/roles.toml".to_owned(),
                        text: render_roles(&name, &principal, &self.now, loaded),
                    });
                }
                if !self.facts.signers_exist {
                    effects.push(Effect::Write {
                        path: "docs/authority/allowed_signers".to_owned(),
                        text: render_allowed_signers(&name, &principal, &key, &self.now, loaded),
                    });
                }
                if !loaded {
                    effects.push(Effect::Say(
                        "Load the key with confirmation on before signing anything:\n  ssh-add -c \
                         ~/.ssh/<your-key>\nWithout -c, any process that can reach the agent \
                         socket can sign as you — including an AI agent's shell."
                            .to_owned(),
                    ));
                }
                Ok(effects)
            }
            (Step::Sas, Answer::Proceed) => Ok(vec![Effect::ProposeSas {
                version: "0.1.0".to_owned(),
            }]),
            (Step::SignSas, Answer::SignNow(yes)) => Ok(if yes {
                vec![Effect::Sign {
                    target: self.latest_sas().unwrap_or("0.1.0").to_owned(),
                }]
            } else {
                vec![Effect::Say(format!(
                    "Later: `war sign {} --ssh-sign`. `war next` will keep saying so.",
                    self.latest_sas().unwrap_or("0.1.0")
                ))]
            }),
            (Step::Authorize, Answer::Proceed) => Ok(vec![Effect::Prepare {
                alias: self.adopt_alias().unwrap_or_default().to_owned(),
            }]),
            (Step::SignAuthorize, Answer::SignNow(yes)) => Ok(if yes {
                vec![Effect::Sign {
                    target: self.adopt_alias().unwrap_or_default().to_owned(),
                }]
            } else {
                vec![Effect::Say(format!(
                    "Later: `war sign {} --ssh-sign`.",
                    self.adopt_alias().unwrap_or("?")
                ))]
            }),
            (Step::Done, _) => Ok(vec![]),
            (step, other) => Err(format!(
                "{} does not take that answer ({other:?})",
                step.title()
            )),
        }
    }

    /// After `Prepare` the next question is the signature, even though the
    /// tree looks the same as before it.
    pub fn prepared(&mut self) {
        if self.step == Step::Authorize {
            self.step = Step::SignAuthorize;
        }
    }
}

fn header(name: &str, now: &str, loaded: bool) -> String {
    format!(
        "# Written by `war init` on {now} from answers typed at a terminal by {name:?}.\n\
         #\n\
         # THE RULE FOR THIS FILE: a tool writes it only from a human's answers at a\n\
         # terminal, once. No command edits it afterwards — every tool in this\n\
         # workspace reads it, and `war init` refuses to touch one that exists.\n\
         # Authority still enters from outside the system it governs: the answers\n\
         # were a human's, the terminal gate is the one `war sign` uses (§27.2), and\n\
         # a pseudo-terminal defeats it exactly as it defeats `war sign` — the\n\
         # residual THREAT_MODEL entry 2 already accepts (OW-ADR-0021, Consequences).\n\
         #\n\
         # The human stated their key was loaded with `ssh-add -c`: {}.\n\
         # `war` cannot check that; it is their statement, recorded as given.\n",
        if loaded {
            "yes"
        } else {
            "NO — signatures will not ask; load it with -c before trusting one"
        }
    )
}

/// `roles.toml`: the human with every human role, the agent as `performer`
/// and nothing else. No answer reaches the agent entry.
#[must_use]
pub fn render_roles(name: &str, principal: &str, now: &str, loaded: bool) -> String {
    format!(
        "# Actor role assignments (SAS §27.4).\n{}\n\
         [[assignment]]\n\
         actor = {name:?}\n\
         actor_kind = \"human\"\n\
         roles = [\"authorizer\", \"resolver\", \"risk_acceptor\", \"judge\"]\n\
         # Who granted this: the repository owner, naming themselves as the source of\n\
         # their own authority — the honest record of a sole-owner project.\n\
         assigned_by = {name:?}\n\
         effective_time = {now:?}\n\
         note = \"Repository owner, from `war init`.\"\n\
         ssh_principal = {principal:?}\n\
         \n\
         # The agent is a performer and nothing else (§27.1). No question in `war\n\
         # init` can widen this; a human edits it, or nobody does.\n\
         [[assignment]]\n\
         actor = \"claude\"\n\
         actor_kind = \"agent\"\n\
         roles = [\"performer\"]\n\
         assigned_by = {name:?}\n\
         effective_time = {now:?}\n\
         note = \"Drafts and executes. Authorizes nothing, resolves nothing (§27.2).\"\n",
        header(name, now, loaded)
    )
}

/// `allowed_signers`: one line, both namespaces, as OpenSSH reads it.
#[must_use]
pub fn render_allowed_signers(
    name: &str,
    principal: &str,
    key: &str,
    now: &str,
    loaded: bool,
) -> String {
    format!(
        "# Who may sign a response with an ssh key (`war sign --ssh-sign`).\n{}\
         #\n\
         # Format: <principal> namespaces=\"oh.war/response,oh.war/dsse\" <keytype> <base64> [comment]\n\
         # Both namespaces: the first signs the response, the second the attestation.\n\
         \n\
         {principal} namespaces=\"oh.war/response,oh.war/dsse\" {key}\n",
        header(name, now, loaded)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(initialized: bool) -> Facts {
        Facts {
            initialized,
            program: initialized.then(|| "Demo".to_owned()),
            namespace: initialized.then(|| "DM".to_owned()),
            ..Facts::default()
        }
    }

    #[test]
    fn the_first_step_follows_the_tree_not_a_state_file() {
        assert_eq!(step_for(&facts(false)), Step::Program);
        let mut f = facts(true);
        assert_eq!(step_for(&f), Step::Signer);
        f.roles_exist = true;
        assert_eq!(
            step_for(&f),
            Step::Signer,
            "one file of two is still unset up"
        );
        f.signers_exist = true;
        assert_eq!(step_for(&f), Step::Sas);
        f.sas = vec![("0.1.0".to_owned(), false)];
        assert_eq!(step_for(&f), Step::SignSas);
        f.sas = vec![("0.1.0".to_owned(), true)];
        f.adopt = Some(("DM-WAR-0001".to_owned(), false));
        assert_eq!(step_for(&f), Step::Authorize);
        f.adopt = Some(("DM-WAR-0001".to_owned(), true));
        assert_eq!(step_for(&f), Step::Done);
    }

    #[test]
    fn canned_answers_walk_to_done_and_every_write_is_from_a_typed_answer() {
        let mut m = Machine::new(facts(false), "2026-09-22T00:00:00Z");
        assert!(
            m.answer(Answer::Program {
                name: "Demo".into(),
                namespace: "dm".into()
            })
            .unwrap_err()
            .contains("uppercase")
        );
        let e = m
            .answer(Answer::Program {
                name: "Demo".into(),
                namespace: "DM".into(),
            })
            .unwrap();
        assert_eq!(
            e,
            vec![Effect::Scaffold {
                program: "Demo".into(),
                namespace: "DM".into()
            }]
        );
        m.observe(facts(true));
        assert_eq!(m.step, Step::Signer);
        assert!(
            m.answer(Answer::Signer {
                name: "Ada".into(),
                principal: "ada lovelace".into(),
                key: "ssh-ed25519 AAAA ada@host".into()
            })
            .is_err()
        );
        assert!(
            m.answer(Answer::Signer {
                name: "Ada".into(),
                principal: "ada".into(),
                key: "ssh-ed25519 AAAA ada@host".into(),
            })
            .unwrap()
            .is_empty()
        );
        assert_eq!(m.step, Step::KeyLoaded);
        let writes = m.answer(Answer::KeyLoaded(true)).unwrap();
        let paths: Vec<&str> = writes
            .iter()
            .filter_map(|e| match e {
                Effect::Write { path, .. } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(
            paths,
            [
                "docs/authority/roles.toml",
                "docs/authority/allowed_signers"
            ]
        );
        for e in &writes {
            if let Effect::Write { text, .. } = e {
                assert!(text.contains("typed at a terminal by \"Ada\""));
                assert!(text.contains("loaded with `ssh-add -c`: yes"));
            }
        }
        let mut f = facts(true);
        f.roles_exist = true;
        f.signers_exist = true;
        m.observe(f.clone());
        assert_eq!(m.step, Step::Sas);
        assert_eq!(
            m.answer(Answer::Proceed).unwrap(),
            vec![Effect::ProposeSas {
                version: "0.1.0".into()
            }]
        );
        f.sas = vec![("0.1.0".into(), false)];
        m.observe(f.clone());
        assert_eq!(m.step, Step::SignSas);
        assert_eq!(
            m.answer(Answer::SignNow(true)).unwrap(),
            vec![Effect::Sign {
                target: "0.1.0".into()
            }]
        );
        f.sas = vec![("0.1.0".into(), true)];
        f.adopt = Some(("DM-WAR-0001".into(), false));
        m.observe(f.clone());
        assert_eq!(m.step, Step::Authorize);
        assert_eq!(
            m.answer(Answer::Proceed).unwrap(),
            vec![Effect::Prepare {
                alias: "DM-WAR-0001".into()
            }]
        );
        m.prepared();
        assert_eq!(m.step, Step::SignAuthorize);
        assert_eq!(
            m.answer(Answer::SignNow(false)).unwrap().len(),
            1,
            "declining says how to do it later"
        );
        f.adopt = Some(("DM-WAR-0001".into(), true));
        m.observe(f);
        assert_eq!(m.step, Step::Done);
    }

    /// OW-WAR-0124 OBL-006: with history, the step after Program is
    /// Baseline, and confirming it yields exactly one effect — the one that
    /// writes `[adoption]`. Nothing about who signs moves.
    #[test]
    fn with_history_the_step_after_program_is_baseline() {
        let head = "0123456789abcdef0123456789abcdef01234567".to_owned();
        let before = Facts {
            commits: 3,
            head: Some(head.clone()),
            ..facts(false)
        };
        let mut m = Machine::new(before, "2026-09-24T00:00:00Z");
        assert_eq!(m.step, Step::Program);
        let e = m
            .answer(Answer::Program {
                name: "Demo".into(),
                namespace: "DM".into(),
            })
            .unwrap();
        assert!(matches!(e.as_slice(), [Effect::Scaffold { .. }]), "{e:?}");
        let mut after = Facts {
            commits: 3,
            head: Some(head.clone()),
            ..facts(true)
        };
        m.observe(after.clone());
        assert_eq!(m.step, Step::Baseline, "Baseline follows Program");
        assert!(m.question().contains("3 commit(s)"), "{}", m.question());
        assert!(m.question().contains(&head[..12]), "{}", m.question());
        // Refusals: not a commit name, and the step does not move.
        assert!(m.answer(Answer::Baseline("--all".into())).is_err());
        assert!(m.answer(Answer::Baseline("a b".into())).is_err());
        assert!(m.answer(Answer::KeyLoaded(true)).is_err());
        assert_eq!(m.step, Step::Baseline);
        // Confirming the proposed commit: exactly one effect, the adoption.
        let e = m.answer(Answer::Baseline(String::new())).unwrap();
        assert_eq!(
            e,
            vec![Effect::Adopt {
                baseline: head.clone()
            }]
        );
        // Naming another commit carries that name to the front end, which
        // resolves it against the history and refuses one outside it.
        assert_eq!(
            m.answer(Answer::Baseline("v1.0".into())).unwrap(),
            vec![Effect::Adopt {
                baseline: "v1.0".into()
            }]
        );
        // Once recorded, the tree says so and the setup goes on to Signer.
        after.baseline = Some(head);
        m.observe(after);
        assert_eq!(m.step, Step::Signer);
    }

    /// With no history there is nothing to adopt: Baseline is skipped.
    #[test]
    fn without_history_baseline_is_skipped() {
        assert_eq!(step_for(&facts(true)), Step::Signer);
        let mut m = Machine::new(facts(false), "2026-09-24T00:00:00Z");
        m.answer(Answer::Program {
            name: "Demo".into(),
            namespace: "DM".into(),
        })
        .unwrap();
        m.observe(facts(true));
        assert_eq!(m.step, Step::Signer);
        // And a repository past its first SAS revision is not asked again.
        let mut f = Facts {
            commits: 40,
            head: Some("abc".into()),
            roles_exist: true,
            signers_exist: true,
            ..facts(true)
        };
        f.sas = vec![("0.1.0".into(), true)];
        assert_eq!(step_for(&f), Step::Done);
    }

    #[test]
    fn an_existing_authority_file_is_never_written_again() {
        let mut f = facts(true);
        f.roles_exist = true;
        let mut m = Machine::new(f, "2026-09-22T00:00:00Z");
        m.answer(Answer::Signer {
            name: "Ada".into(),
            principal: "ada".into(),
            key: "ssh-ed25519 AAAA".into(),
        })
        .unwrap();
        let writes = m.answer(Answer::KeyLoaded(false)).unwrap();
        assert!(
            writes
                .iter()
                .all(|e| !matches!(e, Effect::Write { path, .. } if path.ends_with("roles.toml"))),
            "roles.toml exists and is left alone: {writes:?}"
        );
        assert!(
            writes.iter().any(
                |e| matches!(e, Effect::Write { path, .. } if path.ends_with("allowed_signers"))
            )
        );
        assert!(
            writes
                .iter()
                .any(|e| matches!(e, Effect::Say(s) if s.contains("ssh-add -c"))),
            "a key not loaded with -c earns the instruction"
        );
    }

    #[test]
    fn the_agent_is_a_performer_and_nothing_else() {
        let roles = render_roles("Ada", "ada", "2026-09-22T00:00:00Z", true);
        let parsed: toml::Value = toml::from_str(&roles).expect("roles.toml parses");
        let assignments = parsed["assignment"].as_array().unwrap();
        let agents: Vec<&toml::Value> = assignments
            .iter()
            .filter(|a| a["actor_kind"].as_str() == Some("agent"))
            .collect();
        assert_eq!(agents.len(), 1);
        assert_eq!(
            agents[0]["roles"].as_array().unwrap(),
            &vec![toml::Value::String("performer".into())]
        );
        let human = assignments
            .iter()
            .find(|a| a["actor_kind"].as_str() == Some("human"))
            .unwrap();
        assert_eq!(human["actor"].as_str(), Some("Ada"));
        assert_eq!(human["ssh_principal"].as_str(), Some("ada"));
        let signers = render_allowed_signers("Ada", "ada", "ssh-ed25519 AAAA ada@host", "t", false);
        let line = signers.lines().last().unwrap();
        assert_eq!(
            line,
            "ada namespaces=\"oh.war/response,oh.war/dsse\" ssh-ed25519 AAAA ada@host"
        );
        assert!(signers.contains("ssh-add -c`: NO"));
    }
}
