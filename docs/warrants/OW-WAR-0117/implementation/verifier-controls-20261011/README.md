# OW117 current verifier controls

The selected `62-verifier.sh` run passed 12 named fake-backend controls and the
positive corpus check (13 passes, zero failures). The exact source and log hashes,
command, runtime identity and bounds are in [observation.json](observation.json).
Raw observations are retained in [selected-plants.log.gz](selected-plants.log.gz).

The observations cover OW117 OBL-001’s verdict/identity boundary; OBL-002’s
unclear-answer and failed-run refusals; OBL-003’s argv and empty working-directory
checks; and OBL-004’s model-comparison and declared/undeclared-independence cases.
These are performer observations with a fake backend. They do not establish a
real model’s judgment, enforce a real harness’s confinement, or clear an independent
gate. Existing signed records and verification dispositions remain unchanged.

A separate current OW65 packet observation shows 6,559 gate-input entries in its
request. The packet exceeds its 48,000-token estimate budget before enough task
text can be carried. The whole-corpus document gate has no restricted input list;
`verify::subject` therefore binds the source inventory. Removing those digests
would change the reviewed subject. The wrapper currently receives the whole JSON.
No model was called. A compact review presentation must preserve exact subject
binding and all necessary evidence; this observation does not approve a new schema
or weaken stale-result rejection.

Independent review and human resolution remain pending. A first depth-1 local
clone produced no usable setup; the runner reported a missing helper. The successful shared clone and
exact checkout were disposable; neither setup attempt changed the source checkout.
