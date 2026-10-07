# Implement a planned Warrant

Ordinary coding needs no Warrant; this is for work someone planned as one.

1. Read the Warrant's scope, the repository's instructions, existing changes,
   and `war pins --resolved-only` for files a closed Warrant pins. A missing
   signature is no reason to wait: implementation goes ahead, and the
   sign-off follows whenever a person gives it.
2. Use an isolated worktree per Warrant when several writers share files.
   Implement the scope, run positive and refusal tests, and keep notes. Ask
   about material changes of scope or architecture; carry on with independent
   work meanwhile.
3. Use the commands the installed CLI has. `war perform`, `submit` and
   `resolve` apply their own records' rules; where one refuses, name the rule
   it refused with and carry on with the rest of the work.
4. At the end, record what was done: scope, revision, tests, notes and next
   steps. `war progress --html` writes the human view; see the
   [progress guide](progress.md). Lead with the configured completion word
   only when the scope is complete. If the tracker cannot record something
   yet, say so beside the notes.
5. Report finished work as finished and unverified. Verification needs an
   independent check and a person's acceptance; the next requested Warrant
   starts from a new prompt, with no extra approval round.

An interrupted harness, a pending dependent action or a failed required check
is partial work: record where it stands and where to resume.
