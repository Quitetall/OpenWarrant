# Standalone board offline observation, 2026-10-09

The actual `war board --html` output opened by file URL in a fresh owned browser
with network emulation offline and HTTP/WebSocket requests blocked. All 145
Warrant aliases from the exact `war board --json` payload occur in the rendered
page. Objectives, stages, questions and exact approval commands remain readable.
The page is static: no script or image node, signing control or network action.
An external fetch control fails. No signature or repository mutation occurs.

The first screenshot exposed horizontal overflow from unwrapped objective and
command lines. The source serializer now supplies fixed inline CSS that wraps
preformatted text. CSP allows that fixed style only; scripts, network loads, forms
and base URLs remain denied. The public board CLI test still proves corpus,
frontier/question/approval parity, no writes and refusal of corrupt records.

At a 375-pixel viewport the final page reports 360-pixel document width, with no
horizontal overflow. Desktop and narrow screenshots were visually inspected.
Baseline HTML, JSON, screenshot and observation remain alongside the new capture;
no historical observation was rewritten. The final browser observation includes
exact source and HTML hashes and explicit offline state.

The shared reproduction script, complete web-suite output and limitations are at
`../../OW-WAR-0102/implementation/offline-20261009/`. The script launches only a
fresh temporary headless profile. Agent Workspace MCP tools were unavailable;
no user browser/profile was attached. Browser offline controls are not operating
system network isolation. The browser exited and its temporary profile was removed.

This completes the remaining implementation observation for the legacy standalone
board. Existing roadmap/live-browser and owner offline-roadmap observations cover
the separate configurable tree, filters, details, percentages and refresh controls.
Do not substitute this static-board evidence for those interactive observations.
Implementation completion remains unverified: no independent disposition, human
signature or assurance mark is claimed. Legacy OW69 question reconciliation keeps
its actual historical state.
