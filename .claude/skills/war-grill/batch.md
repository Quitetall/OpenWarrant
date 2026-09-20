# Batch open Warrant questions

1. Run `war questions --open --json` in the target repository. Inspect the report
   as well as the question list. If records are unreadable, report the exact gaps;
   do not describe the partial list as every open question.
2. Present every returned question in one numbered review round, preserving its
   Warrant alias, question ID, stage, blocking status and recorded recommendation.
   Group by Warrant. Keep questions distinct even when their wording matches.
   If the harness limits question widgets, use one readable batch document and
   ask for answers keyed by Warrant and question ID. Do not silently truncate.
3. Reuse explicit answers already supplied by the user; ask only unresolved items.
   Preserve unanswered items. Technical advice does not grant governing authority.
4. For legacy human-only answers, prepare the exact `war answer` commands for the
   human using the live CLI help. Never impersonate the responder or bypass actor
   checks. Retain proposed answers as drafts until the authorized act is recorded.
5. Reread the open queue. Report which answers were recorded, which remain drafts,
   and which questions remain open. Completion means every initial question is
   accounted for; it does not mean every question was answered or work authorized.
