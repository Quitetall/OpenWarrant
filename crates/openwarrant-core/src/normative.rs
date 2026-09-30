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
                && num.starts_with(|c: char| c.is_ascii_digit())
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

/// Numbered headings whose sentences the projection would drop.
///
/// Drift-checking proves a projection is REPRODUCIBLE, not that it is COMPLETE:
/// an extractor that drops the same sentences every time compiles identically
/// every time, and the check passes. This is the complement. It reads the
/// document, finds every `## <number>. Title` heading the section parser could
/// not label, and reports the ones whose body carries a normative keyword —
/// text that belongs in the projection and is not there.
///
/// Knowledge Fabric lost 28 of 162 requirements to exactly this, in five
/// lettered sections, for weeks. The projection's own header said 139 sentences
/// and 139 sentences were present; the absence was invisible from inside it.
#[must_use]
pub fn dropped_sections(text: &str) -> Vec<String> {
    let mut dropped = Vec::new();
    let mut current: Option<(String, bool)> = None;
    let mut in_fence = false;
    let finish = |current: Option<(String, bool)>, dropped: &mut Vec<String>| {
        if let Some((heading, has_keyword)) = current
            && has_keyword
        {
            dropped.push(heading);
        }
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
            finish(current.take(), &mut dropped);
            // Only a heading that LOOKS numbered is a candidate: `## Appendix`
            // is prose and projects nothing by design.
            let looks_numbered = rest.starts_with(|c: char| c.is_ascii_digit());
            let parses = rest
                .split_once(". ")
                .is_some_and(|(num, _)| section_number(num));
            if looks_numbered && !parses {
                current = Some((rest.trim().to_owned(), false));
            }
            continue;
        }
        if let Some((_, has_keyword)) = current.as_mut()
            && NormativeKeyword::of(trimmed).is_some()
        {
            *has_keyword = true;
        }
    }
    finish(current.take(), &mut dropped);
    dropped
}

/// Drop Markdown emphasis markers from a projected sentence.
///
/// The projection re-emits each sentence under its own `**KEYWORD**`, so the
/// source's emphasis is presentation this view does not need. Trimming only the
/// ends — which is what this did — left the interior close behind whenever a
/// label was written `**RQ-001. The Fabric SHALL …**`, so the sentence read
/// `RQ-001. The Fabric SHALL ….** Background follows.` A reader cannot tell
/// whether those two asterisks are the source's or the tool's.
fn strip_emphasis(text: &str) -> String {
    text.replace("**", "")
        .trim()
        .trim_matches('*')
        .trim()
        .to_owned()
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
            && current.matches("**").count().is_multiple_of(2)
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

    /// §8A is a section. A document that has shipped §8 appends §8A rather than
    /// renumbering, and the old rule — every byte a digit — did not merely drop
    /// the label: it cleared the section, and every sentence underneath was
    /// dropped with it. Knowledge Fabric lost 28 of 162 requirements this way.
    #[test]
    fn a_lettered_section_is_a_section_and_keeps_its_sentences() {
        let text = "## 8. Ingestion\n\nThe reader SHALL admit one item.\n\n                    ## 8A. Ingestion, continued\n\n                    The reader SHALL NOT provide recursive synchronisation.\n\n                    ### 8A.1 Containers\n\nAn external container MAY be named.\n";
        let got = normative_sentences(text);
        let sections: Vec<&str> = got.iter().map(|s| s.section.as_str()).collect();
        assert!(
            sections.contains(&"8A"),
            "§8A must survive, got sections {sections:?}"
        );
        assert!(
            sections.contains(&"8A.1"),
            "a subsection of a lettered section too, got {sections:?}"
        );
        assert_eq!(got.len(), 3, "nothing is dropped: {got:?}");
        assert!(
            got.iter()
                .any(|s| s.sentence.contains("recursive synchronisation")),
            "the sentence itself, not just its label"
        );
    }

    /// A heading with no number still clears the section — that behaviour is
    /// deliberate and unchanged, so an appendix does not inherit §8A.
    #[test]
    fn an_unnumbered_heading_still_clears_the_section() {
        let text = "## 8A. Ingestion\n\nThe reader SHALL admit one item.\n\n                    ## Appendix\n\nThe appendix SHALL be ignored here.\n";
        let got = normative_sentences(text);
        assert_eq!(got.len(), 1, "only the numbered section projects: {got:?}");
        assert_eq!(got[0].section, "8A");
    }

    /// A requirement written as `**KF-SAS-RQ-001.** The Fabric SHALL …` is one
    /// sentence. Splitting at the period inside the bold left the closing `**`
    /// leading the next sentence.
    /// The completeness check the drift check cannot be: a heading shape the
    /// parser does not know, carrying normative text, is reported by name
    /// instead of vanishing.
    #[test]
    fn a_heading_shape_the_parser_cannot_label_is_reported() {
        let text = "## 8. Ingestion\n\nThe reader SHALL admit one item.\n\n\
                    ## 8-bis. Later thoughts\n\nThe reader SHALL NOT recurse.\n\n\
                    ## Appendix\n\nNothing normative lives here.\n";
        assert_eq!(dropped_sections(text), vec!["8-bis. Later thoughts"]);
        // §8A parses now, so it is not reported; an appendix never was.
        let fine = "## 8A. Ingestion\n\nThe reader SHALL admit one item.\n";
        assert!(dropped_sections(fine).is_empty());
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
}
