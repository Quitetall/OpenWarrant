# `war inbox`

List Warrants whose next required act needs a human. This is a read-only,
pull-only view of legacy records. It does not approve, verify, resolve, schedule
or execute work. Prompt-only completion and the assurance mark remain separate.

```sh
war inbox
war inbox --json
```

Human output contains one tab-separated row per Warrant: alias, title, current
phase, awaited act and age in seconds. Control characters in titles are escaped.
An empty inbox prints `Nothing waiting on a human.` and exits zero. Load failures
exit nonzero with diagnostics; they do not become an empty successful result.

## Classification

Human requests take precedence over agent or gate work in every phase. An open
**blocking** hotline question takes precedence over signing because its answer
can change the contract to be signed. Nonblocking questions are not required
acts. Signing readiness comes from the existing `war sign --list` request logic:
`authorize`, `correct`, or `resolve`. Multiple corrections produce one `correct`
row; use `war sign --list` for the individual targets. A missing eligible signer
does not erase a required human act. SAS acceptance is outside this inbox.

When neither a blocking question nor a signing request exists:

| Phase | Next act | Included? |
| --- | --- | --- |
| draft | Agent prepares work or a valid request | No |
| proposed | Agent completes request prerequisites | No |
| authorized | Agent work | No |
| ready | Agent work | No |
| executing | Agent work | No |
| verifying | Gate / independent verification | No |
| resolved | None | No |

Phase alone never establishes permission or readiness to sign. The total
classifier covers all seven core phases. The current legacy loader records only
`draft`, `authorized` and `resolved`; other phases are covered at the pure
classifier seam, not invented as persisted state transitions. This preserves
SAS §24.6–24.7 state semantics and §27.2 human acts. Request readiness remains
with the existing §28 and §56 implementations; the hotline owns question records.

Age means time since the last recorded lifecycle transition in journal order:
draft creation, authorization, verification, resolution, correction, question
asked or answered. It is **not** elapsed time since a human first became eligible
to act. Receipt attachments and dispatch context generation do not reset it.
Journal times are local observations, including explicitly backfilled history;
they are not new authoritative timestamps.

Absent, malformed or future transition times produce `unknown` / JSON `null`.
There is no fallback to file modification time or Git commit time. Known times
sort oldest first by instant, then alias; unknown times sort last, then alias.
An unreadable journal fails the command instead of erasing history.

Example from the synthetic conformance repository (age depends on current time):

```text
WARRANT       TITLE                       STATE       AWAITED ACT  AGE
IX-WAR-0001   Adopt OpenWarrant in Inbox   draft       authorize    212544000s
IX-WAR-0002   Adopt OpenWarrant in Inbox   authorized  answer       212544000s
```

## JSON contract: proposed, not adopted

OW-ADR-0018 remains proposed. Its candidate schema is checked in at
`conformance/fixtures/inbox/inbox.schema.json`; it is intentionally **not** in
`war schemas`, and does not change the existing schema-pack digest. Do not treat
this candidate as an adopted stable contract.

Like other commands, JSON is inside the `result` field of the existing
`oh.war/report/v1` envelope. Example result:

```json
{
  "api_version": "oh.war/inbox/v1",
  "namespace": "IX",
  "generated_at": "2026-09-16T00:00:00Z",
  "items": [
    {
      "alias": "IX-WAR-0001",
      "title": "Adopt OpenWarrant in Inbox",
      "state": "draft",
      "awaited_act": "authorize",
      "waiting_since": "2020-01-01T01:00:00+01:00"
    }
  ]
}
```

No notification transport, identity filtering, command-specific sort/filter
options, workflow buttons or agent execution are added by this command.
