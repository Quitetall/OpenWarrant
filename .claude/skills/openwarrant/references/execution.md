# Execute a bounded Warrant

1. Read exact Warrant scope, applicable host instructions, context dependencies,
   existing changes and delivery pins. Identify real input prerequisites, explicit
   action gates and spend/retry limits. An absent qualification signature is not
   a universal execution blocker for prompt-only work.
2. Use an isolated worktree per Warrant and serialize writers. Prepare the selected
   checks, implement the requested scope, run positive/refusal tests and preserve
   progress. Ask about material scope/architecture changes; continue independent work.
3. Use only supported runtime operations. Current legacy `war perform`, `submit`
   and `resolve` enforce legacy contracts. For an explicitly permitted prototype,
   work through the harness and retain implementation notes; do not fabricate a
   legacy authorization/resolution to make those commands accept it. If legacy
   pins or explicit action gates block a write, preserve them and name that boundary.
4. At the declared work stop, record actual completed scope, exact revision, tests,
   implementation notes, document trail and next steps. Generate the human-facing HTML progress view with `war progress --html`;
   use the [progress guide](progress.md) for attributed work reports and live refresh. Return configured completion word first only when that scope is
   complete, followed by concise pointers. If no word is configured, omit it.
   If tracker cannot represent prototype completion yet, disclose that limitation
   beside the notes; never turn a chat assertion into a fabricated tracker record.
5. Report unverified completion distinctly. Qualification requires independent
   evidence and secure human acceptance; continue the next requested Warrant from
   a new prompt without inventing another approval ceremony.

A harness interruption, pending dependent action or failed required check is not
completion. Record the partial state and recovery point instead of a completion word.
