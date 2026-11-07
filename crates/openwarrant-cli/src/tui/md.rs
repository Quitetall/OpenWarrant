// SPDX-License-Identifier: Apache-2.0
//! A little markdown, enough for the documents this repository ships.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};

/// Headings bold, bullets kept, fenced code dimmed, inline code styled,
/// tables passed through as monospace, links reduced to their text.
pub fn render(text: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut in_code = false;
    for raw in text.lines() {
        if raw.trim_start().starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            lines.push(Line::from(Span::styled(
                format!("    {raw}"),
                Style::default().fg(Color::Cyan),
            )));
            continue;
        }
        if let Some(h) = raw.strip_prefix('#') {
            let title = h.trim_start_matches('#').trim();
            lines.push(Line::from(Span::styled(
                title.to_owned(),
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )));
            continue;
        }
        lines.push(inline(raw));
    }
    Text::from(lines)
}

/// `code` spans and `[text](url)` links, on one line.
fn inline(raw: &str) -> Line<'static> {
    let unlinked = strip_links(raw);
    let mut spans = Vec::new();
    let mut rest = unlinked.as_str();
    while let Some(start) = rest.find('`') {
        let (before, after) = rest.split_at(start);
        spans.push(Span::raw(before.to_owned()));
        let after = &after[1..];
        let Some(end) = after.find('`') else {
            spans.push(Span::raw(format!("`{after}")));
            rest = "";
            break;
        };
        spans.push(Span::styled(
            after[..end].to_owned(),
            Style::default().fg(Color::Cyan),
        ));
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        spans.push(Span::raw(rest.to_owned()));
    }
    Line::from(spans)
}

fn strip_links(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('[') {
        let Some(mid) = rest[start..].find("](") else {
            break;
        };
        let Some(end) = rest[start + mid..].find(')') else {
            break;
        };
        out.push_str(&rest[..start]);
        out.push_str(&rest[start + 1..start + mid]);
        rest = &rest[start + mid + end + 1..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_become_their_text_and_code_is_styled() {
        assert_eq!(
            strip_links("see [the docs](docs/x.md) now"),
            "see the docs now"
        );
        let t = render("# Title\n\n- a `war check` bullet\n```\ncode\n```\n[l](u)");
        assert_eq!(t.lines.len(), 5);
        assert_eq!(t.lines[0].spans[0].content, "Title");
        assert!(t.lines[2].spans.iter().any(|s| s.content == "war check"));
        assert_eq!(t.lines[3].spans[0].content, "    code");
        assert_eq!(t.lines[4].spans[0].content, "l");
    }
}
