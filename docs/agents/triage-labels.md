# Triage labels

Use these five default mappings when a Matt Pocock skill names a triage role.

| Role | GitHub label | Meaning |
| --- | --- | --- |
| `needs-triage` | `needs-triage` | Maintainer needs to evaluate the issue. |
| `needs-info` | `needs-info` | Waiting for more information. |
| `ready-for-agent` | `ready-for-agent` | Specified for agent work, subject to Warrant authorization and the frontier. |
| `ready-for-human` | `ready-for-human` | Requires human action. |
| `wontfix` | `wontfix` | Request will not be actioned. |

Labels describe intake state. They do not authorize work, establish obligations,
or resolve Warrants; follow `AGENTS.md` for those acts.

These mappings configure the skills locally. Inspect existing tracker labels
before an authorized triage operation and create missing labels within its scope.
Other labels, such as `bug` and `documentation`, remain independent of these roles.
