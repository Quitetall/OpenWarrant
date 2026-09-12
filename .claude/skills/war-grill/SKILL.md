---
name: war-grill
description: Interview the user relentlessly about a request until every design branch is settled, then record the answers in the draft request and run the drafter. Use when the user says grill me, wants a plan stress-tested, or before `war plan --draft` on a vague sentence.
disable-model-invocation: true
---

# war-grill

Grilling after mattpocock/skills `productivity/grilling` (3cca18b, MIT), with
one change: the answers become a record. Every settled decision lands in the
draft request's `answers` (SAS §74.6) and the drafter reads them; nothing
settled lives only in chat.

## The interview

Map the request as a **design tree**: every decision branches into the
decisions that hang off it. Work it in **rounds**. The **frontier** is every
decision whose prerequisites are settled, the questions you can ask now
without guessing at answers you have not heard. Ask the whole frontier in one
round, numbered, each with your recommended answer, then wait.

```
❓ **Q1** - **<title>**: <body, choices where they help>

➡️ <your recommended answer>

---

❓ **Q2** - ...
```

Each answer reshapes the tree; recompute the frontier and ask the next round.
A question whose answer depends on one still open this round waits for the
next. Facts are yours to find (the corpus, `war status --json`, `CONTEXT.md`,
the code); decisions are the user's. Read `CONTEXT.md` first and use its
words; when the user's word conflicts with it, say so and settle the term.

Done when the frontier is empty and nothing is silently assumed. Do not draft
until the user confirms the shared understanding.

## The record

1. Write the verbatim request sentence first (the work order rule: a request
   rewritten to suit the agent has not tested the vague-request claim).
2. Run the drafter with every settled decision as an answer:

```bash
war plan "<the sentence, verbatim>" --draft \
  --answer Q1="<the user's answer>" --answer Q2="<...>" \
  --out docs/warrants/generated/draft-<slug>.json
```

3. Report the §74.4 result. `plan.interview-required` means a blocker you did
   not settle: ask it, do not answer it yourself.
4. Stop. `--apply --reviewed` is the human's step; say so and hand over the
   proposal path.

## Never

A skill never signs, and never claims a step it did not run. A grilling
agent that answers its own questions has broken the interview.
