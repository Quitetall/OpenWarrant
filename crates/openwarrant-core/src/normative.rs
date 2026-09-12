// SPDX-License-Identifier: AGPL-3.0-or-later
//! The SAS's normative projection (slice E1): every SHALL / SHALL NOT /
//! MUST / MUST NOT / SHOULD / SHOULD NOT / MAY sentence of the document, with
//! the section it sits in. Agents read this instead of the whole document:
//! the sentences are what binds; the rest is why.
//!
//! Its own module, beside `sas.rs`, because that file is a pinned deliverable
//! of resolved Warrants and this is a new capability, not a repair.

/// §3's keywords, most binding first.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormativeKeyword {
    ShallNot,
    Shall,
    MustNot,
    Must,
    ShouldNot,
    Should,
    May,
}

impl NormativeKeyword {
    /// The strongest keyword a sentence carries, if any. Whole uppercase words
    /// only: "shall" in prose and "Mayor" are not keywords.
    #[must_use]
    pub fn of(sentence: &str) -> Option<Self> {
        let words: Vec<&str> = sentence
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect();
        let has = |a: &str, b: Option<&str>| {
            words.windows(2).any(|w| w[0] == a && b == Some(w[1]))
                || (b.is_none() && words.contains(&a))
        };
        if has("SHALL", Some("NOT")) {
            Some(Self::ShallNot)
        } else if has("SHALL", None) {
            Some(Self::Shall)
        } else if has("MUST", Some("NOT")) {
            Some(Self::MustNot)
        } else if has("MUST", None) {
            Some(Self::Must)
        } else if has("SHOULD", Some("NOT")) {
            Some(Self::ShouldNot)
        } else if has("SHOULD", None) {
            Some(Self::Should)
        } else if has("MAY", None) {
            Some(Self::May)
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ShallNot => "SHALL NOT",
            Self::Shall => "SHALL",
            Self::MustNot => "MUST NOT",
            Self::Must => "MUST",
            Self::ShouldNot => "SHOULD NOT",
            Self::Should => "SHOULD",
            Self::May => "MAY",
        }
    }
}

/// One binding sentence and where it lives.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NormativeSentence {
    /// `47` or `47.2` — the numbered heading the sentence sits under.
    pub section: String,
    pub heading: String,
    pub keyword: NormativeKeyword,
    pub sentence: String,
}

