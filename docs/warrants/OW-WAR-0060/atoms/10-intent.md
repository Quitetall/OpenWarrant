---
schema: oh.war/atom/v1
warrant_uuid: 01a060de-a1c9-77f2-962a-74ca52797a34
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

Let someone who is not sitting at a clone see where the corpus stands.

## Where the viewer stood

OW-WAR-0057 delivered the viewer as one committed file,
`docs/warrants/generated/CORPUS_STATUS.html`, with the projection's JSON
inlined so it opens from disk with no server and no fetch. It is
drift-checked on every push. And it is reachable only by cloning: GitHub
renders an HTML file as source, and this repository had no Pages site. The
owner's question — "does an external viewer have the ability to see
OpenWarrant progress?" — had the answer "only with a checkout".

## What this delivers

A GitHub Pages site that IS the committed projection. One workflow, on a
push to `main` that touches the three generated files, copies
`CORPUS_STATUS.{html,json,md}` into the site with the HTML also as
`index.html`, and proves it copied them: the sha256 of every uploaded file
is compared with the sha256 of the committed one before anything is
uploaded. Nothing is rendered, generated or rewritten on the way. A Pages
build that could differ from the checked-in file would be a second,
unchecked projection, and the whole point of OW-WAR-0055 was that there is
one.

And a gate step that can refuse a workflow: every `uses:` in
`.github/workflows/` must be pinned to a full commit SHA, and the Pages
workflow must name the committed projection as its source and compare
digests. The repository always pinned by SHA by convention; a convention
nobody checks is a string, not a gate.

## What this does not do

It adds no server, no framework, no analytics and no second rendering. The
site has exactly the four files the repository has, at the same bytes.
Huly, herdr and whoever else consume `CORPUS_STATUS.json` from the
published URL; nothing here integrates with them.
