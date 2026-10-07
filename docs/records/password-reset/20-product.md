---
schema: oh.war/records/v1
profile: prd
---
# Password reset: scope

What the reset flow deliberately leaves out, and what is still open. The
outcome, the requirement and the constraint are in 10-records.md; the PRD
(generated/prd.md) shows them together.

## NG-pr1 · non_goal

Recovering an account without access to its registered email address.
Support-assisted recovery and security questions are separate work.

## NG-pr2 · non_goal

Changing a password while signed in. That flow exists and does not change.

## Q-pr1 · question

Should a completed reset sign the account out of its other sessions?
