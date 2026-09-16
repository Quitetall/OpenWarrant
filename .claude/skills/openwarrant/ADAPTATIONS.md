# Skill adaptation provenance

Local adaptation revision: sdk-cli-rc3-3 (2026-09-16).
Upstream: mattpocock/skills, MIT; full immutable revision
`3cca18b368ae95cdbdebbff572ccafa662551015`.
[License and copyright notice](LICENSE.mattpocock) travels with this suite.
OpenWarrant adaptations remain attributable to this repository; no upstream
endorsement or transfer of authorship is claimed.

| Local skill | Upstream method | Changed output contract |
| --- | --- | --- |
| war-grill | [skills/productivity/grilling/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/productivity/grilling/SKILL.md) | Warrant/ADR/question/stage/evidence records; prompt-only completion separate from assurance |
| war-spec | [skills/engineering/to-spec/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/engineering/to-spec/SKILL.md) | Warrant/ADR/question/stage/evidence records; prompt-only completion separate from assurance |
| war-tickets | [skills/engineering/to-tickets/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/engineering/to-tickets/SKILL.md) | Warrant/ADR/question/stage/evidence records; prompt-only completion separate from assurance |
| war-map | [skills/engineering/wayfinder/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/engineering/wayfinder/SKILL.md) | Warrant/ADR/question/stage/evidence records; prompt-only completion separate from assurance |
| war-review | [skills/engineering/code-review/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/engineering/code-review/SKILL.md) | Warrant/ADR/question/stage/evidence records; prompt-only completion separate from assurance |
| openwarrant | [skills/productivity/writing-for-agents/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/productivity/writing-for-agents/SKILL.md) | Original OpenWarrant routing/shared workflow using the writing method |
| war | [skills/productivity/writing-for-agents/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/productivity/writing-for-agents/SKILL.md) | Original OpenWarrant routing/shared workflow using the writing method |
| war-migrate | [skills/productivity/writing-for-agents/SKILL.md](https://github.com/mattpocock/skills/blob/3cca18b368ae95cdbdebbff572ccafa662551015/skills/productivity/writing-for-agents/SKILL.md) | Original OpenWarrant migration workflow using the writing method; source preservation, version selection, document mappings and validation report; no upstream migration skill claimed |

These are method adaptations, not an imported copy of the whole upstream suite.
`war` routing is local. Keep this revision fixed until a reviewed upstream update.
Audit evidence: [skill audit](../../../docs/design/war-skills-audit.md).

OW85 update: RC.3 authoring now points to shipped `war sdk` transport. Fixed
artifact/host-entry proposals are tested separately from model invocation quality.
Upstream revision and attribution remain unchanged.
