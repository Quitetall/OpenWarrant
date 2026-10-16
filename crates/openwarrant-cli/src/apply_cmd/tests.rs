// SPDX-License-Identifier: Apache-2.0
//! Reading a package's marked blocks (section 10.5); `war apply` itself is
//! exercised through the binary in tests/context_apply_cli.rs.

use super::*;

const REV: &str = "sha256:59a4183ac803625dd391e5deac0dd1bb39559b9125189a4c3d682d3a347d7173";

#[test]
fn a_marked_block_is_read_exactly_and_text_outside_markers_is_not() {
    let pkg = format!(
        "# Context: t\n[F] t-1/S-intent · section · - · Intent · x.md:3\n\n> t-1/S-intent · section · x.md:3 · sha256:59a4\n<!-- rec t-1/S-intent docs/tickets/t-1.md:10 {REV} -->\n## Intent\nwhy\n\n<!-- /rec -->\n\nafter\n"
    );
    let b = blocks(&pkg);
    assert_eq!(b.len(), 1);
    assert_eq!(b[0].id, "t-1/S-intent");
    assert_eq!(b[0].source, "docs/tickets/t-1.md");
    assert_eq!(b[0].line, 10);
    assert_eq!(b[0].revision, REV);
    assert_eq!(b[0].text, "## Intent\nwhy\n\n");
}

#[test]
fn a_malformed_or_unclosed_marker_is_no_block() {
    for bad in [
        "<!-- rec t-1/S-intent docs/t.md sha256:00 -->\nx\n<!-- /rec -->\n",
        &format!("<!-- rec t-1/S-intent docs/t.md:x {REV} -->\nx\n<!-- /rec -->\n"),
        &format!(
            "<!-- rec t-1/S-intent docs/t.md:3 {} -->\nx\n<!-- /rec -->\n",
            REV.to_uppercase()
        ),
        &format!("<!-- rec t-1/S-intent docs/t.md:3 {REV} -->\nnever closed\n"),
    ] {
        assert!(blocks(bad).is_empty(), "{bad}");
    }
    assert!(blocks("").is_empty());
}

#[test]
fn evidence_directory_is_beside_a_one_file_document_and_the_directory_of_another() {
    assert_eq!(
        evidence_dir(Utf8Path::new("/r/docs/tickets/t-1.md"), "t-1"),
        "/r/docs/tickets/t-1"
    );
    assert_eq!(
        evidence_dir(Utf8Path::new("/r/docs/warrants/W-1/warrant.md"), "W-1"),
        "/r/docs/warrants/W-1"
    );
}