/// Every normative sentence of a SAS document, in document order.
///
/// Sentences split on `. ` and line ends within a paragraph. A lead-in that
/// ends with `:` and carries a keyword ("The compiler SHALL:") makes each
/// bullet under it one sentence, prefixed with the lead-in (§47.2 style).
/// Code fences and table rows are skipped; §3 itself (which defines the
/// words) is skipped so the definitions do not project as requirements.
#[must_use]
pub fn normative_sentences(text: &str) -> Vec<NormativeSentence> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut heading = String::new();
    let mut in_fence = false;
    let mut paragraph: Vec<String> = Vec::new();
    let mut lead_in: Option<String> = None;

    fn push_sentences(out: &mut Vec<NormativeSentence>, section: &str, heading: &str, text: &str) {
        if section.is_empty() || section == "3" {
            return;
        }
        for raw in split_sentences(text) {
            let s = raw.trim().trim_matches(|c| c == '*').trim();
            if s.is_empty() {
                continue;
            }
            if let Some(keyword) = NormativeKeyword::of(s) {
                out.push(NormativeSentence {
                    section: section.to_owned(),
                    heading: heading.to_owned(),
                    keyword,
                    sentence: s.to_owned(),
                });
            }
        }
    }

    let flush = |paragraph: &mut Vec<String>,
                 out: &mut Vec<NormativeSentence>,
                 section: &str,
                 heading: &str| {
        if paragraph.is_empty() {
            return;
        }
        let joined = paragraph.join(" ");
        paragraph.clear();
        push_sentences(out, section, heading, &joined);
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("## ") {
            flush(&mut paragraph, &mut out, &section, &heading);
            lead_in = None;
            if let Some((num, title)) = rest.split_once(". ")
                && num.bytes().all(|b| b.is_ascii_digit())
            {
                section = num.to_owned();
                heading = title.trim().to_owned();
                continue;
            }
            section.clear();
            heading = rest.trim().to_owned();
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("### ") {
            flush(&mut paragraph, &mut out, &section, &heading);
            lead_in = None;
            if let Some((num, title)) = rest.split_once(' ')
                && num.bytes().all(|b| b.is_ascii_digit() || b == b'.')
                && num.contains('.')
            {
                section = num.trim_end_matches('.').to_owned();
                heading = title.trim().to_owned();
                continue;
            }
            heading = rest.trim().to_owned();
            continue;
        }
        if trimmed.starts_with('#') {
            flush(&mut paragraph, &mut out, &section, &heading);
            lead_in = None;
            continue;
        }
        if trimmed.is_empty() {
            flush(&mut paragraph, &mut out, &section, &heading);
            // A blank line after a lead-in keeps it: "SHALL:\n\n- bullet" is
            // the document's own layout.
            continue;
        }
        if trimmed.starts_with('|') {
            flush(&mut paragraph, &mut out, &section, &heading);
            continue;
        }
        // A list item: `- `, `* `, or `N. ` (the document's lead-ins are
        // followed by numbered lists as often as by dashes — "The CLI SHALL:"
        // then "1. …", "2. …").
        let numbered = trimmed
            .split_once(". ")
            // At most three digits: "2026. The next…" is a year in prose,
            // not item two thousand and twenty-six.
            .filter(|(n, _)| (1..=3).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
            .map(|(_, rest)| rest);
        if let Some(bullet) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or(numbered)
        {
            flush(&mut paragraph, &mut out, &section, &heading);
            let bullet = bullet.trim().trim_end_matches([';', '.', ',']).trim();
            match &lead_in {
                Some(lead) => {
                    push_sentences(&mut out, &section, &heading, &format!("{lead} {bullet}."))
                }
                None => push_sentences(&mut out, &section, &heading, &format!("{bullet}.")),
            }
            continue;
        }
        // Prose. A lead-in ending with `:` that carries a keyword governs the
        // bullets that follow; any other prose line clears it.
        if trimmed.ends_with(':') && NormativeKeyword::of(trimmed).is_some() {
            flush(&mut paragraph, &mut out, &section, &heading);
            lead_in = Some(trimmed.trim_end_matches(':').trim().to_owned());
            continue;
        }
        lead_in = None;
        paragraph.push(trimmed.to_owned());
    }
    flush(&mut paragraph, &mut out, &section, &heading);
    out
}

/// Split prose into sentences at `. ` followed by an uppercase letter, `(`,
/// `\`` or a digit — not at every period, since `e.g.` and `§12.4` carry
/// them. The text's end closes the last sentence.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        current.push(c);
        if c == '.'
            && chars.get(i + 1) == Some(&' ')
            && chars.get(i + 2).is_some_and(|n| {
                n.is_ascii_uppercase() || matches!(n, '(' | '`' | '*') || n.is_ascii_digit()
            })
            && !current.trim_end().ends_with("e.g.")
            && !current.trim_end().ends_with("i.e.")
        {
            out.push(std::mem::take(&mut current));
            i += 2;
            continue;
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod normative_tests {
    use super::*;

    #[test]
    fn keywords_are_whole_uppercase_words_strongest_first() {
        assert_eq!(
            NormativeKeyword::of("The tool SHALL NOT sign."),
            Some(NormativeKeyword::ShallNot)
        );
        assert_eq!(
            NormativeKeyword::of("It SHALL refuse; it MAY warn."),
            Some(NormativeKeyword::Shall)
        );
        assert_eq!(
            NormativeKeyword::of("A caller MAY retry."),
            Some(NormativeKeyword::May)
        );
        assert_eq!(NormativeKeyword::of("the mayor shall decide"), None);
        assert_eq!(
            NormativeKeyword::of("SHOULD NOT be silent"),
            Some(NormativeKeyword::ShouldNot)
        );
    }

    #[test]
    fn a_year_at_the_start_of_a_sentence_is_not_a_list_item() {
        let text = "## 9. Scope\n\n2026. The tool SHALL still refuse.\n";
        let got = normative_sentences(text);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].sentence, "2026. The tool SHALL still refuse.");
    }

    #[test]
    fn sentences_are_projected_with_their_section_and_lead_ins_prefix_their_bullets() {
        let text = "## 3. Normative language\n\nThe term SHALL states a requirement.\n\n## 47. Dispatch model\n\nProse without a keyword. The compiler SHALL record the digest. It may not.\n\n### 47.2 Dispatch compilation\n\nThe compiler SHALL:\n\n- select only stage-relevant context;\n- record omitted subgraphs.\n\n```\nSHALL inside a fence is code\n```\n\n| a | SHALL in a table |\n|---|---|\n\nA caller MAY retry (e.g. after a timeout). Nothing else.\n\nThe CLI SHALL:\n\n1. refuse an agent signature;\n2. say why.\n";
        let got = normative_sentences(text);
        let as_pairs: Vec<(String, String, String)> = got
            .iter()
            .map(|s| {
                (
                    s.section.clone(),
                    s.keyword.as_str().to_owned(),
                    s.sentence.clone(),
                )
            })
            .collect();
        assert_eq!(
            as_pairs,
            vec![
                (
                    "47".to_owned(),
                    "SHALL".to_owned(),
                    "The compiler SHALL record the digest.".to_owned()
                ),
                (
                    "47.2".to_owned(),
                    "SHALL".to_owned(),
                    "The compiler SHALL select only stage-relevant context.".to_owned()
                ),
                (
                    "47.2".to_owned(),
                    "SHALL".to_owned(),
                    "The compiler SHALL record omitted subgraphs.".to_owned()
                ),
                (
                    "47.2".to_owned(),
                    "MAY".to_owned(),
                    "A caller MAY retry (e.g. after a timeout).".to_owned()
                ),
                (
                    "47.2".to_owned(),
                    "SHALL".to_owned(),
                    "The CLI SHALL refuse an agent signature.".to_owned()
                ),
                (
                    "47.2".to_owned(),
                    "SHALL".to_owned(),
                    "The CLI SHALL say why.".to_owned()
                ),
            ]
        );
        assert_eq!(got[1].heading, "Dispatch compilation");
        // §3 (the definitions) and the fence and the table row project nothing.
        assert!(got.iter().all(|s| s.section != "3"));
    }
}
