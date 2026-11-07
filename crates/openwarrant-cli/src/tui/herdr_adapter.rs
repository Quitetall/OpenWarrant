// SPDX-License-Identifier: Apache-2.0
//! Herdr as an optional adapter: the Live panel's text, from an agent's
//! terminal when Herdr is there, and from the working tree when it is not.
//! Everything else in the app works the same either way. Called only from the
//! reader's Live thread, so a slow socket never holds a keypress.

use camino::{Utf8Path, Utf8PathBuf};

use super::model::{Herdr, Live};
use crate::herdr;

/// The most lines of `git` output the panel carries.
const GIT_LINES: usize = 200;
/// How much of an agent's terminal one read asks for.
const TERMINAL_LINES: usize = 200;

/// What to look at: a pane, and the checkout to fall back on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Focus {
    pub pane: Option<String>,
    pub workdir: Utf8PathBuf,
}

/// One look: the fleet and the focused agent's terminal, or the working tree.
#[must_use]
pub fn look(focus: &Focus) -> Live {
    let herdr_state = match herdr::ping() {
        Ok(pong) => Herdr::Connected {
            version: pong.version,
        },
        Err(why) => Herdr::Absent { why },
    };
    if !matches!(herdr_state, Herdr::Connected { .. }) {
        return Live {
            herdr: herdr_state,
            agents: Vec::new(),
            text: working_tree(&focus.workdir),
        };
    }
    let agents = match herdr::list_agents() {
        Ok(agents) => agents,
        Err(why) => {
            return Live {
                herdr: herdr_state,
                agents: Vec::new(),
                text: format!(
                    "UNKNOWN: Herdr answered, but its agent list could not be read: {why}\n\n{}",
                    working_tree(&focus.workdir)
                ),
            };
        }
    };
    let pane = focus
        .pane
        .as_ref()
        .filter(|p| agents.iter().any(|a| &a.pane_id == *p))
        .or_else(|| agents.first().map(|a| &a.pane_id))
        .cloned();
    let text = pane.map_or_else(
        || working_tree(&focus.workdir),
        |p| {
            herdr::read_agent(&p, TERMINAL_LINES)
                .unwrap_or_else(|why| format!("UNKNOWN: could not read {p}: {why}"))
        },
    );
    Live {
        herdr: herdr_state,
        agents,
        text,
    }
}

/// `git status` and the diff's shape, as git says them.
#[must_use]
pub fn working_tree(dir: &Utf8Path) -> String {
    let mut out = format!("{dir}\n\n");
    for (title, args) in [
        ("git status", &["status", "--short", "--branch"][..]),
        ("git diff", &["diff", "--stat"][..]),
    ] {
        out.push_str(title);
        out.push('\n');
        match std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .output()
        {
            Ok(o) if o.status.success() => {
                let text = String::from_utf8_lossy(&o.stdout);
                let mut lines = text.lines();
                for line in lines.by_ref().take(GIT_LINES) {
                    out.push_str(line);
                    out.push('\n');
                }
                let more = lines.count();
                if more > 0 {
                    out.push_str(&format!("… {more} more line(s)\n"));
                }
            }
            Ok(o) => out.push_str(&format!(
                "UNKNOWN: git exited {}: {}\n",
                o.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&o.stderr).trim()
            )),
            Err(e) => out.push_str(&format!("UNKNOWN: could not run git: {e}\n")),
        }
        out.push('\n');
    }
    out
}
