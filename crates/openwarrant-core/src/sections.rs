// SPDX-License-Identifier: AGPL-3.0-or-later
//! Markdown sections by heading, for context selection (slice C1).
//!
//! A section is a heading line and everything up to the next heading of the
//! same or a higher level. Headings are matched by their text after the
//! hashes, case-sensitively, with surrounding whitespace trimmed.

/// Every heading in `text`, as `(level, title)`, in order.
#[must_use]
pub fn headings(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let level = t.bytes().take_while(|b| *b == b'#').count();
        if level > 0 && t.as_bytes().get(level) == Some(&b' ') {
            out.push((level, t[level..].trim().to_owned()));
        }
    }
    out
}

/// The section titled `heading` (without hashes), if it exists: the heading
/// line and its body up to the next heading of the same or a higher level.
#[must_use]
pub fn find(text: &str, heading: &str) -> Option<String> {
    let want = heading.trim().trim_start_matches('#').trim();
    let mut out: Option<String> = None;
    let mut level = 0usize;
    let mut in_fence = false;
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with("```") {
            in_fence = !in_fence;
        }
        let this_level = if in_fence || t.starts_with("```") {
            0
        } else {
            let l = t.bytes().take_while(|b| *b == b'#').count();
            if l > 0 && t.as_bytes().get(l) == Some(&b' ') {
                l
            } else {
                0
            }
        };
        match &mut out {
            None => {
                if this_level > 0 && t[this_level..].trim() == want {
                    level = this_level;
                    out = Some(format!("{line}\n"));
                }
            }
            Some(buf) => {
                if this_level > 0 && this_level <= level {
                    break;
                }
                buf.push_str(line);
                buf.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_section_runs_to_the_next_heading_of_its_level_and_fences_do_not_count() {
        let text = "# Work Order\n\n## Deliverables\n\n1. a\n2. b\n\n### Detail\n\nmore\n\n```\n## not a heading\n```\n\n## Frozen Surfaces\n\nnone\n";
        let s = find(text, "Deliverables").unwrap();
        assert!(s.starts_with("## Deliverables\n"));
        assert!(s.contains("### Detail") && s.contains("## not a heading"));
        assert!(!s.contains("Frozen"));
        assert_eq!(
            find(text, "## Frozen Surfaces").unwrap(),
            "## Frozen Surfaces\n\nnone\n"
        );
        assert!(find(text, "Rollback").is_none());
        let hs = headings(text);
        assert_eq!(
            hs.iter().map(|(_, t)| t.as_str()).collect::<Vec<_>>(),
            vec!["Work Order", "Deliverables", "Detail", "Frozen Surfaces"]
        );
    }
}
