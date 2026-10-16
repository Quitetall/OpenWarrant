// SPDX-License-Identifier: Apache-2.0
//! The compiler over hand-built graphs: each claim beside the input it
//! refuses.

use super::*;

fn unit(id: &str, kind: &str, source: &str, line: usize, text: &str) -> Unit {
    Unit {
        id: id.to_owned(),
        kind: kind.to_owned(),
        title: Some(format!("title of {id}")),
        document: id.split_once('/').map(|(d, _)| d.to_owned()),
        source: source.to_owned(),
        line,
        revision: format!(
            "sha256:{}",
            openwarrant_compiler::sha256_hex(text.as_bytes())
        ),
        text: Some(text.to_owned()),
        ..Unit::default()
    }
}

fn edge(from: &str, kind: &str, to: &str) -> Edge {
    Edge {
        from: from.to_owned(),
        kind: kind.to_owned(),
        to: to.to_owned(),
    }
}

/// A Warrant `W-1` with two sections and an obligation, implementing a
/// requirement a constraint constrains and an obligation evaluates; a
/// ticket `t-ab` with an item that implements the requirement.
fn sample() -> Graph {
    let mut g = Graph {
        name: "demo".to_owned(),
        ..Graph::default()
    };
    let mut w = unit(
        "W-1",
        "warrant",
        "docs/warrants/W-1.md",
        1,
        "---\nalias: W-1\n---\n# W\n",
    );
    w.work = true;
    w.document = None;
    w.summary = Some("What W is for".to_owned());
    w.uuid = Some("01a0d342-ee5e-7b71-9623-f2d8cd5acd4e".to_owned());
    w.paths = vec!["crates/**/bundle.rs".to_owned()];
    w.state = Some("draft".to_owned());
    g.add(w);
    g.add(unit(
        "W-1/S-intent",
        "section",
        "docs/warrants/W-1.md",
        6,
        "## Intent\nwhy\n",
    ));
    g.add(unit(
        "W-1/S-basis",
        "section",
        "docs/warrants/W-1.md",
        9,
        "## Basis\nsources\n",
    ));
    g.add(unit(
        "W-1/OBL-001",
        "obligation",
        "docs/warrants/W-1.md",
        12,
        "## OBL-001 · obligation — it holds\nscope: x\n",
    ));
    g.add(unit(
        "REQ-a",
        "requirement",
        "docs/records/a/10.md",
        3,
        "## REQ-a · requirement — a\n",
    ));
    g.add(unit(
        "CON-a",
        "constraint",
        "docs/records/a/10.md",
        9,
        "## CON-a · constraint — c\n",
    ));
    g.add(unit(
        "OBL-a",
        "obligation",
        "docs/records/a/10.md",
        15,
        "## OBL-a · obligation — o\n",
    ));
    g.add(unit(
        "IF-a",
        "interface",
        "docs/records/a/20.md",
        1,
        "## IF-a · interface — i\n",
    ));
    let mut t = unit(
        "t-ab",
        "ticket",
        "docs/tickets/t-ab.md",
        1,
        "---\nalias: t-ab\n---\n# T\n",
    );
    t.work = true;
    t.document = None;
    g.add(t);
    g.add(unit(
        "t-ab/S-intent",
        "section",
        "docs/tickets/t-ab.md",
        5,
        "## Intent\nticket why\n",
    ));
    g.add(unit(
        "t-ab/i-9c01",
        "item",
        "docs/tickets/t-ab.md",
        9,
        "- [ ] do it (i-9c01)\n",
    ));
    g.edges = vec![
        edge("W-1", "implements", "REQ-a"),
        edge("CON-a", "constrains", "REQ-a"),
        edge("OBL-a", "evaluates", "REQ-a"),
        edge("IF-a", "implements", "REQ-a"),
        edge("t-ab/i-9c01", "implements", "REQ-a"),
        edge("W-1/OBL-001", "part_of", "W-1"),
        edge("W-1", "depends_on", "GONE-1"),
    ];
    g.edges.sort();
    g
}

fn ids(p: &Packet) -> Vec<&str> {
    p.records.iter().map(|r| r.id.as_str()).collect()
}

