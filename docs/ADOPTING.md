# Adopting OpenWarrant in a repository with history

This page starts from a repository that already has commits — perhaps
thousands — and has not been set up with OpenWarrant (OW-WAR-0124).
Choose routine ticket tracking below, or the optional governed setup in
section 1. The signing steps later on this page belong to governed setup.

## Routine work: track a change without signing

From the existing repository:

```bash
war init --namespace ACME --name "Acme"
war create "First tracked change" --item "Implement and test the change"
war view prime
```

Plain `war init` asks no questions and uses the vibe preset. This path needs
no SSH key, signer registration or SAS approval. The commands create records;
your connected agent performs the work. They do not award verification.
Existing repository access restrictions and explicit verified-start requirements
still apply to the work they cover.

Use [the ticket loop](TICKETS.md) to claim and finish the work. If the project is
already initialized, use its existing setup and start with `war create`.
[The quickstart](../QUICKSTART.md) also covers prompt-only Warrant records and
later sign-off. A ticket enters the authority layer only through explicit
promotion; ordinary implementation does not require that step.

## 1. Optional governed setup: record the baseline

The remaining steps create the SAS, authority examples and first Warrant for
sign-off. Use this setup instead of the routine initialization above when you
want that scaffold from the start. Human steps are marked **HUMAN**;
everything else an agent may run. The baseline records where tracking begins
and claims nothing about what came before.

```bash
cd your-repository
war init --program "Acme" --namespace ACME
```

Because the repository has commits, `war init` records where OpenWarrant
began in `openwarrant.toml`:

```toml
[adoption]
baseline = "<the full id of HEAD>"
```

and prints it with the number of commits in its history. To start from an
earlier or later point in the history, name it:

```bash
war init --program "Acme" --namespace ACME --baseline v2.3.0
```

A `--baseline` that is not a commit in HEAD's history is refused, and
nothing is written. At a terminal, `war init --guided` asks
instead: after the program's name it shows the commit count and HEAD, and
asks you to confirm or name another commit.

If the directory is not a git repository yet, `war init` runs `git init`
first and says so. Inside an existing repository — a subdirectory of one
included — it never creates a nested one. Without git installed it
continues and says that Warrant identity and journal history cannot be
checked until the directory is a git repository.

## 2. What the baseline claims, and what it does not

The baseline is configuration, not an authority record. It says:

- **Nothing before it is claimed, owned or verified by any Warrant.** The
  first Warrant's Basis (`ACME-WAR-0001/atoms/20-basis.md`) names the
  baseline and says exactly this. Past commits are history in their actual
  state; no Warrant authorized them.
- **No file from before it is pinned.** `war pins` lists only paths a
  resolved Warrant delivered; a file committed before adoption is owned by
  no Warrant (§37.5) until an authorized Warrant declares it.
- **Untracked work counts from it.** `war telemetry` lists commits after
  the baseline whose subject cites no Warrant in this repository's
  namespace (`ACME-WAR-NNNN`, or a `war://` reference), and names the
  baseline it read from. A commit citing another namespace's alias is still
  a candidate here.

It does not say the history was reviewed, that it is correct, or that any
obligation is established by it. It does not stop anyone from editing the
baseline later: a moved baseline hides commits from untracked-work
detection, which is why `war telemetry` prints the baseline beside every
count it reports.

A repository with no commits gets no `[adoption]` table, and `war init`
behaves exactly as it does in `QUICKSTART.md`.

## 3. Existing decision records

If `war init` finds ADR files named `NNNN-*.md` under `docs/adr`, `doc/adr`
or `adr`, it prints the command that would import them:

```bash
war migrate --corpus docs/adr --commit <baseline>
```

It runs nothing. Importing is a separate, deliberate act (§96): the import
records each decision's bytes at that commit and fabricates no proof of
completion. A corpus somewhere else is not found; point `war migrate
--corpus` at it yourself.

## 4. Who signs in governed setup

```bash
cp docs/authority/roles.toml.example docs/authority/roles.toml               # HUMAN: your name, your roles
cp docs/authority/allowed_signers.example docs/authority/allowed_signers     # HUMAN: your key
```

Or let `war init --guided` at a terminal write both from your answers, once;
`war doctor --fix-signing` offers the same for a file that is still missing.

## 5. The SAS

```bash
$EDITOR docs/sas/Acme_SAS.md     # the three sections the tool reads; the rest is yours
war sas propose 0.1.0            # records the document's digest and its §106 rows
war sign 0.1.0 --ssh-sign        # HUMAN: accept it
```

## 6. The first Warrant

```bash
war check ACME-WAR-0001 && war compile && war check --generated
war authorize ACME-WAR-0001      # the request: what a signature would mean
war sign ACME-WAR-0001 --ssh-sign # HUMAN: authorize it, one dialog
```

`war resolve --dry-run ACME-WAR-0001` then reports what still blocks
closure. On a fresh adoption no obligation is established: the history
before the baseline establishes none of them.

## What the owner signs

Five acts are a human's and only a human's: accepting the SAS, authorizing
a Warrant, resolving it, correcting a resolved Warrant's delivered file,
and invalidating a Gate Definition version (which disputes every
resolution resting on it; docs/INVALIDATION.md). Adoption adds none. The baseline is written by `war init`, not
signed, because it is a statement about where the record starts — not an
authorization of anything before or after it.
