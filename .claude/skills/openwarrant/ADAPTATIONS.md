# Skill adaptation provenance

Local adaptation revision: routing-cues-20261024 (2026-10-24).
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

OW85 update: RC.3 authoring now points to shipped `war admin sdk` transport. Fixed
artifact/host-entry proposals are tested separately from model invocation quality.
Upstream revision and attribution remain unchanged.

M9 update (2026-10-07): every skill leads with "ordinary coding needs no
Warrant and no ticket", the ticket loop has its own reference, and rules that
belong to a Warrant with a sign-off step live in references titled for it.
Upstream revision and attribution remain unchanged.

Routing update: simple `war tickets` and `war migrate` chat cues remain distinct
from CLI command groups. A selected workflow does not reload the router. Verified
claims require independent evidence and secure human acceptance of the exact
result under the applicable baseline. These changes have source/validator checks;
model-routing quality and the broader OW68 eval scope are not claimed complete.
