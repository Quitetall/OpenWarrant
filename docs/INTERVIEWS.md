# Resume a planning interview

```sh
war plan interview <intake-key-or-warrant-id>
war plan interview <intake-key-or-warrant-id> --json
```

The read-only view lists unanswered blocking questions, highest known information
gain first, then question ID. `war interview` is the compatible flat spelling.
It prints the command for answering each question. An answer grants no authority;
the existing question store's responder requirements still apply.

An intake's supplied answers apply only to that intake and matching question
text. They cannot hide a later Warrant question that happens to reuse its ID.
Missing or invalid optional request/proposal metadata leaves questions visible.
Metadata must be regular repository files; links and special files are refused.

`recorded_question_count` counts retained question records, including answered
and nonblocking ones. If the question store could not be read completely, the
count is null (unknown), never a partial count reported as complete. This does
not measure questions asked in external chat.

Draft application records `plan/interview-metrics.json`: retained question count,
supplied request-answer count and an explicitly unknown external clarification
count. Blocked intakes retain their request, proposal, questions and the same
bounded metrics so a later interview can resume. Planning still produces the
existing Draft Proposal and never authorizes or independently verifies work.
