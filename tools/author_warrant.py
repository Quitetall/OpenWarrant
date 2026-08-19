#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Author a Warrant from a specification dict.

This is scaffolding for the ROADMAP Warrants, not a replacement for thinking:
every field below is supplied per Warrant. It exists because writing 35 manifests
by hand invites copy-paste drift in the parts that must be identical (schema
strings, atom ordinals, frontmatter shape) while the parts that must differ
(intent, basis, obligations) are supplied as real text.

Once `war new` grows profile templates rich enough to carry this, delete it.
"""

from __future__ import annotations

import os
import pathlib
import sys
import time


def uuid7() -> str:
    """UUIDv7 per SAS §12.2."""
    ts = int(time.time() * 1000) & ((1 << 48) - 1)
    rand = os.urandom(10)
    rand_a = ((rand[0] << 8) | rand[1]) & 0x0FFF
    b = bytearray(16)
    b[0:6] = ts.to_bytes(6, "big")
    b[6] = 0x70 | (rand_a >> 8)
    b[7] = rand_a & 0xFF
    b[8] = 0x80 | (rand[2] & 0x3F)
    b[9:16] = rand[3:10]
    h = b.hex()
    return f"{h[0:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:32]}"


ROOT = pathlib.Path(__file__).resolve().parent.parent


def frontmatter(uuid: str, role: str, order: int) -> str:
    return (
        "---\n"
        "schema: oh.war/atom/v1\n"
        f"warrant_uuid: {uuid}\n"
        f"role: {role}\n"
        "jurisdiction: authored\n"
        f"order: {order}\n"
        "classification: internal\n"
        "---\n\n"
    )


def author(spec: dict) -> pathlib.Path:
    alias = spec["alias"]
    uuid = spec.get("uuid") or uuid7()
    directory = ROOT / "docs" / "warrants" / alias
    atoms = directory / "atoms"
    atoms.mkdir(parents=True, exist_ok=True)

    implements = "\n".join(
        f'\n[[implements]]\nref = "sas://{r}"\ncontribution = "complete"'
        for r in spec.get("implements", [])
    )
    manifest = f"""schema = "oh.war/manifest/v1"
uuid = "{uuid}"
local_alias = "{alias}"
enterprise_id = ""
title = "{spec['title']}"
profile = "delivery"
assurance_level = "{spec.get('assurance', 'basic')}"
{implements}

[[roadmap]]
ref = "{spec['roadmap']}"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"
required = true

[[atoms]]
ordinal = 20
role = "basis"
path = "atoms/20-basis.md"
required = true

[[atoms]]
ordinal = 40
role = "work_order"
path = "atoms/40-work-order.md"
required = true

[[atoms]]
ordinal = 45
role = "milestones"
path = "atoms/45-milestones.yaml"
required = true

[[atoms]]
ordinal = 60
role = "assurance"
path = "atoms/60-assurance.md"
required = true
"""
    (directory / "manifest.toml").write_text(manifest)

    (atoms / "10-intent.md").write_text(
        frontmatter(uuid, "intent", 10)
        + f"# Intent\n\n## Problem\n\n{spec['problem']}\n\n"
        f"## Desired Outcome\n\n{spec['outcome']}\n\n"
        f"## Scope\n\n{spec['scope']}\n\n"
        f"## Non-goals\n\n{spec['non_goals']}\n\n"
        f"## SAS and Roadmap Traceability\n\n{spec['traceability']}\n"
    )

    (atoms / "20-basis.md").write_text(
        frontmatter(uuid, "basis", 20)
        + f"# Basis\n\n## Governing Sources\n\n{spec['sources']}\n\n"
        f"## Prerequisites\n\n{spec['prerequisites']}\n\n"
        f"## Assumptions and Unknowns\n\n{spec['unknowns']}\n\n"
        f"## Constraints and Invariants\n\n{spec['constraints']}\n"
    )

    (atoms / "40-work-order.md").write_text(
        frontmatter(uuid, "work_order", 40)
        + f"# Work Order\n\n## Deliverables\n\n{spec['deliverables']}\n\n"
        f"## Frozen Surfaces\n\n{spec['frozen']}\n\n"
        f"## Premade Instructions\n\n{spec['instructions']}\n\n"
        f"## Autonomy and Escalation\n\n{spec['autonomy']}\n\n"
        f"## Rollback\n\n{spec['rollback']}\n"
    )

    (atoms / "45-milestones.yaml").write_text(spec["milestones"])

    (atoms / "60-assurance.md").write_text(
        frontmatter(uuid, "assurance", 60)
        + f"# Assurance\n\n## Acceptance Obligations\n\n{spec['obligations']}\n\n"
        f"## Gate Adequacy\n\n{spec['adequacy']}\n\n"
        f"## Residual Risk\n\n{spec['residual']}\n"
    )
    return directory


if __name__ == "__main__":
    print("import this module and call author(spec)", file=sys.stderr)
    sys.exit(1)
