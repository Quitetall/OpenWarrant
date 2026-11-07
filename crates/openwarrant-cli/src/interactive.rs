// SPDX-License-Identifier: Apache-2.0
//! `--interactive` on the six commands (t-3c16c W1.7): the questions a person
//! is asked, in plain lines, turned into the same arguments an agent passes.
//!
//! Nothing here acts. Each function reads answers and hands back what the
//! command would have been given, so an interactive run writes exactly what
//! the plain command writes. A caller without a terminal is refused before any
//! question ([`require_terminal`]): a script never meets a prompt it cannot
//! answer. The prompts take any reader and writer, so tests drive them whole.

use std::io::{BufRead, Write};

/// The refusal for a caller with no terminal, or under `--json`.
pub const NO_TERMINAL: &str = "interactive.no-terminal";

/// Why `--interactive` cannot ask, or `None` when it can.
#[must_use]
pub fn refusal(command: &str, json: bool, at_a_terminal: bool) -> Option<String> {
    (json || !at_a_terminal).then(|| {
        format!(
            "{NO_TERMINAL}: `war {command} --interactive` asks a person questions, so it needs a \
             terminal and no `--json`. Pass the arguments instead; nothing was asked or written"
        )
    })
}

/// Questions over a reader and a writer.
pub struct Prompts<R, W> {
    input: R,
    output: W,
}

impl<R: BufRead, W: Write> Prompts<R, W> {
    pub const fn new(input: R, output: W) -> Self {
        Self { input, output }
    }

    /// One trimmed line; empty at the end of the input.
    pub fn ask(&mut self, question: &str) -> std::io::Result<String> {
        write!(self.output, "{question} ")?;
        self.output.flush()?;
        let mut line = String::new();
        self.input.read_line(&mut line)?;
        Ok(line.trim().to_owned())
    }

    /// Lines until an empty one.
    pub fn many(&mut self, question: &str) -> std::io::Result<Vec<String>> {
        let mut out = Vec::new();
        loop {
            let line = self.ask(question)?;
            if line.is_empty() {
                return Ok(out);
            }
            out.push(line);
        }
    }

    pub fn yes(&mut self, question: &str) -> std::io::Result<bool> {
        let line = self.ask(&format!("{question} [y/N]"))?;
        Ok(matches!(line.to_ascii_lowercase().as_str(), "y" | "yes"))
    }

    /// A numbered list, and the index picked. Empty is `default`; anything
    /// that is no row is asked again, and the end of the input is `None`.
    pub fn pick(
        &mut self,
        question: &str,
        rows: &[String],
        default: Option<usize>,
    ) -> std::io::Result<Option<usize>> {
        for (n, row) in rows.iter().enumerate() {
            writeln!(self.output, "{:>3}. {row}", n + 1)?;
        }
        loop {
            write!(self.output, "{question} ")?;
            self.output.flush()?;
            let mut line = String::new();
            if self.input.read_line(&mut line)? == 0 {
                return Ok(None);
            }
            let line = line.trim();
            if line.is_empty() {
                return Ok(default);
            }
            match line.parse::<usize>() {
                Ok(n) if (1..=rows.len()).contains(&n) => return Ok(Some(n - 1)),
                _ => writeln!(self.output, "a number from 1 to {}", rows.len())?,
            }
        }
    }
}

/// What `war create --interactive` was told.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Create {
    pub title: String,
    pub items: Vec<String>,
    pub tests: Vec<String>,
    pub sign: bool,
}

/// The questions of `war create`. `None` when no title was given: nothing is
/// created from an empty answer.
pub fn create<R: BufRead, W: Write>(
    p: &mut Prompts<R, W>,
    title: Option<&str>,
) -> std::io::Result<Option<Create>> {
    let title = match title {
        Some(t) => t.to_owned(),
        None => p.ask("What does this work do?")?,
    };
    if title.is_empty() {
        return Ok(None);
    }
    let items = p.many("A step (empty to finish):")?;
    let test = p.ask("A test that decides it's done (empty for none):")?;
    let sign = p.yes("Does a person sign it off at the end?")?;
    Ok(Some(Create {
        title,
        items,
        tests: if test.is_empty() { vec![] } else { vec![test] },
        sign,
    }))
}