#[test]
fn the_same_graph_and_arguments_give_the_same_bytes_and_a_changed_record_moves_the_digest() {
    let g = sample();
    let a = compile(&g, "W-1", &Options::default()).unwrap();
    let b = compile(&g, "W-1", &Options::default()).unwrap();
    assert_eq!(a.markdown, b.markdown);
    assert_eq!(a, b);
    // Refusal: a changed record is not the same input, and the selection
    // digest says so.
    let mut g2 = g.clone();
    g2.units.get_mut("REQ-a").unwrap().revision = "sha256:00".to_owned();
    let c = compile(&g2, "W-1", &Options::default()).unwrap();
    assert_ne!(a.selection_digest, c.selection_digest);
}

#[test]
fn a_work_document_target_gives_its_units_at_distance_zero_then_the_walk_in_type_rank_order() {
    let p = compile(&sample(), "W-1", &Options::default()).unwrap();
    assert_eq!(
        ids(&p),
        [
            // distance 0: warrant, sections, obligation (rank order)
            "W-1",
            "W-1/S-intent",
            "W-1/S-basis",
            "W-1/OBL-001",
            // distance 1: the requirement it implements
            "REQ-a",
            // distance 2: constraint, interface, obligation by rank
            "CON-a",
            "IF-a",
            "OBL-a",
        ]
    );
    let by: BTreeMap<&str, &PacketRecord> = p.records.iter().map(|r| (r.id.as_str(), r)).collect();
    assert_eq!(by["REQ-a"].distance, 1);
    assert_eq!(
        by["REQ-a"].via,
        Some(Via {
            step: "out:implements".to_owned(),
            from: "W-1".to_owned()
        })
    );
    assert_eq!(by["CON-a"].via.as_ref().unwrap().step, "in:constrains");
    assert_eq!(by["IF-a"].via.as_ref().unwrap().step, "in:implements");
    // An item implementing the requirement is reached by no step: plain
    // `in:implements` is followed only from an interface.
    assert!(!by.contains_key("t-ab/i-9c01"));
    // The unknown target is skipped and reported, not dropped silently.
    assert!(
        p.diagnostics
            .iter()
            .any(|d| d.rule == RULE_TARGET_UNREAD && d.message.contains("GONE-1"))
    );
    assert!(p.markdown.starts_with(
        "# Context: W-1\n<!-- oh.war/context/v1 · budget 16000 · depth 3 · preset agent (walk core) · selection sha256:"
    ));
    assert!(p.markdown.contains(
        "\n## Index\n[F] W-1 · warrant · draft · title of W-1 · docs/warrants/W-1.md:1\n"
    ));
    assert!(
        p.markdown
            .contains("\n## Records\n> W-1 · warrant · docs/warrants/W-1.md:1 · sha256:")
    );
    assert_eq!(p.bytes, p.markdown.len());
}

#[test]
fn in_implements_is_followed_only_from_an_interface() {
    let mut g = sample();
    let p = compile(&g, "REQ-a", &Options::default()).unwrap();
    let got = ids(&p);
    assert!(got.contains(&"IF-a"));
    // Refusal: the item implements REQ-a too, but is no interface.
    assert!(!got.contains(&"t-ab/i-9c01"), "{got:?}");
    // Made an interface, it is followed.
    g.units.get_mut("t-ab/i-9c01").unwrap().kind = "interface".to_owned();
    let p = compile(&g, "REQ-a", &Options::default()).unwrap();
    assert!(ids(&p).contains(&"t-ab/i-9c01"));
}

