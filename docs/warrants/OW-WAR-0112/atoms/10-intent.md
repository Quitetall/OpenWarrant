---
schema: oh.war/atom/v1
warrant_uuid: 01a0ca4a-0c02-7cd3-b49d-786377a1aa06
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

Two days of running this repository's own loop, measured on 2026-09-20 to
22, found where OpenWarrant slows a developer down, and none of it was the
compiler: `war check --generated` over 107 Warrants takes 0.8 s. The time
goes to four things.

The pin model. A resolved Warrant pins every delivered file forever, and
every later edit costs a human-signed correction from every Warrant that
once delivered it: 52 resolved pins over 47 paths, 27 already corrected by
hand, four files pinned by two Warrants, and a one-word fix in `resolve.rs`
blocked behind a signature. The oldest, most-touched code — the debt — is
the most expensive to touch, and the cost only rises. A relicense of
seventeen headers produced twenty-one correction acts.

The warnings. 65 of 92 are one rule, `sas.pin-superseded`, one per Warrant
authorized before SAS 1.0.0, each wanting a signed amendment nobody will
write. A warning no one can clear is wallpaper, and the next real warning
hides in it. No diagnostic names the command that fixes it, although §76.2
has required "likely remediation" since 1.0.0.

Setup. Ten commands, two files "no tool writes", and a flag (`ssh-add -c`)
that nothing can check and everything depends on. `war` with no arguments
prints a usage error.

And the tell: the performer of the last two days of work skipped the step
that says "get a Warrant first", because it was heavy and the work was
moving. Thirteen commits landed on a branch with no Warrant, in the
repository whose thesis is that work without one is unaccounted for.

## Desired Outcome

A developer installs `war`, types `war`, answers three questions, signs
twice, and is working. Old code is touched by declaring it in the Warrant
that does the work. Every red line in `war check` says what to run, and the
app runs it on a keypress when no signature is involved. The 65 warnings
become one act. The dashboard OW-WAR-0073 authorized is built, and it is
what `war` opens.

Concretely, five things, in dependency order:

1. **Pins mean ownership.** The most recently authorized Warrant that
   declares a path governs it; earlier pins are historical, still recorded,
   still verifiable; drift is an error only for a change no authorized
   Warrant declares. OW-ADR-0021; RQ-036 retitled and RQ-037 added in SAS
   1.1.0.
2. **Every diagnostic carries its remedy** — the exact argv and whether a
   human must run it — in the envelope, in the terminal, and in the app.
3. **`war init` is a conversation** that reaches a signed SAS and a signed
   first Warrant, and writes the authority files only from answers typed at
   a terminal, once.
4. **`war` is the app.** OW-WAR-0073's dashboard, executed as authorized,
   plus a Setup tab, a Help tab (what to do next, and the repository's own
   documents rendered), a remedies pane, and no-args as the way in.
5. **`war sas repin`** writes the re-pin amendments for every open Warrant
   in one act, so the next signature is the human's and the warnings clear.

## Scope

The five above, their plants on scratch programs, the SAS revision that
carries the first, the ADR, the four documents whose rule the third one
changes, and the supersession of OW-WAR-0073 with its deliverables adopted.

## Non-goals

- Retroactive ownership. Authorizations recorded before OW-ADR-0021 own
  nothing; the twenty relicense corrections stand and are signed as
  corrections.
- Retiring `war console`. It stays as the line-based fallback over ssh and
  inside the battery, 0073's second escalation kept at its draft answer.
- `war sign --batch`. OW-WAR-0072 owns it; until it lands each re-pin is one
  dialog, and this Warrant says so rather than building a second batch act.
- A markdown dependency. The Help tab renders with an in-house converter of
  about a hundred lines; `deny.toml` treats a new licence family as a
  refusal.
- Auto-compiling inside `war check`. A drift check that writes in order to
  compare is a mutating gate (§44.8, `compile.rs`); "forgot to recompile" is
  a remedy the app applies, never a check that repairs.

## SAS and Roadmap Traceability

`sas://WAR-SAS-RQ-036` (retitled) and `sas://WAR-SAS-RQ-037` (new), complete;
`sas://WAR-SAS-RQ-033`, `sas://WAR-SAS-RQ-070`, `sas://WAR-SAS-RQ-074`,
`sas://WAR-SAS-RQ-075`, partial. `roadmap://OW-PHASE-1/console`: Phase 1's
exit is "OpenWarrant development uses WARs", and `PRODUCTION_ROADMAP.md`
records it as resolved but not whole; the last two days are why.
