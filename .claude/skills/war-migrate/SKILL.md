---
name: war-migrate
description: "War migrate: use when asked to migrate a system's existing documentation into OpenWarrant or upgrade older OpenWarrant documents to the latest version."
---

# war-migrate

Preserve the original, nonconforming documents unchanged in an archive, then
author new OpenWarrant documents from their content. The new documents form the
working set; each links to its exact archived sources. Read the
target repository's AGENTS.md and [shared workflow](../openwarrant/SKILL.md) once.
If the request is an explanation or assessment, keep it read-only. A migration
request permits reversible conversion work within the requested system scope.

## 1. Select the target

Inspect repository configuration, governing documents and available SDK/CLI
versions. Reuse an explicit project migration target; otherwise resolve the latest
supported stable edition against official release metadata. Record exact standard,
schema and tool versions plus how the target was selected. Previously requested
candidate migration is sufficient direction; label that target as a candidate.
If the latest version cannot be established, report that uncertainty rather than
calling a local draft stable. Read only that edition's applicable format and
compatibility contract. Completion: exact target and supported operations known.

## 2. Capture the source set

Inventory documents selected for migration and their relationships. Follow other
branches, worktrees or evidence only where needed to preserve those sources.
A whole-repository backup is optional preservation work, not the migration result.
Retain original bytes in durable storage and record repository/path, schema if
known, commit or captured-content digest, and availability. A hash list alone is
not a backup. Keep each original relative path under an identified archive root;
use separate roots for conflicting revisions instead of flattening or overwriting.
Sequester archive content from default context; retrieve required sources explicitly.
Preserve signatures, timestamps, authorship, status and uncertainty
as historical facts. Completion: source coverage explicit, retained bytes checked,
missing sources and conflicting versions listed.

## 3. Map meaning before conversion

Map source content to new supported documents, recording retained references,
duplicates and unresolved material separately. Several sources may feed one new
document; one source may supply several documents. A specification may become a SAS; a decision remains an ADR;
bounded work may become a Warrant. Keep AGENTS.md, CLAUDE.md and CONTEXT.md in their
native roles, with scoped references. Preserve unsupported fields in linked
originals. Ask only when conflicting meanings require a decision; continue
independent mappings. Completion: every inventoried source accounted for, with
exact provenance and gaps; no invented requirements or authority.

## 4. Convert through supported seams

Use SDK/CLI authoring and compatibility operations that exist in the inspected
version. Check command help: the current shell `war migrate` imports legacy ADRs
and does not implement this whole workflow. Author new conforming documents in the
working document tree, separately from archived originals. Preserve meaning and
link source sections to their new locations. Archiving, wrapping old bytes or
exposing an old record through an adapter alone does not complete conversion.
Reuse identical source/target mappings
on repeat runs. Leave signed subjects and digest domains intact. New documents
retain their real draft/unverified status; old signatures cover only old subjects.
If tools cannot validate the selected target, produce labeled drafts and a precise
unsupported-operation report. Completion: candidate outputs and mapping manifest
exist, or the unsupported slice is explicit without a false migration claim.

## 5. Validate and report

Validate target documents, links and source coverage. Check preserved bytes and
historical signatures against their original subjects using supported tools;
report unavailable verification as UNKNOWN. Check repeat-run behavior, dropped
fields and relationships, and interrupted-write recovery for the used converter.
An independent reviewer owns any required independent verdict. Human qualification
remains separate from ordinary unverified migration completion.

Return links to the migration report, source manifest, converted documents and
available generated progress, with next steps and actual validation results.
Distinguish new validated documents, drafts, opaque preservation and unresolved
items. Switch the current document entry point only within the requested scope
and applicable action gates, retaining original history. Broad migration does not
authorize source deletion. Completion: all scoped sources accounted for and all
claimed conversions produce validated new OpenWarrant documents; unresolved work
remains visible. Preservation-only items do not count as converted documents.
