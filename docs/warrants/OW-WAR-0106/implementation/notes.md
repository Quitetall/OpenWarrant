# Hotline implementation progress

The first library component defines exact question and answer records. Questions
bind the controller-provided source identity, policy digest and observed committed
checkpoint. Answer authentication derives the responder from a configured token
hash; request bodies cannot nominate their own actor. Governing response scope is
explicit per Warrant. A direct-human request additionally requires a configured
human responder. Delegated AI governing scope does not award a human signature or
qualification. Empty responder configuration grants nobody permission.

Four direct contract tests cover valid answer binding, mutable-input isolation,
stale subject and policy differences, actor injection, credential refusal,
governing scope, direct-human distinction, invalid checkpoints, qualification
injection, duplicate credentials and wildcard scope refusal.

These are library observations only. HTTP storage, responder routing, browser
controls, checkpoint/resume lifecycle, cumulative budgets and integration tests
remain unfinished. No runtime completion or security deployment is claimed.
Bearer credential custody belongs to the configured harness/operator; a token
hash is not proof of human presence or secure acceptance.