/// What `war check --interactive` checks.
#[derive(Debug, PartialEq, Eq)]
pub enum CheckScope {
    Everything,
    Changed,
    One(String),
}

/// The question of `war check`.
pub fn check<R: BufRead, W: Write>(
    p: &mut Prompts<R, W>,
    ids: &[String],
) -> std::io::Result<Option<CheckScope>> {
    let mut rows = vec!["everything".to_owned(), "what changed from HEAD".to_owned()];
    rows.extend(ids.iter().cloned());
    Ok(
        p.pick("Check what? (empty for everything)", &rows, Some(0))?
            .map(|n| match n {
                0 => CheckScope::Everything,
                1 => CheckScope::Changed,
                n => CheckScope::One(ids[n - 2].clone()),
            }),
    )
}

/// What `--interactive` turned a command into.
pub(crate) enum Answered {
    /// The command, with the answers in its arguments.
    Run(Box<crate::DailyCommand>),
    /// Nothing to do, and why: an empty answer writes nothing.
    Stopped(String),
}

/// The command `command` becomes once a person has answered. Without
/// `--interactive` it is returned as it came; without a terminal it is refused
/// before any question.
pub(crate) fn answer(
    ctx: &crate::Ctx,
    mut command: crate::DailyCommand,
) -> Result<Answered, Box<dyn std::error::Error>> {
    use crate::DailyCommand as D;
    let name = match &command {
        D::Create {
            interactive: true, ..
        } => "create",
        D::Start {
            interactive: true, ..
        } => "start",
        D::Done {
            interactive: true, ..
        } => "done",
        D::Status {
            interactive: true, ..
        } => "status",
        D::Check {
            interactive: true, ..
        } => "check",
        _ => return Ok(Answered::Run(Box::new(command))),
    };
    if let Some(why) = refusal(name, ctx.json, crate::sign::at_a_terminal()) {
        return Err(Box::new(crate::repo::RepoError::Message(why)));
    }
    let stdin = std::io::stdin();
    let mut p = Prompts::new(stdin.lock(), std::io::stdout());
    let stopped = |why: &str| Ok(Answered::Stopped(why.to_owned()));
    match &mut command {
        D::Create {
            title,
            items,
            tests,
            sign,
            ..
        } => {
            let Some(c) = create(&mut p, title.as_deref())? else {
                return stopped("no title, so nothing was created");
            };
            *title = Some(c.title);
            items.extend(c.items);
            tests.extend(c.tests);
            *sign |= c.sign;
        }
        D::Start { id, actor, .. } if id.is_empty() => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            let (tickets, _) = store.load_all()?;
            let mut seen = std::collections::BTreeSet::new();
            let rows: Vec<_> = crate::ticket::ready_rows(&store, &tickets)?
                .into_iter()
                .filter(|r| seen.insert(r.ticket.clone()))
                .collect();
            if rows.is_empty() {
                return stopped("nothing is ready to start");
            }
            let lines: Vec<String> = rows
                .iter()
                .map(|r| format!("{}  {}", r.ticket, r.title))
                .collect();
            let Some(n) = p.pick("Start which? (empty for the first)", &lines, Some(0))? else {
                return stopped("nothing was started");
            };
            id.push(rows[n].ticket.clone());
        }
        D::Done {
            target,
            note,
            release,
            reopen,
            actor,
            ..
        } => {
            if target.is_none() {
                let (_, store) = ctx.tickets(actor.as_deref())?;
                let held = crate::ticket::held_by_me(&store)?;
                *target = match held.as_slice() {
                    [] => None,
                    [one] => Some(one.clone()),
                    many => match p.pick("Finish which?", many, None)? {
                        Some(n) => Some(many[n].clone()),
                        None => return stopped("nothing was finished"),
                    },
                };
            }
            if note.is_none() && target.is_some() && !*release && !*reopen {
                *note =
                    Some(p.ask("What was done? (empty for no note)")?).filter(|n| !n.is_empty());
            }
        }
        D::Status { alias, .. } if alias.is_none() => {
            let repository = ctx.open_repo()?;
            let work = crate::work::Work::read(&repository);
            let open: Vec<_> = work.items().iter().filter(|i| !i.is_done()).collect();
            let lines: Vec<String> = open
                .iter()
                .map(|i| format!("{}  {}  {}", i.id, i.level.as_str(), i.title))
                .collect();
            if !lines.is_empty()
                && let Some(n) = p.pick("Show which? (empty for the summary)", &lines, None)?
            {
                *alias = Some(open[n].id.clone());
            }
        }
        D::Check {
            action,
            alias,
            changed,
            ..
        } if alias.is_none() && !*changed && action.is_none() => {
            let repository = ctx.open_repo()?;
            let ids: Vec<String> = crate::work::Work::read(&repository)
                .items()
                .iter()
                .filter(|i| !i.is_done())
                .map(|i| i.id.clone())
                .collect();
            match check(&mut p, &ids)? {
                None => return stopped("nothing was checked"),
                Some(CheckScope::Everything) => {}
                Some(CheckScope::Changed) => *changed = true,
                Some(CheckScope::One(id)) => *alias = Some(id),
            }
        }
        _ => {}
    }
    Ok(Answered::Run(Box::new(command)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompts(input: &str) -> Prompts<&[u8], Vec<u8>> {
        Prompts::new(input.as_bytes(), Vec::new())
    }

    #[test]
    fn no_terminal_is_refused_by_name_and_a_terminal_is_not() {
        assert!(
            refusal("start", false, false)
                .unwrap()
                .contains(NO_TERMINAL)
        );
        assert!(
            refusal("start", true, true).is_some(),
            "--json never prompts"
        );
        assert_eq!(refusal("start", false, true), None);
    }

    #[test]
    fn create_asks_for_what_the_flags_would_have_said() {
        let mut p = prompts("Ship it\none\ntwo\n\ncargo test\ny\n");
        assert_eq!(
            create(&mut p, None).unwrap().unwrap(),
            Create {
                title: "Ship it".into(),
                items: vec!["one".into(), "two".into()],
                tests: vec!["cargo test".into()],
                sign: true,
            }
        );
    }

    #[test]
    fn a_given_title_is_not_asked_for_and_no_title_creates_nothing() {
        let mut p = prompts("\n\nn\n");
        let made = create(&mut p, Some("Named")).unwrap().unwrap();
        assert_eq!(made.title, "Named");
        assert!(made.tests.is_empty() && !made.sign);
        assert_eq!(create(&mut prompts("\n"), None).unwrap(), None);
    }

    #[test]
    fn pick_takes_a_number_asks_again_and_never_invents_a_row() {
        let rows = ["a".to_owned(), "b".to_owned()];
        assert_eq!(
            prompts("9\nx\n2\n").pick("?", &rows, None).unwrap(),
            Some(1)
        );
        assert_eq!(prompts("\n").pick("?", &rows, Some(0)).unwrap(), Some(0));
        assert_eq!(prompts("\n").pick("?", &rows, None).unwrap(), None);
        assert_eq!(prompts("").pick("?", &rows, Some(0)).unwrap(), None);
    }

    #[test]
    fn check_offers_everything_changed_and_each_item() {
        let ids = ["t-1".to_owned()];
        assert_eq!(
            check(&mut prompts("\n"), &ids).unwrap(),
            Some(CheckScope::Everything)
        );
        assert_eq!(
            check(&mut prompts("2\n"), &ids).unwrap(),
            Some(CheckScope::Changed)
        );
        assert_eq!(
            check(&mut prompts("3\n"), &ids).unwrap(),
            Some(CheckScope::One("t-1".into()))
        );
    }
}
