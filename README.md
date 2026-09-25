# OpenWarrant

**A document standard and tools for clear, traceable AI-assisted work.**

OpenWarrant connects a request, its limits, the work performed, and the evidence
used to accept the result.

Its goal is to reduce developer effort while making AI-assisted code safer to
use in real projects.

## The problem

Work passes between people, agents, documents, and tools. Each transfer can lose
context. Requirements can change without a clear record. A task marked “done”
can lack evidence that it meets the request.

OpenWarrant gives these tools a shared way to describe work and its history.
Developers can spend more time on decisions and less time copying context,
updating records, and coordinating agents.

## Project status

This repository contains Rust libraries and the `war` CLI for the existing
OpenWarrant format.

The document format and toolchain described below are the proposed redesign.
Their design is still in progress. The existing accepted specification and
repository rules remain in force.

See the [governing specification](docs/sas/WAR_Software_Architecture_Specification.md),
[record status](docs/warrants/generated/CORPUS_STATUS.md),
[object definitions](docs/DEFINITIONS.md), and [change history](CHANGELOG.md).
The definitions explain the current levels and when to write a SAS or a Warrant.

## What is a Warrant?

WAR means **Work Authorization Record**.

A Warrant describes one outcome that someone can review, such as “add password
reset.” It can connect that outcome to:

- Scope and explicit constraints.
- Architecture specifications and decision records.
- Implementation guidance.
- Test inputs and expected results.
- Permissions, questions, evidence, and acceptance records.

One Warrant can contain several implementation stages.

A small draft can be a valid document before it has tests, approval, or results.
Document validity, permission to execute, and accepted completion are separate
states.

The planned primary source format is readable Markdown with a small structured
metadata header and defined sections. The exact format is still being specified.

## Architecture and development phases

OpenWarrant owns the document structure and the meaning of its fields. Tools
that use the standard choose their own drafting and execution workflows.

We will build the supporting tools in three phases:

| Phase | What we build |
| --- | --- |
| **1. Feature/library** | Reusable operations and their tests. Every scoped feature must run through a file or command before the next phase starts. |
| **2. CLI and Compiler** | Commands that expose those operations, followed by interactive authoring and improved command use. The compiler reads documents, checks them, and produces context for callers. |
| **3. Workflow** | Applications and services that consume compiler output, run work, display progress, and collect decisions. Knowledge Fabric Compiler is the first integration. |

The libraries and OpenWarrant compiler must work on their own.

In the Workflow phase, Knowledge Fabric Compiler supplies inputs to the
OpenWarrant compiler and uses its returned output within a larger process.
Other applications can use the same boundary.

## What the compiler does

The compiler assembles master context from source documents and repository
inputs. Users edit facts at their source, then regenerate the assembly.

From that assembly, the compiler produces a **projection**: a smaller view that
contains the context needed for a particular task.

It must:

- Preserve applicable binding rules exactly, with their source revisions.
- Allow background context to be shortened without changing those rules.
- Identify missing required inputs.
- Produce reproducible results from complete, valid inputs without an AI call.

AI can help draft documents, suggest links, and prepare background summaries.
Compilation itself must not depend on a model.

## Planned workflows

Users can draft documents by hand, through interactive or noninteractive
commands, or with an agent using an OpenWarrant skill.

The reviewed workflow is:

**Prepare → approve → execute → verify → accept.**

The planned tooling can connect an existing agent or launch a configured coding
agent command. Each Warrant gets a separate Git worktree, with one writer at a
time. The agent harness supplies the execution sandbox.

During work, agents can send questions to other agents or authorized humans.
A pending decision pauses affected work and its dependents. Independent work
can continue.

A local API and native terminal interface are planned workflow surfaces.
A browser interface follows. These interfaces come after the library and
compiler phases.

## Fast prototypes and reviewed results

Humans can grant agents permission to work automatically. Finished prototypes
can remain unreviewed until review is needed.

A separate, optional assurance mark will identify results that meet a versioned
OpenWarrant baseline and receive human review and acceptance. Its name and full
requirements are still being defined.

The mark covers one Warrant result, its exact code revision, and its stated
scope. It does not qualify an entire repository.

Human acceptance is required to sign a Warrant as verified or complete.
Passing tests or finishing agent execution alone does not earn that status.

A prototype may qualify later, provided its actual history remains recorded.
Repositories can require stronger checks than the common baseline.

## Build and inspect the current tool

Use the Rust toolchain pinned in `rust-toolchain.toml`.

```bash
git clone https://github.com/Quitetall/OpenWarrant
cd OpenWarrant
cargo build --release -p openwarrant-cli
./target/release/war --help
```

To check this repository’s current records and generated files:

```bash
./target/release/war check --generated
```

This validates records and detects generated-file drift. It does not establish
that implementation work passed its required tests or received acceptance.

The current storage format uses manifests and authored atoms, with generated
Markdown and JSON views. Do not edit generated files.

See [QUICKSTART.md](QUICKSTART.md) for the existing workflow and
[AGENTS.md](AGENTS.md) before using an agent in this repository.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) for development checks and
[the threat model](docs/THREAT_MODEL.md) for security boundaries.

## License

[Apache-2.0](LICENSE). Versions distributed before the 2026-09-12 relicensing
remain available under AGPL-3.0-or-later. See [RELICENSING.md](RELICENSING.md).
