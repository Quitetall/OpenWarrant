// SPDX-License-Identifier: Apache-2.0
//! `war view prime --prompt`: the brief of the item a prompt names, for a
//! harness to put in front of an agent before its first turn (t-3c16c W4).
//!
//! A plain agent has its item's text before turn 0; this gives a tracked one
//! the same start. It reads and never claims: taking the work stays
//! `war start`.
//!
//! A prompt is prose, not a query, so `resolve`'s "every word is in the
//! title" cannot be asked of it whole. What counts as naming an item is
//! deliberately narrow, because a brief for the wrong item costs tokens and
//! misleads, while a miss costs one turn. Each form goes through the lookup
//! `war start` uses:
//!
//! 1. an id: `t-x`, `i-y` or `t-x/i-y`, or a unique prefix of three or more
//!    characters. An id anywhere in a prompt outranks every form below;
//! 2. a backlog code in capitals (`T8`, `GH-12`) that leads an open Warrant's
//!    title or is one of its labels;
//! 3. a double-quoted phrase of two or more words, all in one title;
//! 4. a whole title of three or more words, its words in order.
//!
//! One open Warrant is a match. Several are a one-line list and nothing
//! loaded. None, or only done work, is silence. So is a Warrant this actor
//! already holds: it has that brief, from `war start` or the session hook.

use std::collections::BTreeSet;

use super::{
    Claims, Outcome, RepoError, Store, Target, Ticket, held_by_me, matching_words, owns_claim,
    ready_rows_at, render, resolve, words_of,
};

/// The most a brief adds to an agent's context, in bytes, its first line
/// included. About 1,500 tokens: above every item in the adoption study's
/// backlogs (the largest brief measured is 4,484 bytes), because a cut item is read
/// again whole. A long plan is cut and the cut declared.
pub const BRIEF_BUDGET: usize = 6144;

/// The most a list of several matches adds.
const LIST_BUDGET: usize = 320;

/// How much of a prompt is read: a name comes first, and a pasted log is not
/// a request to load what it mentions.
const PROMPT_WINDOW: usize = 8192;

/// An id's characters after `t-` or `i-`, at least. Fewer reads as a word
/// (`t-1`, `i-5`), and a prefix that short is a guess.
const MIN_ID_BODY: usize = 3;

/// A quoted phrase's words, at least.
const MIN_PHRASE_WORDS: usize = 2;

/// An unquoted title's words, at least: "add tests" appears in prose by
/// chance, "add tests for parser" does not.
const MIN_TITLE_WORDS: usize = 3;

const SCHEMA: &str = "oh.war/ticket-prompt/v1";

fn window(prompt: &str) -> &str {
    if prompt.len() <= PROMPT_WINDOW {
        return prompt;
    }
    let mut end = PROMPT_WINDOW;
    while !prompt.is_char_boundary(end) {
        end -= 1;
    }
    &prompt[..end]
}

/// The tokens of `prompt` that may be ids or codes: runs of letters, digits,
/// `-` and `/`, trimmed of those two at the edges.
fn tokens(prompt: &str) -> Vec<&str> {
    prompt
        .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '/'))
        .map(|t| t.trim_matches(|c| c == '-' || c == '/'))
        .filter(|t| !t.is_empty())
        .collect()
}

/// Whether `token` is shaped as an id with enough of it to mean one.
fn id_shaped(token: &str) -> bool {
    let head = token.split('/').next().unwrap_or(token);
    (head.starts_with("t-") || head.starts_with("w-") || head.starts_with("i-"))
        && head.chars().count() >= 2 + MIN_ID_BODY
}

