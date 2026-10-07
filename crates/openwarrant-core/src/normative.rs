// SPDX-License-Identifier: Apache-2.0
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
/// Code fences and table rows are skipped. The legacy §3 "Normative language"
/// definition is skipped, but section 3 in another document is not special.
#[must_use]
pub fn normative_sentences(text: &str) -> Vec<NormativeSentence> {
    let mut out = Vec::new();
    let mut section = String::new();
    let mut heading = String::new();
    let mut in_fence = false;
    let mut paragraph: Vec<String> = Vec::new();
    let mut lead_in: Option<String> = None;

    fn push_sentences(out: &mut Vec<NormativeSentence>, section: &str, heading: &str, text: &str) {
        if section.is_empty()
            || (section == "3" && heading.eq_ignore_ascii_case("Normative language"))
        {
            return;
        }
        for raw in split_sentences(text) {
            let s = strip_emphasis(raw.trim());
            let s = s.as_str();
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
                && section_number(num)
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
                && section_number(num.trim_end_matches('.'))
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

/// A decimal section component may have one uppercase suffix, as in 8A.1.
fn section_number(number: &str) -> bool {
    number.split('.').all(|part| {
        let digits = part.trim_end_matches(|c: char| c.is_ascii_uppercase());
        !digits.is_empty()
            && digits.bytes().all(|b| b.is_ascii_digit())
            && part.len() - digits.len() <= 1
    })
}

/// Unsupported numbered headings whose body contains normative text.
/// A malformed section drops that text; a malformed subsection can instead
/// misattribute it to its parent. Drift checks alone cannot detect either.
/// Use the extractor's section grammar and its fence/table exclusions.
#[must_use]
pub fn dropped_sections(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending: Option<(String, bool)> = None;
    let mut in_fence = false;
    fn finish(pending: &mut Option<(String, bool)>, found: &mut Vec<String>) {
        if let Some((heading, true)) = pending.take() {
            found.push(heading);
        }
    }
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.starts_with('|') {
            continue;
        }
        let section = line.strip_prefix("## ");
        let subsection = line.strip_prefix("### ");
        if let Some(heading) = section.or(subsection) {
            let numbered = heading.starts_with(|c: char| c.is_ascii_digit());
            let supported = if section.is_some() {
                heading
                    .split_once(". ")
                    .is_some_and(|(number, _)| section_number(number))
            } else {
                heading.split_once(' ').is_some_and(|(number, _)| {
                    number.contains('.') && section_number(number.trim_end_matches('.'))
                })
            };
            // An unnumbered subsection retains its parent's section in the
            // extractor; do not lose the unsupported parent's pending body.
            if section.is_some() || numbered {
                finish(&mut pending, &mut found);
                if numbered && !supported {
                    pending = Some((heading.to_owned(), false));
                }
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        if let Some((_, has_rule)) = pending.as_mut() {
            *has_rule |= NormativeKeyword::of(line).is_some();
        }
    }
    finish(&mut pending, &mut found);
    found
}

/// Drop Markdown bold markers from a projected sentence.
///
/// The projection re-emits each sentence under its own `**KEYWORD**`, so the
/// source's emphasis is presentation this view does not need. Trimming only the
/// ends left the interior close behind whenever a label was written
/// `**RQ-001. The Fabric SHALL …**`, so the sentence read
/// `RQ-001. The Fabric SHALL ….** Background follows.` and a reader could not
/// tell whose asterisks those were. A `**` inside a code span is the source's
/// text (`docs/**/*.md`), not emphasis, and is kept.
fn strip_emphasis(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            in_code = !in_code;
        } else if c == '*' && !in_code && chars.peek() == Some(&'*') {
            chars.next();
            continue;
        }
        out.push(c);
    }
    out.trim().trim_matches('*').trim().to_owned()
}

/// Whether `text` ends inside a bold span: an odd number of `**` markers
/// outside code spans. A period there is part of a label such as
/// `**RQ-001. The Fabric SHALL …**`, not a sentence end.
fn bold_is_open(text: &str) -> bool {
    let mut open = false;
    let mut in_code = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '`' {
            in_code = !in_code;
        } else if c == '*' && !in_code && chars.peek() == Some(&'*') {
            chars.next();
            open = !open;
        }
    }
    open
}

/// Split prose into sentences at `. ` followed by an uppercase letter, `(`,
/// `\`` or a digit — not at every period, since `e.g.` and `§12.4` carry
/// them, and not inside a bold span. The text's end closes the last sentence.
fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        current.push(c);
        if c == '.'
            && !bold_is_open(&current)
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
    fn section_numbers_refuse_titles_and_malformed_components() {
        for valid in ["3", "8A", "104A", "8A.1", "47.2", "47.2B"] {
            assert!(section_number(valid), "{valid}");
        }
        for invalid in ["", "A", "8AA", "8a", ".8", "8.", "8..1", "Scope"] {
            assert!(!section_number(invalid), "{invalid}");
        }
    }

    #[test]
    fn lettered_sections_and_subsections_keep_exact_identity() {
        let text = "## 8A. Capture\nThe source SHALL retain identity.\n\n### 8A.1 Retry\nA retry SHALL recheck access.\n\n## 104A. Runtime\nThe index SHALL NOT persist text.\n";
        let got = normative_sentences(text);
        let actual: Vec<(&str, &str)> = got
            .iter()
            .map(|s| (s.section.as_str(), s.sentence.as_str()))
            .collect();
        assert_eq!(
            actual,
            vec![
                ("8A", "The source SHALL retain identity."),
                ("8A.1", "A retry SHALL recheck access."),
                ("104A", "The index SHALL NOT persist text."),
            ]
        );
    }

    #[test]
    fn section_three_is_not_always_a_language_definition() {
        let got = normative_sentences(
            "## 3. Architecture and component ownership\nThe runtime SHALL preserve source authority.\n",
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].section, "3");
        assert_eq!(
            got[0].sentence,
            "The runtime SHALL preserve source authority."
        );
    }

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
        // The year is not a list item (so nothing is prefixed or stripped);
        // ". T" after it is a sentence end, as it would be in any prose.
        let got = normative_sentences(text);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].sentence, "The tool SHALL still refuse.");
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

    /// A heading with no number still clears the section, so an appendix does
    /// not inherit the lettered section before it.
    #[test]
    fn an_unnumbered_heading_still_clears_the_section() {
        let text = "## 8A. Ingestion\n\nThe reader SHALL admit one item.\n\n## Appendix\n\nThe appendix SHALL be ignored here.\n";
        let got = normative_sentences(text);
        assert_eq!(got.len(), 1, "only the numbered section projects: {got:?}");
        assert_eq!(got[0].section, "8A");
    }

    /// A period inside a bold span does not end a sentence. Splitting there
    /// leaves the closing `**` leading the next sentence, which is how a
    /// requirement label written `**RQ-001. The Fabric SHALL ...**` came apart.
    #[test]
    fn a_period_inside_bold_does_not_split_the_sentence() {
        let text = "## 9. Requirements\n\n**RQ-001. The Fabric SHALL record its sources.** Background follows.\n";
        let got = normative_sentences(text);
        assert_eq!(got.len(), 1, "the bold span is one sentence: {got:?}");
        assert!(
            !got[0].sentence.contains('*'),
            "an emphasis marker survived into the sentence: {:?}",
            got[0].sentence
        );
        assert!(got[0].sentence.starts_with("RQ-001."), "{got:?}");
    }

    /// Bold inside a sentence is the source's presentation; the projection
    /// adds its own, so the markers go, wherever they sit.
    #[test]
    fn interior_emphasis_markers_are_stripped() {
        let got = normative_sentences(
            "## 9. Requirements\n\nThe Fabric SHALL **always** record its sources.\n",
        );
        assert_eq!(got.len(), 1, "{got:?}");
        assert_eq!(
            got[0].sentence,
            "The Fabric SHALL always record its sources."
        );
    }

    /// The refusal beside the two above: a `**` inside a code span is quoted
    /// text, so it is neither stripped nor counted as an open bold span.
    #[test]
    fn a_glob_in_a_code_span_is_kept_and_does_not_hold_a_sentence_open() {
        let got = normative_sentences(
            "## 9. Scope\n\nThe tool SHALL read `docs/**/*.md` only. It SHALL NOT recurse.\n",
        );
        let sentences: Vec<&str> = got.iter().map(|s| s.sentence.as_str()).collect();
        assert_eq!(
            sentences,
            vec![
                "The tool SHALL read `docs/**/*.md` only.",
                "It SHALL NOT recurse."
            ]
        );
    }
}
