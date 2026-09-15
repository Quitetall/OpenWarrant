---
name: war-review
description: Two-axis review of a Warrant's delivered work since a fixed point, Standards and Obligations, as two subagents whose findings are never merged; the Obligations axis is what a blind verifier would say. Use when the user asks to review a branch, a PR, a Warrant's delivery, or says review since X.
---

# war-review

After mattpocock/skills `engineering/code-review` (3cca18b, MIT), with the
Spec axis replaced by the Warrant's obligations and the verifier kept blind.

## 1. Pin the fixed point

`git diff <fixed-point>...HEAD` and `git log <fixed-point>..HEAD --oneline`.
Confirm the ref resolves and the diff is non-empty before spawning anything.

## 2. The two axes

**Standards** subagent: this repository's documented rules
(`CONTRIBUTING.md`, `AGENTS.md`, `docs/PROJECTION_CONTRACT.md`, the
programming standard) plus the Fowler smell baseline his skill carries
(Mysterious Name, Duplicated Code, Feature Envy, Data Clumps, Primitive
Obsession, Repeated Switches, Shotgun Surgery, Divergent Change, Speculative
Generality, Message Chains, Middle Man, Refused Bequest). Documented rules are
hard; smells are judgement calls; skip what tooling enforces. Under 400 words.

**Obligations** subagent: the bundle only.

```bash
war verify <alias> --performer <performer> --bundle
```

Hand the subagent the bundle path and nothing else: no transcript, no
rationale, no workspace. It reports per obligation: established, not
established, or refuted, quoting the obligation and the evidence. It also
reports scope creep (delivered, not asked) and what looks implemented but
wrong. Under 400 words. If the user wants it recorded, the subagent's answer
becomes a verification response with the nine independence fields set to
what was true, ingested by `war verify --response`; the performer never
writes that file.

Both run in parallel; neither sees the other.

## 3. Aggregate

Present `## Standards` and `## Obligations` verbatim. Do not merge or
rerank across axes: code that meets every standard and misses an obligation,
and code that establishes every obligation and breaks a convention, are
different failures. End with one line per axis: count and worst.

## Never

A skill never signs. A review that ends in `war resolve` is not a review.