#[test]
fn depth_bounds_the_walk() {
    let p = compile(
        &sample(),
        "W-1",
        &Options {
            depth: 1,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(ids(&p).contains(&"REQ-a"));
    assert!(!ids(&p).contains(&"CON-a"));
    let p0 = compile(
        &sample(),
        "REQ-a",
        &Options {
            depth: 0,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(ids(&p0), ["REQ-a"]);
}

#[test]
fn an_item_target_brings_its_documents_intent_at_distance_one() {
    let p = compile(&sample(), "t-ab/i-9c01", &Options::default()).unwrap();
    let by: BTreeMap<&str, &PacketRecord> = p.records.iter().map(|r| (r.id.as_str(), r)).collect();
    assert_eq!(by["t-ab/i-9c01"].distance, 0);
    assert_eq!(by["t-ab/S-intent"].distance, 1);
    assert_eq!(by["REQ-a"].distance, 1);
    // The rest of the ticket is not: an item is not its whole document.
    assert!(!by.contains_key("t-ab"));
    // The bare item id resolves when only one document has it.
    let bare = compile(&sample(), "i-9c01", &Options::default()).unwrap();
    assert_eq!(bare.records, p.records);
}

#[test]
fn a_uuid_and_paths_resolve_and_an_unknown_target_is_refused_by_rule() {
    let g = sample();
    let by_uuid = compile(
        &g,
        "war://01a0d342-ee5e-7b71-9623-f2d8cd5acd4e",
        &Options::default(),
    )
    .unwrap();
    assert_eq!(by_uuid.records[0].id, "W-1");
    // A document's path: its work document.
    let doc = compile(&g, "./docs/tickets/t-ab.md", &Options::default()).unwrap();
    assert_eq!(doc.records[0].id, "t-ab");
    assert_eq!(doc.target, "docs/tickets/t-ab.md");
    // A path of a document with no work document: all its records.
    let recs = compile(
        &g,
        "docs/records/a/10.md",
        &Options {
            depth: 0,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(ids(&recs), ["REQ-a", "CON-a", "OBL-a"]);
    // Any other path: the work documents whose globs match it.
    let code = compile(
        &g,
        "crates/openwarrant-cli/src/bundle.rs",
        &Options::default(),
    )
    .unwrap();
    assert_eq!(code.records[0].id, "W-1");
    // Refusals: a path no glob matches, an id nothing has.
    for t in [
        "crates/openwarrant-cli/src/lib.rs",
        "NOPE-1",
        "war://00000000-0000-7000-8000-000000000000",
        "i-ffff",
    ] {
        let r = compile(&g, t, &Options::default()).unwrap_err();
        assert_eq!(r.rule, RULE_TARGET_UNKNOWN, "{t}");
    }
}

#[test]
fn roots_that_do_not_fit_are_refused_with_the_bytes_they_need() {
    let g = sample();
    let ok = compile(&g, "REQ-a", &Options::default()).unwrap();
    // The index alone over budget.
    let r = compile(
        &g,
        "REQ-a",
        &Options {
            budget: Some(50),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert_eq!(r.rule, RULE_OVER_BUDGET);
    assert!(r.needed.unwrap() > 50);
    // The index fits but the root's text does not.
    let header_and_index = ok.markdown.find("> REQ-a").unwrap();
    let r = compile(
        &g,
        "REQ-a",
        &Options {
            budget: Some(header_and_index + 3),
            ..Options::default()
        },
    )
    .unwrap_err();
    assert_eq!(r.rule, RULE_OVER_BUDGET);
    assert!(r.message.contains("need"), "{}", r.message);
    // The least budget the root fits in is admitted, the root whole and
    // the rest by summary; one byte less is refused.
    let least = (0..5000)
        .find(|b| {
            compile(
                &g,
                "REQ-a",
                &Options {
                    budget: Some(*b),
                    ..Options::default()
                },
            )
            .is_ok()
        })
        .unwrap();
    let p = compile(
        &g,
        "REQ-a",
        &Options {
            budget: Some(least),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(p.records[0].tier, "full");
    assert!(p.records[1..].iter().all(|r| r.tier == "summary"));
    assert_eq!(p.bytes, least);
    assert!(p.markdown.contains("## REQ-a · requirement — a\n"));
}

#[test]
fn a_partial_packet_marks_the_rest_s_and_never_cuts_a_record() {
    let mut g = sample();
    // Make the basis large so it is the first not to fit.
    let big = format!("## Basis\n{}\n", "x".repeat(400));
    let u = g.units.get_mut("W-1/S-basis").unwrap();
    u.text = Some(big.clone());
    let full = compile(&g, "W-1", &Options::default()).unwrap();
    let cut_at = full.markdown.find("> W-1/S-basis").unwrap() + 100;
    let p = compile(
        &g,
        "W-1",
        &Options {
            budget: Some(cut_at),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(p.bytes <= cut_at);
    let tiers: Vec<(&str, &str)> = p
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.tier.as_str()))
        .collect();
    assert_eq!(tiers[0], ("W-1", "full"));
    assert_eq!(tiers[1], ("W-1/S-intent", "full"));
    // The basis would not fit: it and EVERY later record are summaries,
    // even the small ones that would.
    assert!(tiers[2..].iter().all(|(_, t)| *t == "summary"), "{tiers:?}");
    assert!(p.markdown.contains("[S] W-1/S-basis · section"));
    assert!(p.markdown.contains("[S] W-1/OBL-001 · obligation"));
    // Never cut: every full record's text appears whole; no summary's does.
    for r in &p.records {
        let text = g.units[&r.id].text.as_deref().unwrap();
        if r.tier == "full" {
            assert!(p.markdown.contains(text));
        } else {
            assert!(!p.markdown.contains(&format!("> {} ·", r.id)));
        }
    }
    assert!(!p.markdown.contains(&"x".repeat(50)));
}

#[test]
fn strict_refuses_what_partial_output_would_give_by_summary() {
    let g = sample();
    let full = compile(&g, "W-1", &Options::default()).unwrap();
    let strict_ok = compile(
        &g,
        "W-1",
        &Options {
            strict: true,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(strict_ok.markdown, full.markdown);
    let whole = (0..10_000)
        .find(|b| {
            compile(
                &g,
                "W-1",
                &Options {
                    budget: Some(*b),
                    ..Options::default()
                },
            )
            .is_ok_and(|p| p.records.iter().all(|r| r.tier == "full"))
        })
        .unwrap();
    let tight = whole - 1;
    let partial = compile(
        &g,
        "W-1",
        &Options {
            budget: Some(tight),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(partial.records.iter().any(|r| r.tier == "summary"));
    let r = compile(
        &g,
        "W-1",
        &Options {
            budget: Some(tight),
            strict: true,
            ..Options::default()
        },
    )
    .unwrap_err();
    assert_eq!(r.rule, RULE_OVER_BUDGET);
    assert_eq!(r.needed, Some(whole));
    assert!(
        compile(
            &g,
            "W-1",
            &Options {
                budget: Some(whole),
                strict: true,
                ..Options::default()
            },
        )
        .is_ok()
    );
}

#[test]
fn a_record_whose_span_is_unknown_is_a_summary_and_does_not_stop_the_fill() {
    let mut g = sample();
    g.units.get_mut("REQ-a").unwrap().text = None;
    let p = compile(&g, "W-1", &Options::default()).unwrap();
    let by: BTreeMap<&str, &str> = p
        .records
        .iter()
        .map(|r| (r.id.as_str(), r.tier.as_str()))
        .collect();
    assert_eq!(by["REQ-a"], "summary");
    assert_eq!(by["CON-a"], "full");
    assert!(
        compile(
            &g,
            "W-1",
            &Options {
                strict: true,
                ..Options::default()
            }
        )
        .is_err()
    );
}

#[test]
fn tokens_are_four_bytes() {
    assert_eq!(Options::budget_of(None, Some(1000)), 4000);
    assert_eq!(Options::budget_of(Some(10), None), 10);
    assert_eq!(Options::budget_of(None, None), DEFAULT_BUDGET);
}

#[test]
fn the_index_groups_by_source_with_the_documents_summary() {
    let idx = index(&sample());
    assert!(idx.starts_with("# demo\n\n> OpenWarrant index (openwarrant/1.0). Generated; do not edit. Read any line's record with `war context <id>`.\n\n## docs/records/a/10.md\n> -\n- REQ-a · requirement · - · title of REQ-a · docs/records/a/10.md:3\n"));
    // A work document is its own record only; its units are not repeated.
    assert!(idx.contains("\n## docs/warrants/W-1.md\n> What W is for\n- W-1 · warrant · draft · title of W-1 · docs/warrants/W-1.md:1\n"));
    assert!(!idx.contains("W-1/S-intent"), "{idx}");
    assert!(!idx.contains("t-ab/i-9c01"), "{idx}");
    // The ticket has no summary: its title stands in.
    assert!(idx.contains("\n## docs/tickets/t-ab.md\n> title of t-ab\n"));
    assert_eq!(idx, index(&sample()));
}

#[test]
fn shapes() {
    assert_eq!(slug("Work order"), "work-order");
    assert_eq!(slug("  Risks & Rollback!! "), "risks-rollback");
    assert_eq!(slug("§§"), "");
    assert!(is_item_id("i-9c01"));
    assert!(!is_item_id("i-9C01") && !is_item_id("i-") && !is_item_id("t-9c01"));
    assert!(glob_match(
        "crates/**/bundle.rs",
        "crates/openwarrant-cli/src/bundle.rs"
    ));
    assert!(glob_match("crates/**/bundle.rs", "crates/bundle.rs"));
    assert!(glob_match("docs/*.md", "docs/a.md"));
    assert!(!glob_match("docs/*.md", "docs/x/a.md"));
    assert!(glob_match("conformance/", "conformance/plants.d/1.sh"));
    assert!(!glob_match("conformance/", "conformance2/x"));
    assert_eq!(type_rank("ticket"), 0);
    assert!(type_rank("section") < type_rank("item"));
    assert!(type_rank("instruction") < type_rank("deliverable"));
    let long = "é".repeat(80);
    let t = cut_title(&long);
    assert!(t.ends_with('…') && t.len() <= 120 + '…'.len_utf8());
    assert_eq!(cut_title("short"), "short");
}

/// A ticket whose checklist section holds its item: the section's span
/// contains the item's line.
fn checklist() -> Graph {
    let mut g = Graph::default();
    let src = "docs/tickets/t-cd.md";
    let text = "---\nalias: t-cd\n---\n# T\n\n## Checklist\n- [ ] one thing to do (i-aa01)\n- [ ] another (i-aa02)\n";
    let at = |s: &str| text.find(s).unwrap();
    let mut t = unit("t-cd", "ticket", src, 1, &text[..at("## Checklist")]);
    t.work = true;
    t.document = None;
    t.span = [0, at("## Checklist")];
    g.add(t);
    let mut sec = unit(
        "t-cd/S-checklist",
        "section",
        src,
        6,
        &text[at("## Checklist")..],
    );
    sec.span = [at("## Checklist"), text.len()];
    g.add(sec);
    for (id, line, l) in [
        ("i-aa01", 7, "- [ ] one thing to do (i-aa01)\n"),
        ("i-aa02", 8, "- [ ] another (i-aa02)\n"),
    ] {
        let mut i = unit(&format!("t-cd/{id}"), "item", src, line, l);
        i.section = Some("t-cd/S-checklist".to_owned());
        i.span = [at(l), at(l) + l.len()];
        g.add(i);
    }
    g
}

#[test]
fn an_item_inside_a_section_given_in_full_is_f_and_its_line_is_not_repeated() {
    let g = checklist();
    let p = compile(&g, "t-cd", &Options::default()).unwrap();
    // Each item line appears once: inside the section's text.
    assert_eq!(p.markdown.matches("(i-aa01)").count(), 1, "{}", p.markdown);
    assert!(p.markdown.contains("[F] t-cd/i-aa01 · item"));
    assert!(!p.markdown.contains("> t-cd/i-aa01 · item"));
    let item = p.records.iter().find(|r| r.id == "t-cd/i-aa01").unwrap();
    assert_eq!(item.tier, "full");
    assert_eq!(
        item.text.as_deref(),
        Some("- [ ] one thing to do (i-aa01)\n")
    );
    // The control: a budget that fits the ticket's record but not the
    // section leaves the section `[S]`; its items are then not covered and
    // are not marked `[F]`.
    let tight = Options {
        budget: Some(p.markdown.find("> t-cd/S-checklist").unwrap()),
        ..Options::default()
    };
    let q = compile(&g, "t-cd", &tight).unwrap();
    assert!(
        q.markdown.contains("[S] t-cd/S-checklist"),
        "{}",
        q.markdown
    );
    assert!(!q.markdown.contains("[F] t-cd/i-aa02"), "{}", q.markdown);
    // An item target is printed as its own entry; its section is not pulled in.
    let r = compile(&g, "t-cd/i-aa01", &Options::default()).unwrap();
    assert!(
        r.markdown.contains("> t-cd/i-aa01 · item"),
        "{}",
        r.markdown
    );
    assert!(!r.records.iter().any(|x| x.id == "t-cd/S-checklist"));
}

// ---- presets (section 10.4) ------------------------------------------------

fn preset(name: &str) -> Options {
    Options::of(Preset::builtin(name).unwrap())
}

#[test]
fn every_builtin_preset_exists_and_an_unknown_name_is_none() {
    for name in PRESETS {
        assert_eq!(Preset::builtin(name).unwrap().name, name);
    }
    assert!(Preset::builtin("everything").is_none());
    let o = Options::with(
        Preset::builtin("agent.min").unwrap(),
        None,
        Some(100),
        Some(2),
    );
    assert_eq!((o.budget, o.depth), (Some(400), 2));
    let o = Options::with(Preset::builtin("human").unwrap(), None, None, None);
    assert_eq!((o.budget, o.depth), (None, 3));
}

#[test]
fn agent_min_cuts_rationale_and_its_continuations_and_never_reaches_an_option() {
    let mut g = sample();
    let mut req = unit(
        "REQ-a",
        "requirement",
        "docs/records/a/10.md",
        3,
        "## REQ-a · requirement — a\nstatement: S.\nrationale: R\n  more R\nverify: V.\n\nrationale: body, kept\n",
    );
    req.title = Some("a".to_owned());
    g.units.insert("REQ-a".to_owned(), req);
    g.add(unit(
        "OPT-a",
        "option",
        "docs/records/a/10.md",
        30,
        "## OPT-a · option — o\n",
    ));
    g.edges.push(edge("REQ-a", "selected_over", "OPT-a"));
    let p = compile(&g, "REQ-a", &preset("agent.min")).unwrap();
    assert!(
        p.markdown
            .contains("statement: S.\nverify: V.\n\nrationale: body, kept\n"),
        "{}",
        p.markdown
    );
    assert!(!p.markdown.contains("more R"));
    assert!(!ids(&p).contains(&"OPT-a"));
    // agent keeps both.
    let full = compile(&g, "REQ-a", &Options::default()).unwrap();
    assert!(full.markdown.contains("rationale: R\n  more R\n"));
    assert!(ids(&full).contains(&"OPT-a"));
}

#[test]
fn human_wraps_editable_units_gives_everything_and_puts_its_own_units_first() {
    let mut g = sample();
    for u in g.units.values_mut() {
        u.editable = !u.work;
    }
    let p = compile(&g, "W-1", &preset("human")).unwrap();
    assert!(
        p.markdown
            .contains("· budget none · depth 3 · preset human (walk core) ·")
    );
    assert!(!p.markdown.contains("[S] "));
    assert!(p.records.iter().all(|r| r.tier == "full"));
    assert_eq!(
        &ids(&p)[..4],
        ["W-1", "W-1/S-intent", "W-1/S-basis", "W-1/OBL-001"]
    );
    let s = &g.units["W-1/S-intent"];
    assert!(p.markdown.contains(&format!(
        "> W-1/S-intent · section · docs/warrants/W-1.md:6 · {}\n<!-- rec W-1/S-intent docs/warrants/W-1.md:6 {} -->\n## Intent\nwhy\n<!-- /rec -->\n\n",
        short_revision(&s.revision),
        s.revision
    )));
    // The work document's own record is never wrapped.
    assert!(!p.markdown.contains("<!-- rec W-1 "));
    // agent wraps nothing.
    assert!(
        !compile(&g, "W-1", &Options::default())
            .unwrap()
            .markdown
            .contains("<!-- rec ")
    );
}

#[test]
fn review_walks_to_the_obligations_and_back_to_what_they_evaluate() {
    let mut g = sample();
    g.tracking = Some(Tracking {
        at: "2026-10-14T10:00:00Z".to_owned(),
        journal_digest: selection_digest([]),
        ..Tracking::default()
    });
    let p = compile(&g, "REQ-a", &preset("review")).unwrap();
    assert_eq!(ids(&p), ["REQ-a", "OBL-a"]);
    assert!(
        p.markdown
            .contains("· preset review (walk review) · selection ")
    );
    assert!(
        p.markdown
            .contains(" · journal sha256:4f53cda18c2b · at 2026-10-14T10:00:00Z -->\n")
    );
    // From an obligation: out:evaluates reaches what it evaluates.
    let o = compile(&g, "OBL-a", &preset("review")).unwrap();
    assert_eq!(o.records[1].via.as_ref().unwrap().step, "out:evaluates");
}

#[test]
fn resume_gives_each_item_its_state_line_and_at_most_three_next_and_refuses_a_non_work_target() {
    let mut g = sample();
    for (i, line) in [10, 11, 12, 13].iter().enumerate() {
        g.add(unit(
            &format!("t-ab/i-{i}{i}{i}"),
            "item",
            "docs/tickets/t-ab.md",
            *line,
            &format!("- [ ] more (i-{i}{i}{i})\n"),
        ));
    }
    let mut t = Tracking {
        at: "2026-10-14T10:00:00Z".to_owned(),
        journal_digest: selection_digest([]),
        ..Tracking::default()
    };
    t.state_lines.insert(
        "t-ab/i-9c01".to_owned(),
        "state claimed · holder bob · lease X".to_owned(),
    );
    t.next.insert(
        "t-ab".to_owned(),
        ["t-ab/i-000", "t-ab/i-111", "t-ab/i-222", "t-ab/i-333"]
            .map(str::to_owned)
            .to_vec(),
    );
    g.tracking = Some(t);
    let p = compile(&g, "t-ab", &preset("resume")).unwrap();
    assert!(
        !ids(&p).contains(&"t-ab/S-intent"),
        "the record and its items only"
    );
    assert!(p.markdown.contains("docs/tickets/t-ab.md:9 · sha256:"));
    assert!(
        p.markdown
            .contains("\nstate claimed · holder bob · lease X\n- [ ] do it (i-9c01)\n")
    );
    assert!(p.markdown.ends_with(
        "## Next\n- t-ab/i-000 · title of t-ab/i-000\n- t-ab/i-111 · title of t-ab/i-111\n- t-ab/i-222 · title of t-ab/i-222\n\n"
    ));
    let r = compile(&g, "REQ-a", &preset("resume")).unwrap_err();
    assert_eq!(r.rule, RULE_TARGET_UNKNOWN);
}

#[test]
fn a_profile_declares_a_preset_and_one_naming_an_unknown_step_is_an_error() {
    let profiles = |steps: &str| {
        let text = format!(
            "schema = \"oh.war/profile/v1\"\nformat = \"openwarrant/1.0\"\nname = \"runbook\"\nform = \"document\"\ncapabilities = [\"structure\", \"links\"]\n[[presets]]\nname = \"oncall\"\nsteps = [{steps}]\ndepth = 2\nbudget = \"none\"\nreads_state = true\n"
        );
        openwarrant_core::format_profile::Profiles::load([(
            "profiles/runbook.toml",
            text.as_bytes(),
        )])
    };
    let p = Preset::declared("oncall", &profiles("\"in:evaluates\""))
        .unwrap()
        .unwrap();
    assert_eq!(
        (p.walk.as_str(), p.depth, p.budget, p.reads_state),
        ("oncall", 2, None, true)
    );
    assert_eq!(p.steps, ["in:evaluates"]);
    assert!(Preset::declared("other", &profiles("\"in:evaluates\"")).is_none());
    assert!(
        Preset::declared("oncall", &profiles("\"sideways\""))
            .unwrap()
            .is_err()
    );
}
