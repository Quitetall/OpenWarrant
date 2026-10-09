# Offline report observation, 2026-10-09

Source base: 9791ea9ed132c29757e455366826ef913ebfa8e6. Chrome for Testing
153.0.8010.12, Node 24.21.0, Rust 1.97.1. This is performer evidence, not an
independent verifier verdict, human acceptance or real-user study.

The current renderer replays the exact retained actual HTTP/process/Git attempt
from `../browser-report.html`. Its single configured policy reconstructs the
original one-item inventory; the snapshot identity remains
0b3072e367634b4430af428243572f598a1d6629248d51e8dea869ef29355dea. No new execution
or completion act was created. The historical observation and old downloaded
HTML remain unchanged. `current-render.html` observes the current readable layout.

A separate explicitly synthetic presentation control changes notes to hostile
img/script text and adds one pending inventory item. It displays 1/2 complete,
the configured synthetic word, pending work and literal hostile text. No image
or script node or dialog appears. It is not a claimed implementation result.

## Actual browser checks

`browser-check.mjs.txt` launches the installed headless shell in a fresh temporary
profile; it attaches only to that process. The Agent Workspace MCP was not
available. CDP network emulation is offline, HTTP/WebSocket requests are blocked,
and `navigator.onLine` is false. The files are opened by file URL. External fetch
controls fail. This proves operation under browser offline controls, not operating
system network isolation. The browser and temporary profile were removed after
the run; no user browser/profile was attached.

The report displays the configured word, unverified qualification, scoped progress,
implementation notes, next steps, pending work and an expandable exact-evidence
section. The section starts closed and opens on click. Screenshots were read
visually. Raw DOM observations, exact artifact and source-file digests, requests,
browser version and narrow board checks are in `browser-observation.json`.

## Current package checks

The first full web suite attempt reported 124 setup failures because a fresh
checkout had no default `target/debug/war`. Retain that raw failed run in
`initial-web-suite-missing-binary.log.gz`; it does not establish product behavior.
Using the documented `OW_TEST_WAR` variable with an immutable copy of the tested
main binary reran the complete suite: **212 tests passed in 108.557 seconds**.
This includes real HTTP/process/Git, persisted reports across restart, failed and
incomplete checks, changed policy/source, access refusal and separate legacy state.

Command:
`OW_TEST_WAR=/mnt/2tb/ow-main-9791e-war python3 -m unittest discover -s apps/openwarrant-web -v`

Binary SHA256: a965ab5372f2313ca4c99dcaaf4e115f158c29c941bf51a399bb24b5898119f0.
The main tree equals the previously tested 545df0de tree. No web/report code changed
for this observation. The current board wrapping change has its own CLI and browser
checks, with exact candidate hashes. Required full local and hosted gates for the
final batch remain separate from these focused observations.

Implementation scope is complete, unverified. Formal assurance and secure human
acceptance remain separate acts. Hotline/fencing, provider integration and the
three-developer study are outside this Warrant's bounded report scope.
