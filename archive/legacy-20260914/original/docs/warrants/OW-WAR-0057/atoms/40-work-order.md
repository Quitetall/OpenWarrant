---
schema: oh.war/atom/v1
warrant_uuid: 01a0603f-d2c2-7ad1-a9f7-aa223a6d6559
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

## Deliverables

1. `render_html(status, canonical_json) -> String` in the compiler crate: a
   single HTML file with the §17.1 generated banner as a comment, the
   canonical JSON inlined in a `<script type="application/json">` element
   (`</` escaped so no content can close it), and an inline renderer that
   builds the page from that element. Deterministic: the bytes are a fixed
   template plus the canonical JSON.
2. Sections, in order: the provenance banner and caveats; Release with the
   requirement ladder; Objectives with exit Warrant, achievement and ladder;
   Next actionable; what blocks resolution, by requirement name; Requirements,
   unaddressed first; Warrants; Not reported here.
3. Theme-aware: light tokens on `:root`, dark under `prefers-color-scheme`.
   Wide tables scroll in their own container; the body never scrolls
   horizontally.
4. `CORPUS_STATUS.html` under `docs/warrants/generated/`, emitted on
   full-corpus `war compile` beside the JSON and Markdown, drift-checked by
   `war check --generated` under the existing `corpus-status` rule.
5. A link from the page to the JSON it was built from, and a line naming the
   Markdown twin.

## Frozen Surfaces

The JSON. This Warrant reads it and does not change one byte of what
OW-WAR-0055 emits. The `corpus-status.{drift,missing}` rule names.

## Premade Instructions

- No fetch, no CDN, no framework. If the page needs a library, the page is
  wrong.
- No division anywhere in the renderer. The projection's rule is the page's
  rule.
- The banner is the first thing rendered, above the title.

## Autonomy and Escalation

Tier T2. Escalate if the page cannot be made deterministic — a timestamp or
an unordered collection in the template is a design defect.

## Rollback

Revert. The JSON and Markdown remain; only the page goes.
