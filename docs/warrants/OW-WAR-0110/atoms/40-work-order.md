---
schema: oh.war/atom/v1
warrant_uuid: 01a0b4a5-58bf-7aa2-b621-7e3254ec72dc
role: work_order
jurisdiction: authored
order: 40
classification: internal
---

# Work Order

1. Add deterministic TypeScript generation from actual schema outputs through the
   existing schema tool. Support used constructs explicitly; refuse unsupported
   constructs and unresolved references. Do not silently emit any for known types.
2. Produce declarations plus a separately identified source/artifact manifest.
   Keep existing JSON pack bytes, identity and digest algorithm unchanged. Check
   generated declaration drift through the normal generator check path.
3. Cover local references, nullable fields, enums, discriminated unions, arrays,
   tuples, map values and required/optional properties. Document constraints that
   TypeScript cannot enforce; never advertise runtime validation or authority.
4. KF consumes pinned generated declarations in packages/warrants through its
   normal build. Keep runtime input validation and typed action enforcement.
   Provider changes reference this shared contract, including exact source identity.
5. Test positive and negative TypeScript consumer fixtures, unsupported schema
   refusal, deterministic rerender and hand-edited declaration drift. Run actual
   provider type/build tests and OpenWarrant gates; retain revisions and toolchains.
6. Record preserved versus still-open OW32 obligations. This scope does not claim
   KF server allocation/concurrency/Source Holder qualification or human assurance.

Stages may prepare independently. Final integration requires real successful
producer and consumer observations for matching artifacts. Prompt-authorized
unverified work; no signing, paid calls or mandatory model reviews. Stop affected
work if a required wire meaning or pinned historical source must change.