/// The Warrants the id-shaped `token` could be: one when `resolve` answers,
/// each candidate of an ambiguous prefix, none when it names nothing.
fn by_id(tickets: &[Ticket], token: &str) -> Vec<usize> {
    match resolve(tickets, token) {
        Ok(Target::Ticket(n) | Target::Item(n, _)) => vec![n],
        Err(d) if d.rule == "ticket.ambiguous" => {
            let head = token.split('/').next().unwrap_or(token);
            (0..tickets.len())
                .filter(|&n| {
                    let t = &tickets[n];
                    if head.starts_with("t-") || head.starts_with("w-") {
                        t.id().starts_with(head)
                    } else {
                        t.checklist.ids().iter().any(|i| i.starts_with(head))
                    }
                })
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

/// `T8`, `GH-12`: capitals, an optional hyphen, digits, and nothing else.
fn is_code(token: &str) -> bool {
    let letters = token.bytes().take_while(u8::is_ascii_uppercase).count();
    let rest = token[letters..]
        .strip_prefix('-')
        .unwrap_or(&token[letters..]);
    (1..=6).contains(&letters)
        && (1..=6).contains(&rest.len())
        && rest.bytes().all(|b| b.is_ascii_digit())
}

/// The Warrants `code` identifies: the lookup's matches whose title opens
/// with it or that carry it as a label. A code in the middle of a title is
/// a word there (`Pin SHA256 digests`), not a name.
fn by_code(tickets: &[Ticket], code: &str) -> Vec<usize> {
    let want = words_of(code);
    matching_words(tickets, code)
        .into_iter()
        .filter(|&n| {
            let t = &tickets[n];
            words_of(&t.manifest.title).starts_with(&want)
                || t.manifest.labels.iter().any(|l| words_of(l) == want)
        })
        .collect()
}

/// The stretches of `prompt` between a pair of double quotes, straight or
/// curly; a quote left open closes nothing.
fn quoted(prompt: &str) -> Vec<String> {
    let text = prompt.replace(['\u{201c}', '\u{201d}'], "\"");
    let parts: Vec<&str> = text.split('"').collect();
    parts[..parts.len() - 1]
        .iter()
        .skip(1)
        .step_by(2)
        .map(|s| (*s).to_owned())
        .collect()
}

/// The open Warrants whose whole title is in `prompt`, word for word in order.
fn by_title(tickets: &[Ticket], words: &[String]) -> Vec<usize> {
    (0..tickets.len())
        .filter(|&n| {
            let title = words_of(&tickets[n].manifest.title);
            title.len() >= MIN_TITLE_WORDS && words.windows(title.len()).any(|w| w == title)
        })
        .collect()
}

/// The open Warrants `prompt` names, by index.
fn named(tickets: &[Ticket], prompt: &str) -> BTreeSet<usize> {
    let open = |found: Vec<usize>| found.into_iter().filter(|&n| !tickets[n].is_done());
    let prompt = window(prompt);
    let tokens = tokens(prompt);
    let by_ids: BTreeSet<usize> = tokens
        .iter()
        .filter(|t| id_shaped(t))
        .flat_map(|t| open(by_id(tickets, t)))
        .collect();
    if !by_ids.is_empty() {
        return by_ids;
    }
    let mut found: BTreeSet<usize> = tokens
        .iter()
        .filter(|t| is_code(t))
        .flat_map(|t| open(by_code(tickets, t)))
        .collect();
    for phrase in quoted(prompt) {
        if words_of(&phrase).len() >= MIN_PHRASE_WORDS {
            found.extend(open(matching_words(tickets, &phrase)));
        }
    }
    found.extend(open(by_title(tickets, &words_of(prompt))));
    found
}

/// `text` within `budget` bytes, cut at a line, and how many bytes went.
pub(super) fn fitted(text: &str, budget: usize) -> (&str, usize) {
    if text.len() <= budget {
        return (text, 0);
    }
    let mut end = budget;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let kept = text[..end].rfind('\n').map_or(&text[..end], |n| &text[..n]);
    let kept = kept.trim_end();
    (kept, text.len() - kept.len())
}

/// `text` shortened to `chars`, marked when anything went.
fn clipped(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_owned();
    }
    let kept: String = text.chars().take(chars.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// The one line for several matches, within [`LIST_BUDGET`].
fn listed(tickets: &[Ticket], found: &BTreeSet<usize>) -> String {
    const HEAD: &str = "OpenWarrant: that prompt could name ";
    const TAIL: &str = "; none loaded. `war start <id>` takes one.";
    let room = LIST_BUDGET - HEAD.len() - TAIL.len() - " and 99 more".len();
    let mut names = String::new();
    let mut shown = 0;
    for &n in found {
        let t = &tickets[n];
        let name = format!("{} ({})", t.id(), clipped(&t.manifest.title, 40));
        let added = if names.is_empty() { 0 } else { 2 } + name.len();
        if names.len() + added > room {
            break;
        }
        if !names.is_empty() {
            names.push_str(", ");
        }
        names.push_str(&name);
        shown += 1;
    }
    let more = found.len() - shown;
    if more > 0 {
        names.push_str(&format!(" and {more} more"));
    }
    format!("{HEAD}{names}{TAIL}")
}

/// The first `count` Warrants ready by their records alone, one row each, for
/// the managed block (`war admin agents-md --block --ready`). Claims are left
/// out: they live outside the tree, so a file holding them would differ
/// between one moment and the next with nothing tracked having moved. A row
/// someone has since claimed is refused by `war start`, by name.
pub fn ready_list(
    store: &Store,
    tickets: &[Ticket],
    count: usize,
) -> Result<Vec<String>, RepoError> {
    const TITLE_CHARS: usize = 60;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for row in ready_rows_at(store, tickets, Claims::new(), 0)? {
        if rows.len() == count {
            break;
        }
        if !seen.insert(row.ticket.clone()) {
            continue;
        }
        if let Some(t) = tickets.iter().find(|t| t.id() == row.ticket) {
            let title = t
                .manifest
                .title
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            rows.push(format!("{} {}", t.id(), clipped(&title, TITLE_CHARS)));
        }
    }
    Ok(rows)
}

/// `war view prime --prompt`: see the module. Writes nothing.
pub fn prompt_brief(store: &Store, prompt: &str) -> Result<Outcome, RepoError> {
    let quiet = |why: &str| {
        Outcome::ok(
            String::new(),
            serde_json::json!({"schema": SCHEMA, "named": [], "silent": why}),
        )
    };
    if !store.session_start {
        return Ok(quiet("session-start-off"));
    }
    let (tickets, _) = store.load_all()?;
    let found = named(&tickets, prompt);
    let Some(&first) = found.first() else {
        return Ok(quiet("names-nothing"));
    };
    if found.len() > 1 {
        let ids: Vec<&str> = found.iter().map(|&n| tickets[n].id()).collect();
        return Ok(Outcome::ok(
            listed(&tickets, &found),
            serde_json::json!({"schema": SCHEMA, "named": ids}),
        ));
    }
    let t = &tickets[first];
    let id = t.id();
    if held_by_me(store)?.iter().any(|h| h == id) {
        return Ok(quiet("held"));
    }
    let holder = store
        .claims()?
        .values()
        .flatten()
        .find(|c| c.ticket == id && !owns_claim(store, c))
        .map(|c| c.actor.clone());
    let status = holder
        .as_ref()
        .map_or_else(|| "not claimed".to_owned(), |a| format!("claimed by {a}"));
    let brief = render::brief_with_status(store, &tickets, t, &status);
    let head =
        format!("OpenWarrant: your prompt names {id}; its brief is below, so no lookup is needed.");
    let take = format!(
        " Take it with `war start {id} >/dev/null` in the same command as your first look at the code."
    );
    let cut_note = format!(
        "\n\n… cut to {BRIEF_BUDGET} bytes; `war start {id}` takes it and prints the rest."
    );
    let room = BRIEF_BUDGET - head.len() - take.len().max(cut_note.len()) - 2;
    let (kept, cut) = fitted(&brief.human, room);
    let human = match (cut, &holder) {
        (0, None) => format!("{head}{take}\n\n{kept}"),
        (0, Some(_)) => format!("{head}\n\n{kept}"),
        _ => format!("{head}\n\n{kept}{cut_note}"),
    };
    let mut result = brief.result;
    result["schema"] = serde_json::json!(SCHEMA);
    result["named"] = serde_json::json!([id]);
    result["cut_bytes"] = serde_json::json!(cut);
    Ok(Outcome::ok(human, result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_is_capitals_then_digits_and_nothing_else() {
        for yes in ["T8", "GH-12", "W4", "JIRA-1234"] {
            assert!(is_code(yes), "{yes}");
        }
        for no in [
            "t8",
            "T",
            "8",
            "T8a",
            "SHAPES-",
            "T-",
            "TOOLONGCODE-1",
            "é8",
            "T٣",
        ] {
            assert!(!is_code(no), "{no}");
        }
    }

    #[test]
    fn an_id_needs_enough_of_itself_to_mean_one() {
        for yes in ["t-3c1", "i-abc", "t-3c16c/i-9"] {
            assert!(id_shaped(yes), "{yes}");
        }
        for no in ["t-1", "i-5", "t-", "tshirt", "x-3c16c"] {
            assert!(!id_shaped(no), "{no}");
        }
    }

    #[test]
    fn tokens_and_quotes_come_out_of_prose() {
        assert_eq!(tokens("do t-abc, (T8)/"), vec!["do", "t-abc", "T8"]);
        assert_eq!(
            quoted("the \"monthly budget\" and “net worth” ok \"open"),
            vec!["monthly budget", "net worth"]
        );
    }

    #[test]
    fn a_cut_is_at_a_line_and_on_a_character() {
        assert_eq!(fitted("short", 10), ("short", 0));
        let (kept, cut) = fitted("one\ntwo\nthree", 9);
        assert_eq!((kept, cut), ("one\ntwo", 6));
        let (kept, _) = fitted("ééééé", 5);
        assert_eq!(kept, "éé");
    }

    #[test]
    fn a_long_prompt_is_read_only_as_far_as_the_window() {
        let long = format!("{}é", "x".repeat(PROMPT_WINDOW - 1));
        assert_eq!(window(&long).len(), PROMPT_WINDOW - 1);
    }
}
