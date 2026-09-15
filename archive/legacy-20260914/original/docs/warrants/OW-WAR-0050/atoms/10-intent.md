---
schema: oh.war/atom/v1
warrant_uuid: 01a0399d-05b9-7ad0-b8dc-bf1a226fa641
role: intent
jurisdiction: authored
order: 10
classification: internal
---

# Intent

## Problem

OpenWarrant records work and evidence, while Bonsai checks repository structure.
Neither currently binds one pull-request diff to the other. A passing Bonsai
run can therefore be real but unrelated to a Warrant, and a Warrant can name
scope only in prose.

## Desired Outcome

A draft Warrant can bind a repository, policy digest, path scope, and
obligations in `scope.toml`. `war bonsai check` then emits evidence tying that
scope and contract digest to an exact base, head, tree, policy, Bonsai output,
and verdict.

## Scope

OpenWarrant's compiler and CLI seam, the pilot Bonsai policy, pull-request
automation, draft integration ADRs, and qualification plants.

## Non-goals

- This Warrant does not authorize work, verify a result, or resolve any
  obligation. Those remain external human actions.
- Advisory Bonsai signals and leanness are not blocking policy in this pilot.
- No hosted service or change to Bonsai's generic public interface.
