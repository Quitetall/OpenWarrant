---
schema: oh.war/atom/v1
warrant_uuid: 01a03fb3-3dba-7205-8fdf-bbd7354e0aa3
role: basis
jurisdiction: authored
order: 20
classification: internal
---

# Basis

Measured in `/mnt/4tb/openhuman-knowledge-fabric` and against the `kf-dogfood`
host on 2026-08-26. Re-measure before acting: several of these are host state and
will change the moment commissioning proceeds.

## What exists and makes the invariant checkable

The claim in `10-intent.md` is only worth making because KF already knows what a
person can see. Four primitives carry it, and all four were read from the tree
rather than remembered:

    core.set_access_context(organization, max_classification)
                                            permission(O,C) is ENUMERABLE
    org.person.id references core.object    a person is a NODE, so relevance
                                            is a typed traversal
    secure_object.erasure_tombstone         signed withdrawal primitive
    core.outbox                             transactional delivery socket
    quality.federated_source                check (writable = false)

`core.set_access_context` is the load-bearing one, and its exact signature is
`(p_organization uuid, p_max_classification text)`. It sets `kf.organization` and
`kf.max_classification`; row-level security on 76 tables is enforced against
those. **It takes no person.** `kf.actor` and `kf.acting_role` exist as session
variables but no policy in `row_security.sql` reads either — they carry
authorship, not visibility.

So what is enumerable is `permission(O, C)`, a tenant-and-ceiling set. Two people
in one organization at one ceiling see exactly the same rows. An earlier draft of
this Warrant described this function as person-scoped and built the whole
checkability claim on it; that was wrong, and the correction is why membership is
permission and relevance only sections it.

`core.outbox` is `(id, action_id, topic, payload, created_at, delivered_at)` with
a partial index on undelivered rows. It is written inside the action transaction
and **nothing drains it**. The delivery socket was cut and never plugged, so M5
is an adapter rather than new architecture.

`quality.federated_source` carries `constraint federated_source_read_only check
(writable = false)`. Inbound federation is structurally read-only, which is
correct for gathering. Note carefully for the deferred Drive work: writing OUT is
not federation, so that CHECK does not forbid it — but it is the boundary the
constraint was drawn beside, and crossing it is a decision rather than an
oversight.

`@kf/export` already emits `.html`, `.md` and `.json` in sections
(`business`, `core`, `documents`, `engineering-quality`, `ml`, `secure-runtime`),
so the compiler has a starting point rather than a blank page.

## What does not exist

**No outbound transport of any kind.** Zero matches for nodemailer, SMTP,
SendGrid, Postmark or any mailer across all TypeScript in `packages/` and
`apps/`. This is not a gap in configuration; there is no code.

**No running instance.** On `kf-dogfood`, 2026-08-26:

    kf-* systemd units installed     zero
    API on :4000                     000, connection refused
    nginx on :443                    502, nothing behind it
    databases on :5432               keycloak, postgres, template0/1
    production `kf` database         DOES NOT EXIST
    /etc/kf/migrator/database-url    0 bytes

The release `3054582c84a1` is unpacked at `/opt/kf` and verifies, but no unit was
ever installed and the schema has never been applied to a production database.
The apply step has never run — correctly, because it requires a passing rollback
rehearsal receipt and until 2026-08-26 there had never been one.

## What this changes about sequencing

Everything in STAGE-001 through STAGE-003 needs only the database test harness
(`startHarness()` in `tests/database/`), so the invariant, the closure and the
compiler can be built and proven with no host at all. STAGE-004 onward needs
commissioning.

This matters because ADR 0009 (Google Drive ingestion) was deferred on exactly
the objection that it was a feature on a system nobody runs. That objection does
not reach STAGE-001 through STAGE-003 and does reach STAGE-004 onward. Sequence
accordingly rather than treating the whole Warrant as blocked.

## Adjacent facts that bound the work

**`war kf` has never been called.** The adapter exists from OW-WAR-0044, sends
`x-kf-actor`, `x-kf-acting-role` and `x-kf-organization`, and guards writes with
`--confirm-write`. It carries **no classification header**. For a disclosure act
that is not cosmetic: the wire should carry the ceiling the disclosure is bounded
by. KF decision ADR 0008 is open on what classification means as a granted
privilege versus a self-limit. It is now on the critical path rather than beside
it — see OBL-010 and M2E — because membership is permission, permission is
determined solely by ceiling, and deriving a person's ceiling is exactly what
that decision leaves open.

**pandoc is qualified on the host.** Version 3.1.11.1, and all ten drift-prone
constructs agree with CI's 3.1.3 and the workstation's 3.10.2. So PDF and DOCX
rendering rests on three agreeing versions rather than an assumption — but the
version question is deliberately open, not settled, and a rendering pipeline is a
new reason to care about it.

**`core.retention_hold`** exists and is the mechanism by which something may be
legitimately withheld from a person who could otherwise see it. Any withholding
must appear in the document as withheld, per OBL-003; a hold is a reason, not a
licence to omit silently.
