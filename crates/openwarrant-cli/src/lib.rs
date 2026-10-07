// SPDX-License-Identifier: Apache-2.0
//! `war` — the OpenWarrant command line interface (SAS §70–§76).

#![forbid(unsafe_code)]

use std::process::ExitCode;

use camino::Utf8PathBuf;
use clap::{Parser, Subcommand};
use openwarrant_core::Profile;

pub mod acceptance;
pub mod alias;
pub mod amendment_id;
pub mod attest;
pub mod authority_check;
pub mod authority_cmd;
pub mod authorize;
pub mod batch_cmd;
pub mod blut;
pub mod board;
pub mod bonsai;
pub mod bridge;
pub mod build_identity;
pub mod bundle;
pub mod check;
pub mod commit;
pub mod compile;
pub mod console;
pub mod context_select;
pub mod contract_history;
pub mod corpus;
pub mod correct;
pub mod deliver;
pub mod diagnostic;
pub mod diff_target;
pub mod dispatch;
pub mod dispatch_bundle_cmd;
pub mod doctor;
pub mod document;
pub mod eval;
pub mod eval_ordinary;
pub mod evidence;
pub mod export;
pub mod frontier;
pub mod gate_cmd;
pub mod host;
pub mod impact;
pub mod inbox;
pub mod init;
pub mod install;
pub mod instructions;
pub mod interop;
pub mod invalidation;
pub mod journal_cmd;
pub mod kf;
pub mod mark;
pub mod mcp;
pub mod migrate;
pub mod model;
pub mod new;
pub mod next;
pub mod notice;
pub mod notify;
pub mod official;
pub mod output;
pub mod overview;
pub mod ownership;
pub mod perform;
pub mod pins;
pub mod plan;
pub mod pr_gate;
pub mod preflight_cmd;
pub mod prepare;
pub mod preservation;
pub mod preset;
pub mod progress;
pub mod progress_viewer;
pub mod projects;
pub mod questions;
pub mod records;
pub mod relations;
pub mod release_cmd;
pub mod remedy;
pub mod render_cmd;
pub mod repo;
pub mod resolution_cmd;
pub mod resolve;
pub mod roadmap_cmd;
pub mod roadmap_edit;
pub mod run_cmd;
pub mod sas;
pub mod sas_repin;
#[cfg(feature = "schema")]
pub mod schema_typescript;
#[cfg(feature = "schema")]
pub mod schemas;
pub mod sdk;
pub mod show;
pub mod sign;
pub mod signing_probe;
pub mod skew;
pub mod standing_cmd;
pub mod states;
pub mod status;
pub mod telemetry;
pub mod ticket;
pub mod timeline;
pub mod tui;
pub mod verify;
pub mod vfs;
pub mod warrants;
pub mod watch;
pub mod webui;

#[derive(clap::Subcommand, Debug)]
enum KfCommand {
    /// Read KF's health. The only call here that cannot mutate.
    Health {
        #[arg(long, default_value = "http://127.0.0.1:4000")]
        base: String,
    },
    /// POST a §67 typed action. WRITES to an authoritative external record.
    Act {
        #[arg(long, default_value = "http://127.0.0.1:4000")]
        base: String,
        /// e.g. `document.create`.
        action_type: String,
        #[arg(long)]
        actor: String,
        #[arg(long)]
        acting_role: String,
        #[arg(long)]
        organization: String,
        #[arg(long)]
        reason: String,
        /// Supplied by the CALLER; KF refuses fewer than 8 characters.
        #[arg(long)]
        idempotency_key: String,
        /// What the action targets. KF validates these; an empty envelope
        /// would 400 at the server, which tells the caller nothing useful.
        #[arg(long = "target-id")]
        target_ids: Vec<String>,
        /// The action payload, as JSON.
        #[arg(long, default_value = "{}")]
        payload: String,
        /// Required. Without it this refuses rather than writes.
        #[arg(long)]
        confirm_write: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
enum UiCommand {
    /// The devices paired with `war view ui --lan` for this repository, from the
    /// per-user device file (never a record).
    Devices {
        /// Revoke one device by id; its next request is refused.
        #[arg(long)]
        revoke: Option<String>,
    },
}

#[derive(clap::Subcommand, Debug)]
enum RoadmapCommand {
    /// Write the roadmap ref on an UNSIGNED Warrant. A signed one is refused
    /// by name: its ref is inside the contract digest.
    Assign {
        alias: String,
        /// `OW-PHASE-3` or `OW-PHASE-3/slug`.
        phase: String,
    },
    /// Record the roadmap atoms as they stand as the next revision; a human
    /// then accepts it with `war sign roadmap --ssh-sign`.
    Propose {
        /// What you changed, in a line; shown on the signing screen as your
        /// statement beside the computed diff.
        #[arg(long)]
        note: Option<String>,
    },
    /// Edit the phases by keystroke, at a terminal, and sign once. No file is
    /// opened; the atom is written, the revision proposed, and one
    /// `war sign roadmap --ssh-sign` raises the dialog.
    Edit,
}

/// `war sign standing` (OW-ADR-0029): a class of routine work one human
/// signature pre-authorizes.
#[derive(clap::Subcommand, Debug)]
enum StandingCommand {
    /// Validate a class file and place it where `war sign` offers it for one
    /// signature. It covers nothing until a human signs it:
    /// `war sign standing:<id>@<revision> --ssh-sign`.
    Propose {
        /// A `oh.war/standing-authorization/v1` TOML file.
        file: Utf8PathBuf,
        /// Validate and say what would be written; write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Every class (or one): its terms' state, who signed it and when, each
    /// glob with what it matches today, the count used and the expiry.
    Show {
        /// A class id, `standing://<id>@<rev>` or `standing:<id>@<rev>`.
        id: Option<String>,
    },
    /// The coverage check: inside the class, write this Warrant's
    /// authorization with the class's signer as authorizer; outside it,
    /// refuse by the term broken and write nothing.
    Apply {
        alias: String,
        /// The class; defaults to the manifest's `[standing] ref`.
        #[arg(long = "class")]
        class: Option<String>,
        /// Run every refusal and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(clap::Subcommand, Debug)]
enum SasCommand {
    /// Record the document as it stands as a proposed revision (§101.2).
    Propose {
        /// e.g. `0.1.0-draft.1`, `1.0.0`.
        version: String,
    },
    /// Emit the acceptance request, or ingest a human's signed response.
    Accept {
        version: String,
        /// A signed response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
    },
    /// §106 of the document versus a candidate document; refuses a removed id.
    Diff { candidate: Utf8PathBuf },
    /// Every revision on record, and which one the document matches.
    Status,
    /// Write the amendment that re-pins an authorized, unresolved Warrant to
    /// the latest recorded revision (OW-ADR-0016); a human then re-authorizes
    /// it. `sas.pin-superseded`'s remedy (OW-WAR-0112).
    Repin {
        /// One Warrant. Refuses a resolved one by name.
        alias: Option<String>,
        /// Every authorized, unresolved Warrant behind the latest revision;
        /// resolved ones are skipped. All-or-nothing.
        #[arg(long, conflicts_with = "alias")]
        all: bool,
        /// The amendment's reason, instead of the one the tool writes.
        #[arg(long)]
        reason: Option<String>,
        /// Say what would be written and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
}

/// `war evidence gate <action>` (OW-WAR-0136).
#[derive(clap::Subcommand, Debug)]
enum GateAction {
    /// Invalidate one Gate Definition version (§45, RQ-057). With
    /// `--grounds`: the request — its definition digest, the grounds, every
    /// resolution the sweep would dispute, transitively, by alias, and who may
    /// sign. Writes nothing. With `--response`: ingest a signed invalidation
    /// (`war sign <gate>@<version> --grounds … --ssh-sign` does both halves),
    /// writing the record and one dispute per reached resolution. An agent,
    /// the performer, and an unsigned response are refused.
    Invalidate {
        /// `<gate_id>@<version>`.
        gate: String,
        /// Why the definition is invalid (§56.4 grounds).
        #[arg(long, required_unless_present = "response")]
        grounds: Option<String>,
        /// A signed `oh.war/invalidation-response/v1` to ingest.
        #[arg(long, conflicts_with = "grounds")]
        response: Option<Utf8PathBuf>,
    },
}

#[derive(clap::Subcommand, Debug)]
enum BonsaiCommand {
    /// Check a Warrant-bound change with a fixed Bonsai binary.
    Check {
        /// Warrant authorizing this change.
        #[arg(long)]
        warrant: String,
        /// Merge-base commit for the candidate diff.
        #[arg(long)]
        base: String,
        /// Candidate commit. It must be checked out at HEAD.
        #[arg(long)]
        head: String,
        /// Explicit Bonsai binary. The Warrant never supplies an executable.
        #[arg(long, value_name = "BONSAI_BINARY")]
        bonsai: Utf8PathBuf,
    },
    /// Validate a completed passing Bonsai evidence document without rerunning Bonsai.
    VerifyEvidence {
        /// Repository-relative evidence document emitted by `war admin bonsai check`.
        #[arg(long)]
        evidence: Utf8PathBuf,
    },
}

/// Exit codes. §76.4 wants machine-usable output; a caller distinguishing
/// "your input was wrong" from "the Warrant is not sound" needs more than 0/1.
///
/// Codes are added when a command can actually produce them. An exit code the
/// binary never returns is a promise to callers that nothing keeps.
pub const EXIT_OK: u8 = 0;
pub const EXIT_DIAGNOSTIC: u8 = 1;
pub const EXIT_NOT_READY: u8 = 2;

#[derive(Parser)]
#[command(
    name = "war",
    about = "Work Authorization Records — author, check, and compile Warrants.",
    // OW-WAR-0143: the bare `war <version>` is a release build's alone; every
    // other build names its class, commit and dirty flag.
    version = build_identity::version_text(),
    long_about = None,
)]
pub struct Cli {
    /// Machine output (SAS §76.4): one `oh.war/report/v1` envelope on stdout,
    /// for every command. Errors are envelopes too.
    #[arg(long, global = true)]
    json: bool,
    /// The repository to act on. Defaults to the nearest ancestor of the
    /// current directory holding `openwarrant.toml`. A command that reads no
    /// repository (`init`, `sdk`, `install`) ignores it.
    ///
    /// A flag and not an environment variable on purpose: which repository was
    /// checked is a fact a reader should be able to recover from the process
    /// table, the journal and the gate argv, not one the ambient environment
    /// can change out from under them.
    #[arg(long, global = true, value_name = "ROOT")]
    root: Option<Utf8PathBuf>,
    /// None: `war` alone. At a terminal that is the app (OW-WAR-0112); on a
    /// pipe it is a refusal by name, so no script ever meets a full-screen
    /// application by accident.
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum EvalCommand {
    /// Run every task (or one) and write `evals/results/<sha>-<drafter>.json`.
    Run {
        /// One task id under the tasks directory.
        #[arg(long)]
        task: Option<String>,
        /// The drafter argv, one element per flag (repeat --drafter). Falls
        /// back to `[plan] drafter_argv`.
        #[arg(long = "drafter", value_name = "ARGV")]
        drafter: Vec<String>,
        /// The verifier argv, one element per flag. Falls back to
        /// `[verify] verifier_argv`.
        #[arg(long = "verifier", value_name = "ARGV")]
        verifier: Vec<String>,
        /// Where to write the result (canonical JSON; timings beside it).
        #[arg(long, value_name = "FILE")]
        out: Option<Utf8PathBuf>,
        /// Keep each scratch program for inspection.
        #[arg(long)]
        keep: bool,
        /// The tasks directory.
        #[arg(long, default_value = eval::DEFAULT_TASKS_DIR, value_name = "DIR")]
        tasks_dir: Utf8PathBuf,
    },
    /// Compare a result against the committed baseline, one rung per task.
    Verify {
        /// The result file to compare.
        result: Utf8PathBuf,
        #[arg(long, default_value = eval::DEFAULT_BASELINE, value_name = "FILE")]
        baseline: Utf8PathBuf,
    },
    /// Opt-in, costs tokens (M9): put a real agent in a scratch repository
    /// with OpenWarrant installed, ask it to fix a trivial bug, and fail if
    /// it refuses, asks for a Warrant, or edits nothing. Never run by the
    /// battery with a real agent. evals/ordinary/README.md has the how-to.
    Ordinary {
        /// The agent's argv, one element per flag (repeat --agent; an
        /// element may start with `-`, as in `--agent -p`). An element
        /// `{prompt}` is replaced by the scenario's sentence; without one
        /// the sentence goes to the agent's stdin.
        #[arg(long = "agent", value_name = "ARGV", allow_hyphen_values = true)]
        agent: Vec<String>,
        /// The scenario directory (`scenario.toml` and `repo/`).
        #[arg(long, default_value = eval_ordinary::DEFAULT_SCENARIO, value_name = "DIR")]
        scenario: Utf8PathBuf,
        /// Kill the agent after this many seconds (default: the scenario's).
        #[arg(long, value_name = "SECS")]
        timeout_secs: Option<u64>,
        /// Keep the scratch repository and the transcript for inspection.
        #[arg(long)]
        keep: bool,
    },
}

#[derive(Subcommand)]
enum DocumentCommand {
    /// Author a draft interactively; checkpoints remain on cancel or failure.
    Draft {
        #[arg(long)]
        draft_dir: Utf8PathBuf,
        #[arg(long)]
        output: Utf8PathBuf,
        #[arg(long)]
        resume: bool,
    },
    /// Review Markdown deliverables (the gate `document.review@1.0.0` runs this).
    Review {
        /// One Warrant; omit for every Warrant with a Markdown deliverable.
        alias: Option<String>,
    },
}

/// `war evidence kpi <action>` (OW-WAR-0148 M13).
#[derive(Subcommand)]
enum KpiCommand {
    /// Run every KPI of a Warrant (or one item's), parse the one number each
    /// prints, journal the run (value, time, commit), and say the latest,
    /// best and target. A command that prints no number is UNKNOWN.
    Run {
        /// A ticket (every KPI) or an item (the Warrant's and its own).
        target: String,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
}

/// `war admin bridge <harness>` (OW-WAR-0148 M13).
#[derive(Subcommand)]
enum BridgeCommand {
    /// Read a Claude Code task list and propose the item ticks it implies:
    /// each completed task that names an item (`t-x/i-y`) becomes `war done
    /// <item> --check` (claimed when the item has nothing to check). Prints
    /// what it would do; writes only with --apply.
    ClaudeTasks {
        /// The task list directory (default: `~/.claude/tasks/$CLAUDE_CODE_TASK_LIST_ID`).
        #[arg(long, value_name = "DIR", conflicts_with_all = ["file", "event"])]
        dir: Option<Utf8PathBuf>,
        /// One JSON file holding a task, an array of tasks, or `{"tasks": [...]}`.
        #[arg(long, value_name = "FILE", conflicts_with = "event")]
        file: Option<Utf8PathBuf>,
        /// A `TaskCompleted` hook's input (JSON), from a file or `-` for
        /// stdin: one completed task.
        #[arg(long, value_name = "FILE")]
        event: Option<String>,
        /// Tick what is proposed. Without it nothing is written.
        #[arg(long)]
        apply: bool,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
}

// ---- the command surface (OW-WAR-0148 M12; docs/COMMANDS.md) -------------
//
// `war --help` lists the daily verbs and, below them, the five groups. Every
// other spelling `war` ever had still parses: each group's members are
// flattened a second time into the top level and hidden there at run time
// ([`command`]), so `war board` and `war view board` are the same variant,
// run by the same code, and print the same bytes. Each group is its own
// enum: clap builds each in its own frame, and one enum holding every
// subcommand overflowed a test thread's stack while clap assembled it.

/// The daily verbs: the only commands `war --help` lists, in this order.
/// At most twelve (a plant counts them); `war start` (M15) is the twelfth.
pub const DAILY: &[&str] = &[
    "init", "create", "next", "claim", "done", "add", "note", "edit", "show", "status", "check",
];

/// The groups, named under "More:" in `war --help`, each with its purpose
/// (the first line of its own help).
pub const GROUPS: &[&str] = &["plan", "sign", "evidence", "view", "admin"];

/// Each group's members as `war <group> --help` lists them, aliases
/// included, for code that reads the words of a suggested command (the
/// remedy classifier, the app's sign button). A test holds these lists to
/// the clap tree. A `war sign` target is never one of the sign members:
/// `war sign <alias>` signs, `war sign authorize <alias>` drafts a request.
pub const GROUP_MEMBERS: &[(&str, &[&str])] = &[
    (
        "plan",
        &[
            "new",
            "promote",
            "render",
            "impact",
            "model",
            "state",
            "roadmap",
            "frontier",
            "questions",
            "ask",
            "answer",
            "answers",
        ],
    ),
    (
        "sign",
        &[
            "authorize",
            "resolve",
            "correct",
            "amend",
            "attest",
            "standing",
            "sas",
            "inbox",
            "authority",
            "approve",
            "release",
        ],
    ),
    (
        "evidence",
        &[
            "record", "gate", "verify", "prepare", "run", "perform", "submit", "kpi", "mark",
            "document", "eval",
        ],
    ),
    (
        "view",
        &[
            "ui", "tui", "board", "console", "watch", "overview", "progress", "warrants",
            "tickets", "ls", "ready", "prime", "timeline",
        ],
    ),
    (
        "admin",
        &[
            "compile",
            "doctor",
            "pins",
            "preflight",
            "diff",
            "journal",
            "deliver",
            "dispatch",
            "dispatch-bundle",
            "commit",
            "heartbeat",
            "release",
            "renumber",
            "agents-md",
            "import",
            "export",
            "migrate",
            "archive",
            "bridge",
            "host",
            "sdk",
            "mcp",
            "kf",
            "telemetry",
            "bonsai",
            "blut",
            "projects",
            "update",
            "version",
            "schemas",
            "merge-ticket",
            "preset",
        ],
    ),
];

/// Whether `word` names a member of `group` (`group_member("sign",
/// "authorize")`), so the words after `war <group>` read as that member.
#[must_use]
pub fn group_member(group: &str, word: &str) -> bool {
    GROUP_MEMBERS
        .iter()
        .any(|(g, members)| *g == group && members.contains(&word))
}

/// The daily loop (OW-WAR-0147; OW-WAR-0148 M10, M12; docs/TICKETS.md):
/// `war create`, `war claim`, `war done`, and the reads around them. A
/// Warrant made here is in its light encoding (what earlier releases called
/// a ticket). None of these commands asks a human for anything or signs
/// anything.
#[derive(Subcommand)]
enum DailyCommand {
    /// Set up this repository: openwarrant.toml, record directories, AGENTS.md.
    ///
    /// Initialize repository configuration and directories (§71.1). Asks
    /// nothing and writes no signing setup: `openwarrant.toml`, the record
    /// directories and AGENTS.md. Without `--namespace` one is derived from
    /// the directory name. `--program` adds the sign-off scaffold (a SAS,
    /// authority examples, a first Warrant); `--guided`, at a terminal, walks
    /// the whole setup as a conversation (OW-WAR-0112).
    Init {
        /// Namespace prefixing every local alias, e.g. `OW` in `OW-WAR-0001`.
        /// Derived from the directory name (or `--program`'s name) when not
        /// given: one word is that word, uppercased, several their initials.
        #[arg(long)]
        namespace: Option<String>,
        /// Project name. Defaults to the directory name.
        #[arg(long, conflicts_with = "program")]
        name: Option<String>,
        /// Scaffold a whole program for sign-off: a SAS the tool reads, the
        /// authority examples, the `war check` gate, and a first Warrant with
        /// real atoms. `war check` on the result exits 0.
        #[arg(long, value_name = "PROGRAM")]
        program: Option<String>,
        /// Ask the setup questions at a terminal: who signs, the SAS, the
        /// first Warrant. Refused without a terminal; plain `war init` asks
        /// nothing.
        #[arg(long, conflicts_with_all = ["non_interactive", "namespace"])]
        guided: bool,
        /// Never ask. Plain `war init` already asks nothing; the flag is kept
        /// so scripts that pass it keep working.
        #[arg(long)]
        non_interactive: bool,
        /// Where governed work begins in a repository with history: the
        /// commit recorded as `[adoption] baseline` (OW-WAR-0124). Defaults
        /// to HEAD when there are commits; refused, with nothing written,
        /// unless it names a commit in HEAD's history.
        #[arg(long, value_name = "COMMIT", conflicts_with = "guided")]
        baseline: Option<String>,
        /// The vibe preset (docs/PRESETS.md): no signatures, no PR gate, and
        /// `war done` claims an unclaimed item itself. Without a preset flag,
        /// no preset is written and nothing changes.
        #[arg(long, conflicts_with_all = ["guided", "team", "regulated"])]
        vibe: bool,
        /// The team preset: the PR gate on, signatures batched at release,
        /// and the signing examples under docs/authority/.
        #[arg(long, conflicts_with_all = ["guided", "regulated"])]
        team: bool,
        /// The regulated preset: every tick observed, signatures at merge, a
        /// test (or a formal Warrant) from everyone, and the signing examples.
        #[arg(long, conflicts_with = "guided")]
        regulated: bool,
    },
    /// Create a Warrant (a ticket) and print its id: a title is enough.
    ///
    /// Workable at once: no signature.
    Create {
        /// What this work accomplishes, in one sentence. With `--issue`, the
        /// issue's title unless given.
        #[arg(required_unless_present_any = ["issue", "issue_file"])]
        title: Option<String>,
        /// A checklist item; repeat for more. Items can be added later with `war add`.
        #[arg(long = "item", short = 'i', value_name = "TEXT")]
        items: Vec<String>,
        /// Context, decisions, links: Markdown for the Warrant's description.
        #[arg(long, value_name = "MARKDOWN")]
        body: Option<String>,
        /// 0 (most urgent) to 4; default 2. `war view ready` lists urgent work first.
        #[arg(long, short = 'p', value_parser = clap::value_parser!(u8).range(0..=4))]
        priority: Option<u8>,
        /// Ask the configured `[plan] drafter_argv` to propose the items. Without a
        /// drafter this is refused and nothing is invented.
        #[arg(long)]
        draft: bool,
        /// With --draft: the drafter proposes typed records too, and the
        /// Warrant's items implement them (OW-WAR-0148 M7). The records are
        /// validated as authored ones are, then written to one record atom
        /// under `docs/records/<area>/`; a refused proposal writes nothing.
        #[arg(long, requires = "draft", conflicts_with_all = ["kind", "labels", "part_of", "implements", "issue", "issue_file"])]
        records: bool,
        /// With --records: the area under `docs/records/` the records land in.
        #[arg(long, value_name = "NAME", requires = "records")]
        area: Option<String>,
        /// What kind of work: one of the working form's `[fields] types`
        /// in profiles/ticket.toml (task, bug, feature, chore, epic as shipped).
        #[arg(long = "type", value_name = "TYPE")]
        kind: Option<String>,
        /// A label; repeat for more. Refused outside a closed label set.
        #[arg(long = "label", short = 'l', value_name = "LABEL")]
        labels: Vec<String>,
        /// The Warrant (an epic) this one is part of.
        #[arg(long = "part-of", value_name = "WARRANT")]
        part_of: Option<String>,
        /// Make the Warrant from GitHub issue <N>, read once through
        /// `[intake] fetch_argv`; the Warrant records the link. With
        /// `[intake.writeback]` set, finishing it comments on and closes the issue.
        #[arg(long, value_name = "N", conflicts_with = "issue_file")]
        issue: Option<String>,
        /// The same from `gh issue view <n> --json number,title,body,url` output in a file.
        #[arg(long, value_name = "PATH")]
        issue_file: Option<Utf8PathBuf>,
        /// An item implementing this record (`REQ-pr1`): its text is the
        /// record's first sentence and `(implements REQ-pr1)`. Repeatable.
        #[arg(long, value_name = "RECORD")]
        implements: Vec<String>,
        /// Who is acting (default: $OPENWARRANT_ACTOR, else `[project] performer`).
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// What is ready, and whose step it is.
    ///
    /// Ready items of light Warrants first (what `war view ready` lists),
    /// then an agent's acts, then the acts a person signs. With nothing
    /// tracked it says "nothing tracked; work freely": ordinary work needs
    /// no Warrant. A signing step is always a person's.
    Next,
    /// Take an item or a whole Warrant, so no other agent works it.
    ///
    /// An item (`i-...`, `t-.../i-...`) or a whole Warrant (`t-...`).
    /// Refused, by name, when someone else holds it.
    Claim {
        /// An item or Warrant id, or a unique prefix of one.
        target: String,
        /// Take a claim older than `[tickets] claim_ttl_minutes` (default 120). Journalled.
        #[arg(long)]
        steal: bool,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Tick an item done in its Warrant's checklist and release the claim.
    ///
    /// A Warrant reads done when every item is. A plain tick is `claimed`;
    /// `--check` runs the item's tests and ticks at `observed`.
    Done {
        /// An item id (or a Warrant with no items left open).
        target: String,
        /// What was done, for the next reader; written on the item's line.
        #[arg(long)]
        note: Option<String>,
        /// Write only if the item (or Warrant) is still at this revision, the
        /// one `war show --json` gave; refused `warrant.stale-revision`
        /// otherwise, naming the current one.
        #[arg(long = "if-rev", value_name = "DIGEST")]
        if_rev: Option<String>,
        /// Run the Warrant's tests and KPIs (and the item's own) first, and
        /// tick at `observed` only when every one that decides passes; the
        /// receipt is journalled. On a done item, raise its tick the same way.
        /// A failing check refuses the tick by name; one that could not run
        /// is UNKNOWN, never a pass.
        #[arg(long)]
        check: bool,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Add an item to a Warrant, or a test, KPI or milestone.
    ///
    /// Append an item to a Warrant's checklist, or attach an optional part to
    /// any Warrant, a title-only one included: a test, a KPI, a milestone
    /// (docs/TYPES.md, "Optional parts and the tick ladder").
    Add {
        /// The Warrant. For a test or KPI of one item, the item (`t-x/i-y`).
        #[arg(value_name = "WARRANT")]
        ticket: String,
        /// The item, one line. Optional when a part is given; given with a
        /// test or KPI, the part is the new item's.
        #[arg(required_unless_present_any = ["tests", "kpi", "milestone", "min"])]
        text: Option<String>,
        /// What the item waits on: an item of this Warrant, a Warrant, or `t-x/i-y`. Repeatable.
        #[arg(long, value_name = "ITEM|WARRANT")]
        after: Vec<String>,
        /// A test: a shell command whose exit code decides pass or fail.
        /// Repeatable. `war done <item> --check` runs it.
        #[arg(long = "test", value_name = "COMMAND")]
        tests: Vec<String>,
        /// The name of the one --test given (default `test-N`).
        #[arg(long, value_name = "NAME", requires = "tests")]
        name: Option<String>,
        /// A KPI by name: `--cmd` prints one number, `--direction max|min`
        /// says which way is better, `--target` and `--mode` are optional.
        #[arg(long, value_name = "NAME")]
        kpi: Option<String>,
        /// The KPI's command; it prints one number.
        #[arg(long, value_name = "COMMAND", requires = "kpi")]
        cmd: Option<String>,
        /// max or min: which way the KPI is better.
        #[arg(long, value_name = "max|min", requires = "kpi")]
        direction: Option<String>,
        /// The value the KPI passes at (at or above for max, at or below for min).
        #[arg(
            long,
            value_name = "N",
            requires = "kpi",
            allow_negative_numbers = true
        )]
        target: Option<f64>,
        /// best (the default: pass or fail against the target, best value
        /// kept), threshold (pass or fail only), optimise (a signal only;
        /// never decides a tick).
        #[arg(long, value_name = "MODE", requires = "kpi")]
        mode: Option<String>,
        /// A milestone: an item that ticks a marker on the progress tracker.
        #[arg(long, value_name = "TEXT")]
        milestone: Option<String>,
        /// The least the milestone's tick must show: claimed, observed,
        /// independent or signed. Alone, on an item, it makes that item a
        /// milestone.
        #[arg(long, value_name = "LEVEL")]
        min: Option<String>,
        /// Write only if the item (or Warrant) is still at this revision, the
        /// one `war show --json` gave; refused `warrant.stale-revision`
        /// otherwise, naming the current one.
        #[arg(long = "if-rev", value_name = "DIGEST")]
        if_rev: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Append a dated note to a Warrant: context for whoever comes next.
    ///
    /// For the next agent or person; Markdown, may span lines.
    Note {
        /// The Warrant (or one of its items).
        target: String,
        /// The note; Markdown, may span lines.
        text: String,
        /// Write only if the item (or Warrant) is still at this revision, the
        /// one `war show --json` gave; refused `warrant.stale-revision`
        /// otherwise, naming the current one.
        #[arg(long = "if-rev", value_name = "DIGEST")]
        if_rev: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Change a Warrant's type, labels, epic or priority.
    ///
    /// One line of its manifest each, journalled. Nothing else moves.
    Edit {
        #[arg(value_name = "WARRANT")]
        ticket: String,
        /// The new type; `none` clears it.
        #[arg(long = "type", value_name = "TYPE")]
        kind: Option<String>,
        /// Add a label; repeatable.
        #[arg(long = "label", short = 'l', value_name = "LABEL")]
        labels: Vec<String>,
        /// Remove a label; repeatable.
        #[arg(long = "unlabel", value_name = "LABEL")]
        unlabels: Vec<String>,
        /// The Warrant (epic) this one is part of; `none` detaches it.
        #[arg(long = "part-of", value_name = "WARRANT")]
        part_of: Option<String>,
        #[arg(long, short = 'p', value_parser = clap::value_parser!(u8).range(0..=4))]
        priority: Option<u8>,
        /// Write only if the item (or Warrant) is still at this revision, the
        /// one `war show --json` gave; refused `warrant.stale-revision`
        /// otherwise, naming the current one.
        #[arg(long = "if-rev", value_name = "DIGEST")]
        if_rev: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Show a Warrant by its id.
    ///
    /// A light one (`t-...`, or one of its items), a directory one (its
    /// alias, as one of §17.5's projections), or one read in place
    /// (`openspec:<change>`, `speckit:<feature>`).
    Show {
        /// The Warrant's id: `t-...`, an alias, `openspec:...` or `speckit:...`.
        #[arg(value_name = "ID")]
        alias: String,
        /// Which projection. Defaults to the full Warrant.
        #[arg(long, default_value = "full_warrant")]
        view: String,
    },
    /// Where the work stands, from the records.
    ///
    /// §17.5 `status`; §34.3; §98. Bare `war status` is the corpus projection. `war status <alias>` is the
    /// per-Warrant form §72.5 names. Every count is a ladder; nothing is a
    /// percentage.
    Status {
        /// A Warrant's id (an alias, `t-...`, `openspec:...`, `speckit:...`).
        /// Omit for the whole corpus. (`--json` is the global flag; for the
        /// corpus it yields the canonical projection.)
        #[arg(value_name = "ID")]
        alias: Option<String>,
        /// The corpus timeline (`oh.war/corpus-timeline/v1`) instead of the status.
        /// (One projection per call: `timeline` excludes `pending`, and both
        /// exclude an alias; clap checks conflicts in both directions.)
        #[arg(long, conflicts_with_all = ["alias", "pending"])]
        timeline: bool,
        /// The pending human acts (`oh.war/corpus-pending/v1`) instead of the status.
        #[arg(long, conflicts_with = "alias")]
        pending: bool,
    },
    /// Check the records deterministically, without any agent.
    ///
    /// §71.7. A Warrant's id checks that one; alone, the whole corpus.
    Check {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
        /// Also compare committed generated views against a fresh compilation.
        #[arg(long)]
        generated: bool,
        /// The PR gate (docs/PRESETS.md): read PR <NUMBER> through `gh api`
        /// and pass only when it cites a Warrant official at its author's
        /// level. This reads the network; a call that fails is UNKNOWN,
        /// never a pass.
        #[arg(long, value_name = "NUMBER", conflicts_with_all = ["alias", "generated"])]
        pr: Option<String>,
        /// With --pr: the repository, `owner/name` (default:
        /// $GITHUB_REPOSITORY, else what `gh repo view` names).
        #[arg(long, value_name = "OWNER/NAME", requires = "pr")]
        repo: Option<String>,
        /// With --pr: append the Markdown summary to this file
        /// ($GITHUB_STEP_SUMMARY in CI).
        #[arg(long, value_name = "FILE", requires = "pr")]
        summary: Option<Utf8PathBuf>,
        /// With --pr: post the summary as a comment on the PR through `gh`.
        #[arg(long, requires = "pr")]
        comment: bool,
    },
}

/// `war plan …`: shaping the work. Also flattened, hidden, into the top level.
#[derive(Subcommand)]
enum PlanCommand {
    /// Create a draft Warrant (§71.2) from a preset: typed atoms, each
    /// heading followed by the question it answers (OW-ADR-0022).
    New {
        /// The Warrant's title.
        title: String,
        /// The preset: `feature`, `fix` (delivery) or `decision`.
        #[arg(long)]
        preset: Option<String>,
        /// Composition profile (§16.3). Implied by the preset; naming one the
        /// preset does not compose is refused.
        #[arg(long)]
        profile: Option<String>,
        /// Cite this Warrant as the parent, at its latest authorized revision
        /// and that revision's digest (§20.2, OW-WAR-0123). A parent with no
        /// authorized revision is refused and nothing is created.
        #[arg(long, value_name = "ALIAS")]
        parent: Option<String>,
    },
    /// Draft a directory Warrant (delivery) from a light one, for when
    /// someone wants sign-off. Opt-in: the authority layer starts here, not
    /// before.
    Promote {
        #[arg(value_name = "WARRANT")]
        ticket: String,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Render a declared projection of records (OW-WAR-0148 M6): one set of
    /// records, many documents. `prd`, `architecture`, `test-plan` and
    /// `agent-packet` ship as document types (profiles/*.toml, `form =
    /// "document"`); a program declares its own. Prints the rendering
    /// (Markdown or JSON) and writes nothing; `--json` returns it as
    /// `oh.war/projection/v1`, every line traced to the record id and
    /// revision it came from. `war admin compile` writes the projections of the
    /// documents an area declares in `docs/records/<area>/documents.toml`.
    Render {
        /// The projection's name (`prd`, `architecture`, `test-plan`,
        /// `agent-packet`).
        projection: String,
        /// What to render: a declared document (`password-reset/prd`), a
        /// record area (`password-reset`), or one record (`REQ-pr1`). Omit
        /// when exactly one document of the projection's type is declared.
        #[arg(long = "of")]
        of: Option<String>,
        /// A byte budget, in place of the document's or projection's own.
        /// Over budget is refused by name; nothing is truncated.
        #[arg(long = "max-bytes")]
        max_bytes: Option<usize>,
    },
    /// What a change to one record affects (OW-WAR-0148 M3): the records that
    /// reach it through incoming relations, transitively; the Warrants,
    /// tickets and record atoms that hold or name them; the obligations that
    /// evaluate them, with verdicts bound to the revision they judged (a
    /// verdict on an older revision stays recorded and reads stale); the
    /// roadmap phases and generated views they feed. Read-only.
    Impact {
        /// A record id of `war plan model` (`REQ-pr1`, `OW-WAR-0001/OBL-002`,
        /// `t-3f2a/i-9c01`).
        record: String,
    },
    /// The compiled corpus as one document, `oh.war/model/v1` (OW-WAR-0148):
    /// every record with its revision and governor, every relation, the
    /// states the builders derive, and a diagnostic per relation whose target
    /// is not a record. Read-only; the same tree gives the same bytes.
    Model,
    // ---- OW-WAR-0148 M4: declared states ---------------------------------
    /// Enter a declared state on a record (OW-WAR-0148 M4): a refinement of
    /// a fixed kernel state that the record's profile declares in
    /// `[[states]]` (`in_review` refines `in_progress`). An authored event in
    /// the journal of the Warrant or ticket that owns the record. It holds
    /// only while its fixed parent holds and lapses when the parent stops;
    /// refining an authenticated state (`verified`), it is entered only while
    /// that state already holds. It satisfies no resolution check and no
    /// capability gate.
    State {
        /// A record id of `war plan model`: a ticket, an item (`t-x/i-y`, `i-y`),
        /// a Warrant or one of its records (`NS-WAR-0001/OBL-001`).
        record: String,
        /// The declared state's name.
        name: String,
        /// Why, for the next reader; kept in the journal entry.
        #[arg(long)]
        note: Option<String>,
        /// Who is acting (default: $OPENWARRANT_ACTOR, else `[project] performer`).
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// The roadmap: one per program, beside the SAS (OW-ADR-0023). Without a
    /// subcommand, its phases in dependency order with members, exits and
    /// whether each is achieved.
    Roadmap {
        #[command(subcommand)]
        command: Option<RoadmapCommand>,
    },
    /// The stages that can start now (OW-WAR-0068): open, unblocked by
    /// their milestone's `depends_on`, and not yet dispatched. Derived from
    /// the same records a resolution reads; never a status claim.
    Frontier {
        /// One Warrant; omit for every unresolved Warrant.
        alias: Option<String>,
    },
    /// Every question across the corpus, blocking and open first, each with
    /// the command that answers it (OW-WAR-0069).
    Questions {
        /// One Warrant; omit for all.
        alias: Option<String>,
        /// Only what awaits an answer.
        #[arg(long)]
        open: bool,
    },
    /// Ask the human a question that blocks a stage (OW-WAR-0069). An agent
    /// asks; only a human answers. Neither act authorizes anything.
    Ask {
        /// The Warrant's local alias.
        alias: String,
        /// The stage the answer unblocks; must exist in its milestones atom.
        stage: String,
        /// The question, in full.
        question: String,
        /// Your recommended answer, so it can be answered in one word.
        #[arg(long, default_value = "")]
        recommend: String,
        /// The stage cannot proceed without the answer.
        #[arg(long)]
        blocking: bool,
    },
    /// Answer a question an agent asked (OW-WAR-0069). Refused for an
    /// agent-kind actor by the register, and nothing is written.
    Answer {
        /// The Warrant's local alias.
        alias: String,
        /// The question id, e.g. `Q-001`.
        id: String,
        /// The answer, in full.
        answer: String,
        /// Who is answering; must hold a human role in roles.toml.
        #[arg(long = "as")]
        actor: String,
    },
    /// The answers a stage's performer should read before it starts.
    Answers {
        /// The Warrant's local alias.
        alias: String,
        /// One stage; omit for every stage of the Warrant.
        stage: Option<String>,
    },
}

/// `war sign …`: the human acts. Also flattened, hidden, into the top level.
#[derive(Subcommand)]
enum SignCommand {
    /// §28.4 authorization. Emits a request; ingests a human's signature.
    ///
    /// Like `verify`, this command never decides anything itself: §27.2 says an
    /// agent SHALL NOT authorize a proposed WAR, and ingestion refuses every
    /// agent named in a response regardless of what the response claims.
    Authorize {
        /// The Warrant's local alias.
        alias: String,
        /// A signed response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
    },
    /// Resolve a Warrant: emit the request, evaluate it, or ingest a signature.
    ///
    /// `--dry-run` evaluates §56.1's thirteen resolution requirements without
    /// recording one.
    Resolve {
        /// The Warrant's local alias.
        alias: String,
        /// Report the thirteen §56.1 requirements and stop.
        #[arg(long)]
        dry_run: bool,
        /// A resolver's signed response to ingest (§56.2). Without it and
        /// without --dry-run, the resolution REQUEST is emitted: what a
        /// signature would bind, which outcomes §38.6 permits, and who may sign.
        #[arg(long, conflicts_with = "dry_run")]
        response: Option<camino::Utf8PathBuf>,
    },

    /// Correct a delivered artifact of a RESOLVED Warrant (OW-WAR-0064): emit
    /// the request naming the pinned digest and the file's digest now, or
    /// ingest a human's signed correction. The pin in `deliverables.toml` is
    /// never edited; the correction is appended beside it and the superseded
    /// digest stays visible.
    Correct {
        /// The Warrant's local alias.
        alias: String,
        /// The deliverable id, e.g. `D-002`.
        deliverable_id: String,
        /// A human's signed correction to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<camino::Utf8PathBuf>,
    },

    /// Start a §31 amendment record for a Warrant: writes
    /// `amendments/AM-<n>-<hash>.yaml` with its id minted so two branches
    /// amending the same Warrant never write the same file (t-dc28), and the
    /// fields left for you to fill. `war check` refuses it until they are.
    Amend {
        /// The Warrant to amend. A resolved one is refused by name.
        alias: String,
        /// Print the file name it would write, and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Attestations for ssh-signed acts (OW-ADR-0015): list them, or verify
    /// every signature and subject digest against the tree today.
    Attest {
        /// A Warrant alias or a SAS version; omit with --all.
        target: Option<String>,
        /// Verify signatures and subject digests (default lists only).
        #[arg(long)]
        verify: bool,
        /// Every attestation in the repository (the xtask step).
        #[arg(long)]
        all: bool,
        /// Audit a resolution's evidence custody (§41.5, OW-WAR-0136): each
        /// field per relied-on receipt, present or UNKNOWN, and
        /// `attest.custody-drift` for any subject that moved since signing.
        #[arg(long, conflicts_with_all = ["verify", "all"])]
        custody: bool,
        /// With --custody: record the audit as `custody-audit.toml`, naming
        /// this auditor. Refused to the performer.
        #[arg(long, value_name = "AUDITOR", requires = "custody")]
        record: Option<String>,
    },

    /// A standing authorization (OW-ADR-0029): propose a class, show
    /// classes, apply one to a Warrant. Signing and revoking a class are
    /// `war sign standing:<id>@<rev>` and `… --revoke`.
    Standing {
        #[command(subcommand)]
        command: StandingCommand,
    },

    /// The SAS as a controlled document (§101): propose, accept, diff, status.
    Sas {
        #[command(subcommand)]
        command: SasCommand,
    },
    /// Warrants waiting on a human act (read-only; OW-WAR-0070).
    Inbox {
        /// Only the acts this actor may sign now, assigned ones first
        /// (OW-WAR-0137). Questions stay: answering one is no role's act.
        #[arg(long = "as")]
        actor: Option<String>,
    },
    /// Draft, authenticate and activate authority changes under previous trusted state.
    Authority {
        #[command(subcommand)]
        command: authority_cmd::Command,
    },
}

/// `war evidence …` beyond `record`: proving the work. Also flattened,
/// hidden, into the top level.
#[derive(Subcommand)]
enum EvidenceCommand {
    /// Inspect or run local gate definitions (§44).
    Gate {
        #[command(subcommand)]
        action: Option<GateAction>,
        /// Execute the gates rather than listing them.
        #[arg(long)]
        run: bool,
        /// A single gate id or `<id>@<version>`. Defaults to every gate.
        #[arg(long)]
        gate: Option<String>,
        /// Record the run as evidence under the receipts path (§44.6).
        ///
        /// Off by default. Running a gate to PROBE its behaviour — as the
        /// conformance battery does, by corrupting the definition on purpose —
        /// must not overwrite the evidentiary record for the subject.
        #[arg(long)]
        record: bool,
        /// Subject digests bound into a receipt. Requires --record.
        #[arg(long = "subject-digest")]
        subject_digests: Vec<String>,
        /// Immutable evidence references bound into a receipt. Requires --record.
        #[arg(long = "evidence-ref")]
        evidence_refs: Vec<String>,
    },

    /// §46 independent verification. Emits a request; ingests verdicts.
    ///
    /// This command never verifies anything itself — the actor that produced
    /// the work cannot be the actor that clears it.
    Verify {
        /// The Warrant's local alias.
        alias: String,
        /// The performer whose work is under verification.
        #[arg(long, default_value = "unknown")]
        performer: String,
        /// A verifier's response to ingest. Without it, the request is emitted.
        #[arg(long)]
        response: Option<Utf8PathBuf>,
        /// Write the verification bundle (`oh.war/verification-bundle/v2`):
        /// the request with the authorized digest, every atom, each
        /// deliverable's bytes, the plants naming the alias, gate runs and
        /// prior verifications, under `verifications/bundle-<digest>.json`.
        /// Over `[verify] max_bundle_tokens`, one bundle per obligation,
        /// each carrying what that obligation names, excerpted to fit.
        #[arg(long, conflicts_with = "response")]
        bundle: bool,
        /// Write the bundle(s), run `[verify] verifier_argv` once on each, and
        /// ingest what each prints through the same seam as `--response`.
        #[arg(long, conflicts_with_all = ["response", "bundle"])]
        run: bool,
    },
    /// Take Warrants to their sign-off unattended (t-cee5): deliver, run
    /// each gate-executed stage (`war evidence run`), record each cited gate not
    /// already admissible (`war evidence record`), run the configured
    /// independent verifier (`war evidence verify --run`), record the document gates
    /// last — then print what is left for a human. Idempotent and resumable:
    /// what is current is skipped. Never signs, never writes a disposition,
    /// never asks. See docs/SIGNING.md "One sitting".
    Prepare {
        /// Warrant aliases. Or `--all`.
        #[arg(required_unless_present = "all")]
        aliases: Vec<String>,
        /// Every Warrant not resolved that is authorized or awaits
        /// authorization.
        #[arg(long, conflicts_with = "aliases")]
        all: bool,
        /// Gates that run outside the working tree (a battery in a clone),
        /// and verifier runs, at most this many at once. Gates that run
        /// `war` itself read the projections receipts change, so they run
        /// one at a time with `war admin compile` just before each, whatever this
        /// says.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=64))]
        jobs: u32,
        /// Do not commit the delivery and stage-run records before the
        /// evidence. A receipt over a dirty tree names no source, so without
        /// the commit prepare stops before recording evidence and says what
        /// to commit.
        #[arg(long)]
        no_commit: bool,
        /// Run the verifier even when every obligation already has an
        /// admissible `established` verdict.
        #[arg(long)]
        reverify: bool,
        /// Print each Warrant's plan and run nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// The ops / runs work kind (1.0 plan C4b): run a `service` stage's gate
    /// under its wall time, mint the receipt, write the submission.
    Run {
        /// The Warrant's local alias.
        alias: String,
        /// The stage id, e.g. `STAGE-002`.
        stage: String,
        /// Run a stage of a contract no human has signed. The Dispatch the run
        /// is built from records `prototype://unauthorized`, and so does the
        /// receipt's subject.
        #[arg(long)]
        prototype: bool,
    },
    /// Perform an agent stage with the configured performer (OW-WAR-0069): the
    /// Dispatch goes in on stdin, a Stage Submission comes back on stdout, and
    /// it is ingested through `war evidence submit`'s refusals. It cannot decide the work
    /// is done — §51.2 — and writes nothing if the performer sends nothing.
    Perform {
        /// The Warrant's local alias. Omit with --all.
        alias: Option<String>,
        /// The stage to perform. Omit with --all.
        stage: Option<String>,
        /// Every open agent stage on the frontier, one at a time.
        #[arg(long)]
        all: bool,
        /// Perform a stage of a contract no human has signed. The operator
        /// types this, never the agent: the Dispatch records
        /// `prototype://unauthorized` and the work is not authorized work.
        #[arg(long)]
        prototype: bool,
    },
    /// Ingest a Stage Submission something else produced (§51): it must name a
    /// dispatch this Warrant compiled and may not request its own resolution.
    Submit {
        /// The Warrant's local alias.
        alias: String,
        /// The submission (`oh.war/stage-submission/v1` JSON).
        file: Utf8PathBuf,
    },
    /// KPIs (OW-WAR-0148 M13): run each one, journal every value, and say
    /// its latest, best and target. Ticks nothing.
    Kpi {
        #[command(subcommand)]
        command: KpiCommand,
    },
    /// The assurance mark (OW-ADR-0025, proposed; OW-WAR-0135): evaluate a
    /// resolved Warrant against a versioned baseline, and emit an
    /// `oh.war/mark/v1` statement only when every requirement is met. Unmet
    /// and UNKNOWN requirements are named; a baseline not `accepted` earns no
    /// mark. Derived from records already signed: it grants no authority.
    Mark {
        /// The Warrant's local alias.
        alias: String,
        /// The baseline id (default: `openwarrant.toml [mark] baseline`, else `v1`).
        #[arg(long, value_name = "ID")]
        baseline: Option<String>,
        /// Write the statement to `<warrant>/mark-<baseline>.json`, only when
        /// earned. A cache: `--verify` recomputes it and never trusts it.
        #[arg(long, conflicts_with = "verify")]
        record: bool,
        /// Recompute every binding of the recorded mark against the tree today
        /// and name each one that moved.
        #[arg(long)]
        verify: bool,
        /// With --verify: the mark to check (default: the recorded one).
        #[arg(long, value_name = "FILE", requires = "verify")]
        file: Option<Utf8PathBuf>,
    },
    /// The document work kind (1.0 plan C4a): review every Warrant's Markdown
    /// deliverables — digest, citations, placeholders, independent review.
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// The agent loop, measured (1.0 plan F1): scaffold a throwaway program
    /// per task, draft, dispatch, perform, verify blind, and ask what a
    /// resolution would say. Nothing is authorized, resolved or signed.
    Eval {
        #[command(subcommand)]
        command: EvalCommand,
    },
}

/// `war view …` beyond `timeline`: looking at the work. Also flattened,
/// hidden, into the top level.
#[derive(Subcommand)]
enum ViewCommand {
    /// The web UI: Progress on the canonical roadmap, the queue with each
    /// act's dry-run verdict, questions, frontier, corpus and help, served on
    /// 127.0.0.1 with a per-session token (OW-WAR-0116). A button starts
    /// only `war sign <target> --ssh-sign` or an automatic remedy; your key's
    /// dialog is the signature.
    ///
    /// `--lan <addr:port>` also serves paired devices on the LAN, TLS only
    /// (OW-WAR-0139): a device reads, runs automatic remedies and can ask for
    /// a signature at this machine; it can never sign. `war view ui devices`
    /// lists and revokes them.
    Ui {
        #[command(subcommand)]
        command: Option<UiCommand>,
        /// Local port; 0 picks a free one.
        #[arg(long, default_value_t = 8765)]
        port: u16,
        /// The page to open: tickets, progress, queue, questions, frontier,
        /// corpus, help.
        #[arg(long, default_value = "tickets")]
        page: String,
        /// Who signs, when roles.toml names more than one eligible signer.
        #[arg(long = "as")]
        actor: Option<String>,
        /// Also serve paired devices at this address:port, TLS only. Nothing
        /// is bound beyond loopback unless you type it (0.0.0.0 included).
        #[arg(long, value_name = "ADDR:PORT")]
        lan: Option<String>,
        /// The name devices reach this host by; the Host must be exactly
        /// <name>:<port>. Defaults to the --lan address.
        #[arg(long, requires = "lan")]
        name: Option<String>,
        /// The TLS certificate chain (PEM), e.g. from `tailscale cert`.
        #[arg(long, requires = "lan", value_name = "PEM")]
        cert: Option<Utf8PathBuf>,
        /// The certificate's private key (PEM).
        #[arg(long, requires = "lan", value_name = "PEM")]
        key: Option<Utf8PathBuf>,
        /// Fallback: a self-signed certificate kept in the per-user state
        /// directory. Every device shows a browser warning; compare the
        /// fingerprint the host prints.
        #[arg(long, requires = "lan", conflicts_with_all = ["cert", "key"])]
        self_signed: bool,
        /// Seconds the host's human has to answer a pairing. Hidden; the battery's.
        #[arg(long, hide = true)]
        pair_answer_secs: Option<u64>,
        /// Seconds a pairing code lives. Hidden; the battery's.
        #[arg(long, hide = true)]
        pair_code_secs: Option<u64>,
        /// Seconds a device credential lives. Hidden; the battery's.
        #[arg(long, hide = true)]
        device_secs: Option<u64>,
    },
    /// The app: every pane of the corpus, the queue, help and setup, in the
    /// terminal. `war` with no arguments is the same thing (OW-WAR-0112).
    Tui {
        /// Panic right after the terminal is set up — the fixture that proves
        /// it is restored. Hidden; the battery's.
        #[arg(long, hide = true)]
        panic_after_setup: bool,
    },
    /// Read-only project board, including every stage and exact approval commands.
    Board {
        /// Print a self-contained offline HTML document to stdout.
        #[arg(long, conflicts_with = "json")]
        html: bool,
    },
    /// One screen for everything you owe: the acts awaiting your signature as
    /// a checklist, the questions an agent asked, and the stages it can start
    /// (OW-WAR-0069). Check rows, pick a reason from your presets, sign the
    /// batch. It never signs for you: each row is your own ssh confirmation.
    Console,
    /// Tell the human when an act awaits them: print the pending set, then
    /// poll the record trees and print what appears or is signed away.
    Watch {
        /// Print the pending set once and exit.
        #[arg(long)]
        once: bool,
        /// Poll interval in milliseconds (minimum 50; lower values are raised to it).
        #[arg(long, default_value_t = 500)]
        interval: u64,
        /// Also raise a desktop notification through `notify-send`.
        #[arg(long)]
        notify_send: bool,
        /// Stop after this many polls (for tests).
        #[arg(long, hide = true)]
        ticks: Option<u64>,
    },
    /// List remaining Warrant records and status, read-only, from live sources.
    /// `progress` is an alias. Legacy resolution is not implementation completion.
    #[command(visible_alias = "progress")]
    Overview {
        /// Include records with an existing resolution in text/JSON output.
        #[arg(long)]
        all: bool,
        /// Return the validated viewer snapshot, including attributed work reports.
        #[arg(long, conflicts_with_all = ["html", "serve"])]
        snapshot: bool,
        /// Write a self-contained HTML snapshot (all records), with no implicit server.
        #[arg(long, num_args=0..=1, default_missing_value=".openwarrant/state/progress.html", conflicts_with="serve")]
        html: Option<std::path::PathBuf>,
        /// Serve a read-only, periodically refreshed view on 127.0.0.1.
        #[arg(long)]
        serve: bool,
        /// Local server port; 0 selects an available port.
        #[arg(long, default_value_t = 8765, requires = "serve")]
        port: u16,
        /// Refresh interval, in seconds (1..=3600).
        #[arg(long, default_value_t=5, value_parser=clap::value_parser!(u64).range(1..=3600), requires="serve")]
        refresh_secs: u64,
    },
    /// Every Warrant, its type, state and progress: the light ones (tickets)
    /// first, then those with a directory, then those read in place.
    /// Filters narrow it to exactly the Warrants every one admits.
    #[command(name = "warrants", visible_aliases = ["tickets", "ls"])]
    Tickets {
        /// Only Warrants of this type: a light one's type (bug), or a
        /// profile (delivery, decision, ticket, openspec, speckit).
        #[arg(long = "type", value_name = "TYPE")]
        kind: Option<String>,
        /// Only Warrants carrying this label; repeat to require several.
        #[arg(long = "label", short = 'l', value_name = "LABEL")]
        labels: Vec<String>,
        /// open, in_progress, done, a declared state (in_review) the
        /// Warrant or one of its items holds, or a phase a directory
        /// Warrant's journal records (draft, authorized, resolved).
        #[arg(long, value_name = "STATE")]
        state: Option<String>,
        /// A phrase anywhere in the title, description, notes or items
        /// (case-insensitive).
        #[arg(long, value_name = "PHRASE")]
        text: Option<String>,
        /// Words, each beginning a word somewhere in the title, description,
        /// notes or items, in any order (case-insensitive).
        #[arg(long, value_name = "WORDS")]
        search: Option<String>,
        /// Only the Warrants part of this one (an epic).
        #[arg(long, value_name = "WARRANT")]
        epic: Option<String>,
    },
    /// What can start now: open, unclaimed, unblocked items across
    /// Warrants, most urgent and oldest first. A directory Warrant's ready
    /// stages are in `war next`.
    Ready {
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// What an arriving agent or person reads first: open Warrants with their
    /// remaining items, who holds what, recent notes, done work compacted.
    Prime {
        /// One Warrant in full instead.
        #[arg(value_name = "WARRANT")]
        ticket: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
}

/// `war admin …`: setting up and maintaining. Also flattened, hidden, into
/// the top level.
#[derive(Subcommand)]
enum AdminCommand {
    /// Compile the configured projections (§71.8).
    Compile {
        /// A single Warrant's local alias. Defaults to the whole corpus.
        alias: Option<String>,
    },
    /// Read-only repository diagnostics; never signs, repairs, or starts work.
    Doctor {
        /// Inspect one Warrant instead of the whole corpus.
        alias: Option<String>,
        /// Include deterministic generated-view drift checks.
        #[arg(long)]
        generated: bool,
        /// At a terminal, offer to repair each signing finding. Never signs;
        /// writes roles.toml or allowed_signers only when absent, after you
        /// confirm the exact bytes; prints the lines to add to one that
        /// exists. Refused without a terminal.
        #[arg(long, conflicts_with_all = ["alias", "generated"])]
        fix_signing: bool,
    },
    /// Every file a Warrant's `deliverables.toml` pins, with the Warrant's state.
    ///
    /// Ask BEFORE editing; a resolved Warrant's pin moves only through
    /// `war sign correct` (OW-WAR-0064).
    Pins {
        /// Re-record each DRAFT Warrant's content digests from the bytes on
        /// disk. Refused for anything a human has signed for; `war sign correct`
        /// is the act that moves those.
        #[arg(long)]
        refresh: bool,
        /// One Warrant instead of every draft.
        #[arg(long, value_name = "ALIAS")]
        alias: Option<String>,
        /// Only pins held by a resolution.
        #[arg(long)]
        resolved_only: bool,
        /// Every Warrant that ever governed one path, oldest first, and
        /// whether each delivery still verifies from history (OW-ADR-0021).
        #[arg(long, value_name = "PATH")]
        history: Option<String>,
        /// Whether each resolved Warrant's accepted candidate is still the
        /// one about to merge (OW-WAR-0134): the paths that moved since the
        /// resolution's locator, in scope or not. Defaults to `HEAD`. An
        /// in-scope move is `acceptance.candidate-moved` and exits non-zero;
        /// history that cannot answer is UNKNOWN.
        #[arg(long, value_name = "REV", num_args = 0..=1, default_missing_value = "HEAD", conflicts_with_all = ["refresh", "history"])]
        candidate: Option<String>,
        /// With `--candidate`: set aside Warrants already resolved at this
        /// revision (merged earlier). CI passes the target branch's commit.
        #[arg(long, value_name = "REV", requires = "candidate")]
        base: Option<String>,
    },
    /// Report six readiness dimensions; unavailable checks block readiness.
    Preflight { alias: String },
    /// Semantic difference between the committed compilation and a fresh one (§71.10).
    Diff {
        /// The Warrant's local alias.
        alias: String,
        /// JSON baseline or contract:N retained revision (requires --to). Defaults to committed JSON.
        #[arg(long)]
        from: Option<Utf8PathBuf>,
        /// Explicit JSON target or contract:N retained revision. Defaults to fresh compilation.
        #[arg(long)]
        to: Option<Utf8PathBuf>,
    },
    /// Read a Warrant's local draft journal (§66), or reconstruct one from its
    /// records. There is no way to append by hand: events come from the
    /// commands that change records.
    Journal {
        /// The Warrant's local alias.
        alias: String,
        /// Write the events the existing records imply, each marked
        /// `backfilled`. Refused on a journal that already has events.
        #[arg(long)]
        backfill: bool,
    },

    /// Declare a Warrant's deliverables delivered (t-39dc): record §37.2
    /// provenance on each — the sha256 of the file now, how it was made, the
    /// build of `war` that recorded it — and set `content_addressed`.
    /// Refuses a resolved Warrant (its `deliverables.toml` is bound), a
    /// missing file, and a path a later authorized Warrant governs
    /// (OW-ADR-0021); any refusal writes nothing. Idempotent: a deliverable
    /// already recorded at its bytes is left as it is.
    Deliver {
        /// The Warrant's local alias.
        alias: String,
        /// Deliverable ids (`D-001`). Omit for every deliverable declared.
        ids: Vec<String>,
        /// §37.2 `producer`. Defaults to the recorded one, then
        /// `[project] performer`.
        #[arg(long)]
        producer: Option<String>,
        /// §37.2 `creation_method` (`authored`, `generated`, …). Defaults to
        /// the recorded one, then `authored`.
        #[arg(long)]
        method: Option<String>,
        /// Report what would be recorded and write nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Compile a Stage Dispatch for one stage of a Warrant (§47).
    Dispatch {
        /// The Warrant's local alias.
        alias: String,
        /// The stage id, e.g. `STAGE-003`.
        stage: String,
        /// §52: initial, replay, repair, restart.
        #[arg(long, default_value = "initial")]
        attempt_kind: String,
        /// Prior failure evidence refs. Required for a repair (§52.3).
        #[arg(long = "prior-failure")]
        prior_failure: Vec<String>,
        /// Write the packet here instead of stdout.
        #[arg(long, value_name = "PATH")]
        emit: Option<Utf8PathBuf>,
        /// Write the §33 context manifest (what was selected and what was
        /// omitted, with reasons) beside the packet.
        #[arg(long, value_name = "PATH")]
        emit_context: Option<Utf8PathBuf>,
        /// Compile a packet for a contract no human has signed. The packet
        /// records `prototype://unauthorized` as the authority it acted under,
        /// so nothing downstream can mistake the work for authorized.
        #[arg(long)]
        prototype: bool,
    },
    /// Capture or check portable context for an existing Dispatch; never execute it.
    DispatchBundle {
        #[command(subcommand)]
        command: dispatch_bundle_cmd::Command,
    },
    /// The commit message, drafted from the records that changed (OW-WAR-0069).
    Commit {
        /// Stage everything and commit with the drafted message.
        #[arg(long)]
        write: bool,
    },
    /// Renew the lease on your claims (or the one named), so no other agent
    /// reclaims them while you work. Every war command you run renews them
    /// too; a claim whose lease runs out is taken by a plain `war claim`.
    Heartbeat {
        /// One claimed item or Warrant; omit for every claim you hold.
        target: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Give a claim back without finishing the item.
    Release {
        target: String,
        /// Write only if the item (or Warrant) is still at this revision, the
        /// one `war show --json` gave; refused `warrant.stale-revision`
        /// otherwise, naming the current one.
        #[arg(long = "if-rev", value_name = "DIGEST")]
        if_rev: Option<String>,
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },
    /// Give an unsigned Warrant a free alias: its `local_alias`, its
    /// directory, and a journal line. A Warrant with an authorization keeps
    /// its alias; that is refused by name.
    Renumber {
        /// The Warrant's alias now.
        alias: String,
        /// The alias to give it; refused if any branch this clone knows has it.
        new: String,
    },
    /// Write the AGENTS.md this repository ships, for the repository's
    /// namespace. `war init` writes it once; this rewrites (--force) or prints it.
    /// `--block` keeps only a small managed pointer block in the files you
    /// already have (M16).
    AgentsMd {
        /// Print to stdout instead of writing.
        #[arg(long)]
        stdout: bool,
        /// Overwrite an existing AGENTS.md.
        #[arg(long, conflicts_with = "block")]
        force: bool,
        /// Insert or update the managed pointer block (`<!-- openwarrant:begin -->` …
        /// `<!-- openwarrant:end -->`) in every root AGENTS.md and CLAUDE.md, or in a
        /// new AGENTS.md when neither exists. It says ordinary coding needs no Warrant
        /// and that `war view prime` shows tracked work, and carries the version stamp.
        /// Bytes outside the markers are never touched; a second run changes
        /// nothing. A file with two blocks or an unterminated one is refused by
        /// name, and then no file is written.
        #[arg(long)]
        block: bool,
        /// With --block: write the block into this file instead (repository-relative;
        /// created when absent). Repeatable.
        #[arg(long = "file", value_name = "PATH", requires = "block")]
        files: Vec<String>,
    },

    /// Bring work in as Warrants (OW-WAR-0148 M10): `beads <file.jsonl>`
    /// (Beads' issue JSONL, as `bd export` writes it), `openspec <dir>` (each
    /// change of an OpenSpec folder) or `speckit <dir>` (each feature of a
    /// Spec Kit `specs/` folder). Each becomes a light Warrant (a ticket)
    /// with its tasks as items. Running it again changes nothing; input it
    /// cannot map is refused by name, and a refusal writes nothing. To keep
    /// working in those tools instead, read the folder in place:
    /// `[[adapters]]` in openwarrant.toml (docs/TYPES.md).
    Import {
        /// `beads`, `openspec` or `speckit`.
        format: String,
        /// The Beads JSONL file, or the OpenSpec or Spec Kit folder (or the
        /// folder holding `openspec/` or `specs/`).
        path: Utf8PathBuf,
        /// Who is acting (default: $OPENWARRANT_ACTOR, else `[project] performer`).
        #[arg(long = "as", value_name = "ACTOR")]
        actor: Option<String>,
    },

    /// §68 portable export and round trip; or, as `war admin export beads`, every
    /// light Warrant (a ticket) as Beads issue JSONL on stdout, in the shape
    /// `bd export` writes, which `war admin import beads` reads back (OW-WAR-0148
    /// M10).
    Export {
        /// The Warrant's local alias (§68), or `beads`. Not needed with
        /// --progress.
        #[arg(required_unless_present_any = ["progress", "verify_progress"])]
        alias: Option<String>,
        /// Write the progress bundle (`oh.war/progress-bundle/v1`) — the corpus
        /// projections and every compiled WAR.json with a sha256 manifest — into
        /// this empty directory instead of exporting one Warrant.
        #[arg(long, value_name = "DIR", conflicts_with_all = ["force", "round_trip", "reconnect"])]
        progress: Option<Utf8PathBuf>,
        /// Verify a progress bundle directory against its MANIFEST.json.
        #[arg(long, value_name = "DIR", conflicts_with_all = ["alias", "progress", "force", "round_trip", "reconnect"])]
        verify_progress: Option<Utf8PathBuf>,
        /// Write the package here even if §68.2 contents are missing. Never
        /// reports the result as valid.
        #[arg(long)]
        force: bool,
        /// §68.3 — export, re-export, compare. Without --reconnect the
        /// comparison is refused as vacuous.
        #[arg(long)]
        round_trip: bool,
        /// Record that the preserved evidence bytes were reconnected.
        #[arg(long)]
        reconnect: bool,
    },
    /// Import a legacy ADR corpus (§96), discharging OW-WAR-0043.
    Migrate {
        /// Directory of ADR files named `NNNN-*.md`.
        #[arg(long)]
        corpus: Utf8PathBuf,
        /// The one named, frozen commit the corpus is read at (OBL-001).
        #[arg(long)]
        commit: String,
        /// Where to write the import artifact.
        #[arg(long, default_value = "artifacts/lamquant-adr-import.json")]
        out: Utf8PathBuf,
        /// Compare against the existing artifact instead of writing it — OBL-001's
        /// "a re-run at that SHA producing byte-identical output".
        #[arg(long)]
        verify: bool,
        /// NEGATIVE CONTROL. Attempt to promote each completion line to a
        /// resolution, so §96.3's refusal is observable from outside the binary.
        /// It must always fail; a build where this succeeds is the defect.
        #[arg(long)]
        attempt_promotion: bool,
    },

    /// Experimental preservation transport. Imported records grant no authority.
    Archive {
        #[command(subcommand)]
        cmd: preservation::Command,
    },
    /// Feed a harness's own task list into the tick ladder (OW-WAR-0148
    /// M13). Proposes ticks; writes only with `--apply`; never blocks.
    Bridge {
        #[command(subcommand)]
        command: BridgeCommand,
    },
    /// Host the WAR domain for Liminal, `oh.war/liminal-v1` (OW-WAR-0148 M8;
    /// SAS §82.2; docs/LIMINAL_HOST.md): one request on stdin, one response
    /// on stdout. Pure — no repository, no file, no network, no process.
    /// Exit 0 compiled, 1 refused, 2 compiled with something not
    /// established. `--export` writes the request that reproduces this
    /// repository's model instead.
    Host {
        /// Write this repository's request (honours `--root`).
        #[arg(long)]
        export: bool,
        /// With `--export`: a projection to request, `KIND:SUBJECT`
        /// (`warrant:OW-WAR-0001`, `warrant:*`). Repeatable.
        #[arg(long = "projection", value_name = "KIND:SUBJECT", requires = "export")]
        projections: Vec<String>,
    },
    /// Run an offline SDK operation from an explicit JSON request. Always emits JSON.
    Sdk {
        /// Request JSON file, or - for stdin.
        #[arg(long)]
        request: String,
        /// Also save the result envelope to a new file; existing files are never replaced.
        #[arg(long)]
        output: Option<Utf8PathBuf>,
    },
    /// Serve the Model Context Protocol over stdio (OW-ADR-0014): every
    /// read, request half and agent-permitted write as a tool; no signing,
    /// no ingest. `--describe` prints the tool table instead of serving.
    Mcp {
        /// Print the tools, the refusal list and the resources, then exit.
        #[arg(long)]
        describe: bool,
    },

    /// §67 Knowledge Fabric seam. `health` reads; `act` WRITES and needs
    /// --confirm-write.
    Kf {
        #[command(subcommand)]
        cmd: KfCommand,
    },

    /// §94 telemetry baseline, §95 untracked-work candidates, §100 metrics.
    Telemetry {
        /// The commit this baseline is taken at (§94, OBL-001).
        #[arg(long)]
        commit: String,
        /// Where to write the baseline artifact.
        #[arg(long, default_value = "artifacts/telemetry-baseline.json")]
        out: camino::Utf8PathBuf,
        /// Compare against the existing artifact instead of writing it.
        #[arg(long)]
        verify: bool,
        /// §95 — relate an untracked-work candidate to a Warrant. Requires
        /// --reviewer; an empty one is refused as a fabrication.
        #[arg(long, value_name = "SCOPE")]
        attach: Option<String>,
        /// The Warrant the candidate belongs to.
        #[arg(long, value_name = "ALIAS", default_value = "")]
        warrant: String,
        /// Who reviewed the attribution. §95 will not accept an empty one —
        /// pass `--reviewer <name>`. It is not defaulted to the acting user on
        /// purpose: a review nobody performed is the fabrication §95 forbids.
        #[arg(long, default_value = "")]
        reviewer: String,
    },
    /// Bind a Warrant's machine scope to Bonsai evidence.
    Bonsai {
        #[command(subcommand)]
        command: BonsaiCommand,
    },
    /// Lower a computational Warrant's stage graph into a BLUT PlanSpec (§49).
    Blut {
        /// The Warrant's local alias.
        alias: String,
        /// Path to a real BLUT binary. When given, the lowered PlanSpec is
        /// handed to `<binary> plan check --json` and BLUT's own verdict is
        /// reported. Without it the lowering is only structurally faithful to
        /// a schema, which is a weaker claim.
        #[arg(long, value_name = "BLUT_BINARY")]
        verify: Option<camino::Utf8PathBuf>,
        /// Write the lowered PlanSpec here, so it can be handed to
        /// `blut plan run`. Without this the spec exists only inside the
        /// report, and running it means copying JSON out of prose.
        #[arg(long, value_name = "PATH")]
        emit: Option<camino::Utf8PathBuf>,
    },
    /// The repositories this user works in, remembered on use (OW-WAR-0115):
    /// the list `war` opens from anywhere. Each row is read from that
    /// project's own records; a path that no longer holds one is reported
    /// missing, never dropped silently. `OPENWARRANT_NO_PROJECTS=1` turns
    /// remembering off.
    Projects {
        /// Remember a repository without running a command in it.
        #[arg(long, conflicts_with = "forget")]
        add: Option<Utf8PathBuf>,
        /// Take one off the list (the repository itself is untouched).
        #[arg(long)]
        forget: Option<Utf8PathBuf>,
    },
    /// Install a published release into ~/.local/lib/openwarrant and repoint
    /// the links this tool put on PATH. Reads the network; never touches a
    /// `war` it did not install.
    Update {
        /// Report what is published and stop.
        #[arg(long)]
        check: bool,
        /// Offer prereleases (1.0.0-alpha.2 and the like). The default follows
        /// this build: a prerelease build is already on the preview channel.
        #[arg(long)]
        preview: bool,
        /// A specific version, e.g. `1.0.0-alpha.2`.
        #[arg(long, value_name = "VERSION")]
        to: Option<String>,
        /// Install even when the version is the one already running.
        #[arg(long)]
        force: bool,
    },
    /// Which `war` this is, where it came from, and what else answers to that
    /// name on PATH. Needs no repository.
    Version {
        /// Classify the build a directory would produce, as JSON: the facts
        /// `build.rs` reads, then its verdict. For the conformance plants.
        #[arg(long, hide = true, value_name = "DIR")]
        probe: Option<Utf8PathBuf>,
    },
    /// The JSON Schema pack (OW-WAR-0032): write `schemas/` from the record
    /// types, or `--check` the tree against them. Built with `--features schema`.
    #[cfg(feature = "schema")]
    Schemas {
        /// Compare instead of writing; drift is reported by file.
        #[arg(long)]
        check: bool,
    },
    /// git's merge driver for ticket files: checklists item by item, notes
    /// appended (`merge=war-ticket` in .gitattributes). `--install` writes
    /// the .gitattributes lines and this clone's git configuration.
    MergeTicket {
        /// The common ancestor's copy (git's %O).
        #[arg(required_unless_present_any = ["install", "probe"])]
        base: Option<Utf8PathBuf>,
        /// Ours (git's %A); the result is written here.
        #[arg(required_unless_present_any = ["install", "probe"])]
        ours: Option<Utf8PathBuf>,
        /// Theirs (git's %B).
        #[arg(required_unless_present_any = ["install", "probe"])]
        theirs: Option<Utf8PathBuf>,
        /// The path in the repository (git's %P).
        path: Option<String>,
        /// Configure this clone instead of merging.
        #[arg(long)]
        install: bool,
        /// Exit 0: this war has the driver (what the configured command
        /// asks before it runs it).
        #[arg(long)]
        probe: bool,
    },
}

/// `war evidence …`: `record`, the one member that never stood alone, and
/// the rest.
#[derive(Subcommand)]
enum EvidenceGroup {
    /// Run the gates the Warrant's assurance atom cites and mint each §44.6
    /// receipt into `docs/warrants/<alias>/gate-runs/`, bound to the Warrant's
    /// current contract digest. Refuses a Warrant that cites no gate, and a
    /// named gate the Warrant does not cite.
    Record {
        /// The Warrant's local alias.
        alias: String,
        /// One cited gate (`<id>@<version>`). Defaults to every cited gate.
        #[arg(long)]
        gate: Option<String>,
        /// The Bonsai evidence document the Bonsai gate's receipt binds,
        /// `file:<path>#sha256:<digest>` (t-dec1). Required to record
        /// `software.repo.bonsai-evidence`; refused for any other gate.
        #[arg(long = "evidence-ref")]
        evidence_ref: Option<String>,
    },
    #[command(flatten)]
    Member(EvidenceCommand),
}

/// `war view …`: the members, and `timeline`, new with the group.
// Parsed once per process; boxing would only obscure the clap derive.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum ViewGroup {
    #[command(flatten)]
    Member(ViewCommand),
    /// The corpus timeline (`oh.war/corpus-timeline/v1`): every journal
    /// event by day. The same as `war status --timeline`, and its envelope's
    /// command is `status`, as that one's is.
    Timeline,
}

/// `war sign …`: the members, and the two acts new with OW-WAR-0148 M14.
/// Neither is a top-level spelling: `war release` was always the claim's
/// release (`war admin release`), so only the group's spelling exists.
#[derive(Subcommand)]
enum SignGroup {
    #[command(flatten)]
    Member(SignCommand),
    /// Approve a Warrant as an official plan (docs/PRESETS.md).
    ///
    /// Without `--ssh-sign`, the request: what an approval binds, who may
    /// sign it, recorded in the Warrant's journal so `war next` and `war
    /// sign inbox` list it, and `[notify]` run. Nothing is signed and no key
    /// is asked. With `--ssh-sign`, a person's approval, signed with their
    /// key and recorded only when the signature verifies as theirs, by a
    /// roster principal whose role in `[roles.roster]` allows the Warrant's
    /// kind.
    Approve {
        /// A light Warrant's id (`t-...`).
        #[arg(value_name = "WARRANT")]
        id: String,
        /// Sign as this person (required when more than one may).
        #[arg(long = "as", value_name = "HUMAN")]
        actor: Option<String>,
        /// Your own words, appended to the drafted meaning.
        #[arg(long)]
        meaning: Option<String>,
        /// Sign with your ssh key (`docs/authority/allowed_signers`).
        #[arg(long)]
        ssh_sign: bool,
        /// Draft the approval and say whether it would be recorded; writes
        /// nothing, touches no key.
        #[arg(long, conflicts_with = "ssh_sign")]
        dry_run: bool,
    },
    /// Sign what the preset asks for before a release, in one batch.
    ///
    /// Lists every act awaiting a signature (`war sign --list`), drafts the
    /// ones one signature can carry, and says which sign alone or in a
    /// second batch. Without `--ssh-sign` it is the request: nothing is
    /// written, no key is asked, and `[notify]` runs. With `--ssh-sign` it is
    /// `war sign --batch`, each response's meaning naming the release. Under
    /// the vibe preset nothing is asked for.
    Release {
        /// The release, as you tag it (`v1.2.0`).
        tag: String,
        /// Sign as this person (required when more than one may).
        #[arg(long = "as", value_name = "HUMAN")]
        actor: Option<String>,
        /// Sign the batch with your ssh key.
        #[arg(long)]
        ssh_sign: bool,
        /// Judge the batch as the signature would, and stop; writes nothing.
        #[arg(long, conflicts_with = "ssh_sign")]
        dry_run: bool,
    },
}

/// `war admin …`: the members, and `preset`, new with OW-WAR-0148 M14 and
/// only under the group.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum AdminGroup {
    #[command(flatten)]
    Member(AdminCommand),
    /// Show the repo preset, or switch it (docs/PRESETS.md).
    ///
    /// `vibe`, `team` or `regulated` rewrites `[preset]` in openwarrant.toml
    /// with the preset's defaults, and writes its `[roles]` when the file
    /// has none; `none` removes `[preset]`. Every other byte of the file is
    /// kept. Alone, it shows the preset in force and what follows from it.
    Preset {
        /// vibe, team, regulated, or none.
        name: Option<String>,
        /// Also write the preset's [roles] over the table in the file
        /// ([roles.roster] is kept).
        #[arg(long, requires = "name")]
        reset_roles: bool,
    },
}

/// `war plan`: a drafting request alone, or one of the group's members.
#[derive(clap::Args)]
struct PlanArgs {
    #[command(subcommand)]
    command: Option<PlanCommand>,
    /// What the Warrant should accomplish. Ignored with --proposal.
    #[arg(default_value = "")]
    request: String,
    #[arg(long, default_value = "delivery")]
    profile: String,
    #[arg(long, default_value = "basic")]
    assurance: String,
    /// A Draft Proposal returned by an agent (v2), to validate against §74.4.
    #[arg(long)]
    proposal: Option<Utf8PathBuf>,
    /// Record that §74.4 steps 5 and 6 (semantic diff, review) happened.
    #[arg(long)]
    reviewed: bool,
    /// Hand the request to the drafter configured in `[plan] drafter_argv`
    /// and read its proposal (§75.2). The process must not touch the tree.
    #[arg(long)]
    draft: bool,
    /// Where `--draft` writes the proposal. Defaults to a scratch file.
    #[arg(long)]
    out: Option<Utf8PathBuf>,
    /// An interview answer, `<question-id>=<text>` (§74.6). Repeatable.
    #[arg(long = "answer")]
    answers: Vec<String>,
    /// Apply a reviewed proposal: create the Warrant and its atoms through
    /// the seven §74.3 operations. Never a raw file write from the model.
    #[arg(long)]
    apply: bool,
    /// Draft from a GitHub issue, fetched by `[intake] fetch_argv` with
    /// `{id}` substituted (OW-WAR-0141). Refused, starting nothing, when
    /// no `[intake]` table is configured. Never writes to the tracker.
    #[arg(long, value_name = "ID", conflicts_with = "issue_file")]
    issue: Option<String>,
    /// Draft from an issue file: `gh issue view <n> --json
    /// number,title,body,url` output. The Warrant records where it came
    /// from in `plan/intake.json`.
    #[arg(long, value_name = "PATH")]
    issue_file: Option<Utf8PathBuf>,
    /// Draft typed records (`oh.war/records-request/v1`) and the ticket
    /// whose items implement them, in place of a Warrant (OW-WAR-0148
    /// M7). Validated by the rules authored records get before anything
    /// is written; `--reviewed --apply` writes one record atom under
    /// `docs/records/<area>/` and the ticket.
    #[arg(long, conflicts_with_all = ["issue", "issue_file"])]
    records: bool,
    /// With --records: the area under `docs/records/` the records land in.
    #[arg(long, value_name = "NAME", requires = "records")]
    area: Option<String>,
}

/// `war sign`: a signature alone, or one of the group's members.
#[derive(clap::Args)]
struct SignArgs {
    #[command(subcommand)]
    command: Option<SignGroup>,
    /// A Warrant alias, a SAS version, `<alias>/<D-id>` (a correction),
    /// `roadmap`, or `<gate_id>@<version>` (a gate invalidation, with
    /// --grounds). Omit with --list or --all.
    target: Option<String>,
    /// Show what awaits a signature and exit. Needs no terminal.
    #[arg(long)]
    list: bool,
    /// Sign every pending act, one prompt each.
    #[arg(long)]
    all: bool,
    /// Sign as this actor (required when more than one is eligible).
    #[arg(long = "as")]
    actor: Option<String>,
    /// Your own words, appended to the drafted meaning.
    #[arg(long)]
    meaning: Option<String>,
    /// For a gate invalidation (`<gate_id>@<version>`): why the definition
    /// is invalid (§56.4). Your words, signed as written; every dispute
    /// repeats them.
    #[arg(long, conflicts_with = "meaning")]
    grounds: Option<String>,
    /// Resolution outcome when §38.6 forbids `satisfied`:
    /// not_satisfied, cancelled, blocked.
    #[arg(long)]
    outcome: Option<String>,
    /// §101.3 ADR reference for an architecture-changing SAS revision.
    #[arg(long)]
    adr: Option<String>,
    /// none | separate_role | organizational (§27.4).
    #[arg(long, default_value = "separate_role")]
    independence: String,
    /// Open $VISUAL/$EDITOR on the drafted response before the prompt.
    #[arg(long)]
    edit: bool,
    /// Render what would be signed and stop. No terminal needed; writes
    /// nothing. Read this from anywhere; sign it from a terminal.
    #[arg(long)]
    show: bool,
    /// Draft the response and run the act's ingest with the write
    /// withheld: every refusal the real signature would meet, by its rule
    /// name, and `<act>.would-record` when none. Writes nothing, touches
    /// no key. For an agent to troubleshoot before asking a human.
    #[arg(long)]
    dry_run: bool,
    /// Sign with your ssh key instead of a terminal prompt. Needs
    /// `ssh_principal` on your roles.toml entry and a matching line in
    /// docs/authority/allowed_signers. Load the key with `ssh-add -c` so
    /// every signature asks YOU through a dialog; `war` cannot check that.
    #[arg(long)]
    ssh_sign: bool,
    /// Verify a recorded response's .sig sidecar and stop. Writes nothing.
    #[arg(long)]
    verify: bool,
    /// For a correction (`<alias>/<D-id>`): behaviour-change | added-refusal.
    #[arg(long)]
    kind: Option<String>,
    /// One signature over many acts (OW-WAR-0072). Alone: every pending
    /// act that can be batched. With a comma-separated list: just those.
    /// Needs --ssh-sign; one dialog signs the list (docs/SIGNING.md).
    #[arg(long, num_args = 0..=1, value_delimiter = ',', default_missing_value = "")]
    batch: Option<Vec<String>>,
    /// Undo an interrupted batch (OW-WAR-0130): put every path its marker
    /// lists back to its prior bytes. Signs nothing, needs no key. The
    /// same act as the target `recover:<id>`, which stays as an alias.
    #[arg(long, value_name = "BATCH_ID")]
    recover: Option<String>,
    /// Revoke a signed standing authorization (`standing:<id>@<rev>`).
    /// A human act like its acceptance; records already made under the
    /// class stand (§31). Never part of `--all`.
    #[arg(long)]
    revoke: bool,
}

#[derive(Subcommand)]
enum Command {
    // The daily verbs: what `war --help` lists.
    #[command(flatten)]
    Daily(DailyCommand),
    // The five groups: named under "More:" in `war --help`. The first
    // line of each doc comment is the group's purpose there.
    /// Shape the work: drafts, records, projections, roadmap, questions.
    ///
    /// Alone, `war plan "<sentence>"` builds a drafting request for an agent
    /// (§71.3, §75.2). It emits the canonical request and stops: this build
    /// ships no agent, and a seam with nothing on the other side should say
    /// so rather than pretend. `--proposal`, `--reviewed` and `--apply` take
    /// the proposal back. A one-word request that is also a member's name
    /// (`model`) runs the member; `war plan -- model` drafts it.
    #[command(args_conflicts_with_subcommands = true)]
    Plan(PlanArgs),
    /// The human acts: sign, authorize, resolve, correct, amend, attest.
    ///
    /// Alone, `war sign <target>` signs what is waiting — authorize, resolve,
    /// or accept a SAS revision — from one screen at a terminal. It refuses
    /// without a TTY (§27.2: an agent's shell has none), drafts the response
    /// from the record's own facts, shows what is being signed, asks once,
    /// and on `y` runs the same ingest a hand-written response would. There
    /// is no `--yes`. A target is never one of the members' names, so `war
    /// sign authorize <alias>` is the member and `war sign <alias>` the
    /// signature. Nothing here signs for anyone: every signature is a
    /// person's own act at their own terminal or key.
    #[command(args_conflicts_with_subcommands = true)]
    Sign(SignArgs),
    /// Prove the work: gates, verification, stage runs, KPIs, prepare.
    ///
    /// `war evidence record <alias>` mints receipts for a Warrant's cited
    /// gates; the members run gates, tests and stages, and ask for an
    /// independent verdict. None of them decides the work is done.
    Evidence {
        #[command(subcommand)]
        command: EvidenceGroup,
    },
    /// Look at the work: web UI, terminal app, board, lists, timeline.
    ///
    /// Read-only screens and lists over the same records the daily verbs
    /// write.
    View {
        #[command(subcommand)]
        command: ViewGroup,
    },
    /// Set up and maintain: compile, doctor, pins, import, export, hooks.
    ///
    /// Setup, projections, diagnostics, interchange with other tools, and
    /// the integrations (MCP, SDK, Liminal host, git's merge driver).
    Admin {
        #[command(subcommand)]
        command: AdminGroup,
    },
    // Every earlier spelling: each group's members once more at the top
    // level, hidden by `command()` so help never lists them.
    #[command(flatten)]
    PlanMember(PlanCommand),
    #[command(flatten)]
    SignMember(SignCommand),
    #[command(flatten)]
    EvidenceMember(EvidenceCommand),
    #[command(flatten)]
    ViewMember(ViewCommand),
    #[command(flatten)]
    AdminMember(AdminCommand),
    /// Read the releases list once and record the newest in the notice's
    /// cache. Started detached by the notice; prints nothing.
    #[command(name = "__release-check", hide = true)]
    ReleaseCheck,
}

pub fn entrypoint() -> ExitCode {
    use clap::FromArgMatches;
    let parsed = command()
        .try_get_matches()
        .and_then(|matches| Ok((Cli::from_arg_matches(&matches)?, matches)));
    let (cli, matches) = match parsed {
        Ok(parsed) => parsed,
        Err(error) => {
            // Preserve legacy argument handling. SDK callers always receive a
            // report for invocation errors; --help and --version remain help.
            //
            // The global flags have to be stepped over to find the subcommand.
            // `--root` also takes a VALUE, so the token after a bare `--root`
            // is skipped too; without that, `war --root /x sdk` finds `/x`,
            // decides this is not an SDK call, and hands an SDK caller clap's
            // help instead of the envelope it parses.
            //
            // M12: `war admin sdk` is the same command, so its first word may
            // be the group's.
            let mut args = std::env::args_os().skip(1);
            let mut words = Vec::new();
            while words.len() < 2 {
                let Some(arg) = args.next() else { break };
                if arg == "--json" {
                    continue;
                }
                if arg == "--root" {
                    let _ = args.next();
                    continue;
                }
                if arg.to_string_lossy().starts_with("--root=") {
                    continue;
                }
                let first_is_admin = words.is_empty() && arg == "admin";
                words.push(arg);
                if !first_is_admin {
                    break;
                }
            }
            let sdk = words.last().is_some_and(|w| w == "sdk")
                && (words.len() == 1 || words[0] == "admin");
            if sdk && error.use_stderr() {
                return ExitCode::from(sdk::argument_error(&error.to_string()));
            }
            error.exit();
        }
    };
    let mode = output::Mode::from_flag(cli.json);
    let json = cli.json;
    let code = match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(report) => {
            // §76.2: an explicit diagnostic naming what was wrong and where,
            // never a bare "error". Under --json, an envelope on stdout.
            if let Some(repo::RepoError::ObservationUnavailable { rule, message }) =
                report.downcast_ref::<repo::RepoError>()
            {
                output::unavailable(mode, rule, message);
                return ExitCode::from(EXIT_NOT_READY);
            }
            let message = report.to_string();
            output::error(mode, &message);
            // OW-WAR-0130: a `war` the repository does not admit is a refusal
            // to proceed, not a malfunction: not ready, exit 2.
            if message.contains(repo::compat::TOO_OLD) {
                return ExitCode::from(EXIT_NOT_READY);
            }
            ExitCode::from(EXIT_DIAGNOSTIC)
        }
    };
    // OW-WAR-0143: after everything the command printed, at most one stderr
    // line when a newer release is published. It never waits on the network.
    // M12: a group's member is named by its own word (`war admin update` is
    // `update`), as the command's earlier spelling was.
    let leaf = match matches.subcommand() {
        Some((group, sub)) if GROUPS.contains(&group) => sub.subcommand_name().or(Some(group)),
        other => other.map(|(name, _)| name),
    };
    notice::after(leaf, json);
    code
}

/// OW-WAR-0148 M14: under a preset (or a `[roles]` table), `war show` and
/// `war status <id>` of a light Warrant end with whether it is official, and
/// why. Without one the answer is the bytes it always was.
fn with_official(
    repository: &repo::Repository,
    id: &str,
    mut outcome: ticket::Outcome,
) -> ticket::Outcome {
    if outcome.is_refused() {
        return outcome;
    }
    if let Some((md, value)) = official_section(repository, id) {
        if !outcome.human.ends_with('\n') {
            outcome.human.push('\n');
        }
        outcome.human.push_str(md.trim_end());
        if let Some(obj) = outcome.result.as_object_mut() {
            obj.insert("official".to_owned(), value);
        }
    }
    outcome
}

/// The `## Official` section and its JSON for one Warrant, when a preset
/// (or `[roles]`) is configured; `None` otherwise.
fn official_section(
    repository: &repo::Repository,
    id: &str,
) -> Option<(String, serde_json::Value)> {
    let policy = preset::Policy::read_or_default(&repository.root);
    if !policy.configured() {
        return None;
    }
    let subject = official::subject(repository, id).ok()?;
    let standing = official::local(repository, &policy, subject);
    Some((
        standing.section(&policy),
        serde_json::to_value(&standing).unwrap_or_default(),
    ))
}

/// Print a ticket command's answer and return its exit code: the rendering
/// (or the refusal, on stderr) for a person, the envelope under `--json`.
fn ticket_answer(mode: output::Mode, command: &str, outcome: &ticket::Outcome) -> u8 {
    match mode {
        output::Mode::Human => {
            for d in &outcome.report.diagnostics {
                match d.severity {
                    diagnostic::Severity::Error => eprintln!("refused ({}): {}", d.rule, d.message),
                    diagnostic::Severity::Warn => eprintln!("warning ({}): {}", d.rule, d.message),
                    // OW-WAR-0148 M5: an outcome nobody can establish (an
                    // issue write that failed) is said, never swallowed.
                    diagnostic::Severity::Unknown => {
                        eprintln!("UNKNOWN ({}): {}", d.rule, d.message);
                    }
                    _ => {}
                }
            }
            if !outcome.is_refused() {
                println!("{}", outcome.human);
            }
        }
        output::Mode::Json => println!(
            "{}",
            output::envelope(command, &outcome.report, Some(outcome.result.clone()))
        ),
    }
    output::exit_code(&outcome.report)
}

/// What every command needs from the invocation: how to print, and which
/// repository. The repository stays LAZY: `init`, `sdk`, `install` and
/// `schemas` must work where no `openwarrant.toml` exists, and `doctor`
/// reports a discovery failure itself, inside a valid envelope, rather than
/// aborting on it.
struct Ctx {
    mode: output::Mode,
    json: bool,
    root: Option<Utf8PathBuf>,
}

impl Ctx {
    /// Every command that opened a repository remembers it for the hub
    /// (OW-WAR-0115): best-effort, and nothing it does changes the result.
    ///
    /// M11: every command that opens a repository renews the acting agent's
    /// claim leases (`$OPENWARRANT_ACTOR`, else `[project] performer`): a
    /// touch per lock it holds, nothing more. The ticket commands renew for
    /// their own `--as`.
    fn open_repo(&self) -> Result<repo::Repository, repo::RepoError> {
        let r = repo::Repository::discover(self.root.clone());
        if let Ok(r) = &r {
            projects::touch(&r.root);
            ticket::renew_ambient(r);
        }
        r
    }

    /// The ticket loop: a store over the ticket files. Every ticket command
    /// renews the acting agent's claims (M11).
    fn tickets(
        &self,
        actor: Option<&str>,
    ) -> Result<(repo::Repository, ticket::Store), Box<dyn std::error::Error>> {
        let repository = repo::Repository::discover(self.root.clone())?;
        projects::touch(&repository.root);
        let store = ticket::Store::open(&repository, actor)?;
        store.renew_all();
        Ok((repository, store))
    }
}

/// The clap tree as `war` parses it: [`Cli`]'s, with every top-level
/// subcommand outside [`DAILY`] hidden and the help that lists the daily
/// verbs, then the groups under "More:" (M12). Hidden is absent from help
/// and nothing else: every spelling still parses, with no warning.
#[must_use]
pub fn command() -> clap::Command {
    use clap::CommandFactory as _;
    let cmd = Cli::command().mut_subcommands(|s| {
        let daily = DAILY.contains(&s.get_name());
        s.hide(!daily)
    });
    let width = GROUPS.iter().map(|g| g.len()).max().unwrap_or(0);
    let more: String = GROUPS
        .iter()
        .map(|g| {
            let about = cmd
                .find_subcommand(g)
                .and_then(clap::Command::get_about)
                .map(ToString::to_string)
                .unwrap_or_default();
            format!("  {g:<width$}  {}\n", about.trim_end_matches('.'))
        })
        .collect();
    cmd.help_template(format!(
        "{{before-help}}{{about-with-newline}}\n{{usage-heading}} {{usage}}\n\n\
         Commands:\n{{subcommands}}\n\nMore:\n{more}\nOptions:\n{{options}}{{after-help}}"
    ))
}

pub fn run(cli: Cli) -> Result<u8, Box<dyn std::error::Error>> {
    let ctx = Ctx {
        mode: output::Mode::from_flag(cli.json),
        json: cli.json,
        root: cli.root,
    };
    let Some(command) = cli.command else {
        // `war` alone. `--json` has no envelope to give — a terminal
        // application is not a projection — so it names the commands that do.
        // Refused by name (`tui.json`), exit 2, like every other refusal.
        if ctx.json {
            return Ok(tui::refuse_json());
        }
        let code = tui::run(ctx.root, false)?;
        // No terminal: the refusal above, then clap's usage — `war` alone in a
        // script is a caller that wanted a subcommand.
        if code == EXIT_NOT_READY && !sign::at_a_terminal() {
            eprintln!("\n{}", command().render_usage());
        }
        return Ok(code);
    };
    match command {
        Command::Daily(c) => run_daily(&ctx, c),
        Command::Plan(args) => run_plan(&ctx, args),
        Command::PlanMember(c) => run_plan_member(&ctx, c),
        Command::Sign(args) => run_sign(&ctx, args),
        Command::SignMember(c) => run_sign_member(&ctx, c),
        Command::Evidence { command } => run_evidence(&ctx, command),
        Command::EvidenceMember(c) => run_evidence_member(&ctx, c),
        Command::View { command } => run_view(&ctx, command),
        Command::ViewMember(c) => run_view_member(&ctx, c),
        Command::Admin {
            command: AdminGroup::Member(command),
        }
        | Command::AdminMember(command) => run_admin(&ctx, command),
        Command::Admin {
            command: AdminGroup::Preset { name, reset_roles },
        } => run_preset(&ctx, name.as_deref(), reset_roles),
        Command::ReleaseCheck => {
            notice::refresh();
            Ok(EXIT_OK)
        }
    }
}

fn run_daily(ctx: &Ctx, command: DailyCommand) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        DailyCommand::Create {
            title,
            items,
            body,
            priority,
            draft,
            records,
            area,
            kind,
            labels,
            part_of,
            issue,
            issue_file,
            implements,
            actor,
        } => {
            let (repository, store) = ctx.tickets(actor.as_deref())?;
            // OW-WAR-0148 M5: the issue is read once, before anything is
            // written; a refusal of the read is the command's refusal.
            let issue = match (issue, issue_file) {
                (Some(n), _) => {
                    if let Some(refusal) = ticket::issue_already_linked(&store, &n)? {
                        return Ok(ticket_answer(mode, "create", &refusal));
                    }
                    match plan::intake::fetch(&repository, &n) {
                        Ok(i) => Some(i),
                        Err(e) => {
                            return Ok(ticket_answer(mode, "create", &ticket::refusal_of(e)));
                        }
                    }
                }
                (None, Some(path)) => match plan::intake::read_file(&path) {
                    Ok(i) => Some(i),
                    Err(e) => return Ok(ticket_answer(mode, "create", &ticket::refusal_of(e))),
                },
                (None, None) => None,
            };
            let title = title
                .or_else(|| issue.as_ref().map(|i| i.title.clone()))
                .unwrap_or_default();
            let body = match (issue.as_ref().map(|i| i.body.trim().to_owned()), body) {
                (Some(b), Some(extra)) if !b.is_empty() => Some(format!("{b}\n\n{extra}")),
                (Some(b), None) if !b.is_empty() => Some(b),
                (_, extra) => extra,
            };
            if records {
                let outcome = plan::records::run_create(
                    &repository,
                    &store,
                    &title,
                    area.as_deref(),
                    &items,
                    body.as_deref(),
                    priority,
                )?;
                return Ok(ticket_answer(mode, "create", &outcome));
            }
            let mut items = items;
            match ticket::implementing_items(&repository, &implements)? {
                Ok(more) => items.extend(more),
                Err(refusal) => return Ok(ticket_answer(mode, "create", &refusal)),
            }
            if draft {
                match ticket::drafted_items(&repository, &title)? {
                    Ok(drafted) => items.extend(drafted),
                    Err(refusal) => return Ok(ticket_answer(mode, "create", &refusal)),
                }
            }
            let args = ticket::CreateArgs {
                title,
                items,
                body,
                priority,
                kind,
                labels,
                part_of,
                issue,
            };
            Ok(ticket_answer(
                mode,
                "create",
                &ticket::create(&store, &args)?,
            ))
        }
        DailyCommand::Claim {
            target,
            steal,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "claim",
                &ticket::claim_cmd(&store, &target, steal)?,
            ))
        }
        DailyCommand::Done {
            target,
            note,
            if_rev,
            check,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "done",
                &ticket::done_with(&store, &target, note.as_deref(), if_rev.as_deref(), check)?,
            ))
        }
        DailyCommand::Add {
            ticket: target,
            text,
            after,
            tests,
            name,
            kpi,
            cmd,
            direction,
            target: kpi_target,
            mode: kpi_mode,
            milestone,
            min,
            if_rev,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            let parts = ticket::parts::PartsArgs {
                tests,
                name,
                kpi: kpi.map(|name| ticket::parts::KpiSpec {
                    name,
                    cmd,
                    direction,
                    target: kpi_target,
                    mode: kpi_mode,
                }),
                milestone,
                min,
            };
            // Without a part, `war add` is the item it always was.
            let outcome = if parts.is_empty() {
                ticket::add(
                    &store,
                    &target,
                    text.as_deref().unwrap_or_default(),
                    &after,
                    if_rev.as_deref(),
                )?
            } else {
                ticket::parts::add(
                    &store,
                    &target,
                    text.as_deref(),
                    &after,
                    &parts,
                    if_rev.as_deref(),
                )?
            };
            Ok(ticket_answer(mode, "add", &outcome))
        }
        DailyCommand::Note {
            target,
            text,
            if_rev,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "note",
                &ticket::note(&store, &target, &text, if_rev.as_deref())?,
            ))
        }
        DailyCommand::Edit {
            ticket: target,
            kind,
            labels,
            unlabels,
            part_of,
            priority,
            if_rev,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            let none = |v: Option<String>| v.map(|v| Some(v).filter(|v| v != "none" && v != "-"));
            let args = ticket::EditArgs {
                kind: none(kind),
                add_labels: labels,
                remove_labels: unlabels,
                part_of: none(part_of),
                priority,
                if_rev,
            };
            Ok(ticket_answer(
                mode,
                "edit",
                &ticket::edit(&store, &target, &args)?,
            ))
        }
        // M10: one id space. A light Warrant's id shows it whole, wherever
        // the command is `show` or `status`; an alias falls through to the
        // directory Warrant's projections below.
        DailyCommand::Show { alias, .. }
            if warrants::kind_of(&alias) == warrants::IdKind::Light =>
        {
            let (repository, store) = ctx.tickets(None)?;
            let shown = with_official(&repository, &alias, ticket::show(&store, &alias)?);
            Ok(ticket_answer(mode, "show", &shown))
        }
        // M10: a Warrant read in place shows from its folder, written to
        // never.
        DailyCommand::Show { alias, .. }
            if warrants::kind_of(&alias) == warrants::IdKind::ReadInPlace =>
        {
            let repository = ctx.open_repo()?;
            Ok(ticket_answer(
                mode,
                "show",
                &interop::adapters::show(&repository, &alias),
            ))
        }
        DailyCommand::Show { alias, view } => {
            let repository = ctx.open_repo()?;
            let mut rendered = show::run(&repository, &alias, &view)?;
            // OW-WAR-0137, OBL-004: the two views a reader opens to ask "who
            // reviewed this?" end with who acted, from the records. Here rather
            // than in show.rs, which OW-WAR-0033's resolution pins.
            let review = if matches!(view.as_str(), "full_warrant" | "status") {
                let review = status::review_of(&repository, &repository.warrant_dir(&alias)?)?;
                rendered.push_str(&status::render_review(&review));
                Some(review)
            } else {
                None
            };
            // OW-WAR-0148 M4: the declared states on record, only where any is.
            let declared = if matches!(view.as_str(), "full_warrant" | "status") {
                states::warrant_section(&repository, &alias).map(|(md, d)| {
                    rendered.push_str(&md);
                    d
                })
            } else {
                None
            };
            // OW-WAR-0148 M14: whether it is official, under a preset only.
            let official = if matches!(view.as_str(), "full_warrant" | "status") {
                official_section(&repository, &alias).map(|(md, v)| {
                    rendered.push_str(&md);
                    v
                })
            } else {
                None
            };
            let mut result = serde_json::json!({"alias": alias, "view": view, "rendered": rendered, "review": review});
            if let Some(d) = declared {
                result["declared_states"] = serde_json::json!(d);
            }
            if let Some(v) = official {
                result["official"] = v;
            }
            output::emit(mode, "show", &rendered, result);
            Ok(EXIT_OK)
        }
        DailyCommand::Status {
            alias: Some(alias), ..
        } if warrants::kind_of(&alias) == warrants::IdKind::Light => {
            let (repository, store) = ctx.tickets(None)?;
            let shown = with_official(&repository, &alias, ticket::show(&store, &alias)?);
            Ok(ticket_answer(mode, "status", &shown))
        }
        DailyCommand::Status {
            alias: Some(alias), ..
        } if warrants::kind_of(&alias) == warrants::IdKind::ReadInPlace => {
            let repository = ctx.open_repo()?;
            Ok(ticket_answer(
                mode,
                "status",
                &interop::adapters::show(&repository, &alias),
            ))
        }
        DailyCommand::Status {
            alias,
            timeline,
            pending,
        } => {
            let repository = ctx.open_repo()?;
            // One-shot and read-only: the tree does not move under it (t-eca6).
            gate_cmd::source::remember_tree_reads();
            if timeline || pending {
                let (what, value) = if timeline {
                    let t = timeline::build_timeline(&repository)?;
                    (
                        format!(
                            "{} event(s) across {} Warrant(s), {} day(s)",
                            t.events.len(),
                            t.warrants,
                            t.days.len()
                        ),
                        output::value(&t),
                    )
                } else {
                    let p = timeline::build_pending(&repository)?;
                    let lines: Vec<String> = p
                        .acts
                        .iter()
                        .map(|a| format!("{}  {}  {}", a.warrant, a.action, a.command))
                        .collect();
                    (
                        if lines.is_empty() {
                            "nothing awaits a signature".to_owned()
                        } else {
                            lines.join("\n")
                        },
                        output::value(&p),
                    )
                };
                output::emit(mode, "status", &what, value);
                return Ok(EXIT_OK);
            }
            match alias {
                Some(alias) => {
                    let mut rendered = show::run(&repository, &alias, "status")?;
                    // OW-WAR-0137, OBL-004: who acted, from the records.
                    let review = status::review_of(&repository, &repository.warrant_dir(&alias)?)?;
                    rendered.push_str(&status::render_review(&review));
                    let official = official_section(&repository, &alias).map(|(md, v)| {
                        rendered.push_str(&md);
                        v
                    });
                    let mut result = serde_json::json!({"alias": alias, "view": "status", "rendered": rendered, "review": review});
                    if let Some(v) = official {
                        result["official"] = v;
                    }
                    output::emit(mode, "status", &rendered, result);
                }
                None => match mode {
                    // The corpus projection IS canonical JSON already; under
                    // --json it rides inside the envelope as `result`, so the
                    // committed CORPUS_STATUS.json (written by `compile`) and
                    // this output agree on the payload.
                    // M10: with `[[adapters]]`, the Warrants read in place
                    // follow (`read_in_place`). They are never written into
                    // the committed projection: their folders are another
                    // tool's, and change without a compile.
                    output::Mode::Json => {
                        let (_, text) = status::corpus_status_json(&repository)?;
                        let mut value: serde_json::Value = serde_json::from_str(&text)
                            .map_err(|e| repo::RepoError::Message(format!("corpus status: {e}")))?;
                        let adapted = interop::adapters::load(&repository);
                        if !adapted.is_empty() {
                            value["read_in_place"] = interop::adapters::status(&adapted).1;
                        }
                        // OW-WAR-0148 M14: each Warrant's officialness, under
                        // a preset only; never in the committed projection.
                        let policy = preset::Policy::read_or_default(&repository.root);
                        if policy.configured() {
                            value["official"] =
                                serde_json::json!(official::corpus(&repository, &policy));
                        }
                        output::emit(mode, "status", &text, value);
                    }
                    output::Mode::Human => {
                        let (_, text) = status::corpus_status_md(&repository)?;
                        println!("{text}");
                        let adapted = interop::adapters::load(&repository);
                        if !adapted.is_empty() {
                            print!("{}", interop::adapters::status(&adapted).0);
                        }
                        // OW-WAR-0148 M13: how the tickets' ticks were
                        // earned, after the projection and never in it (the
                        // committed projection does not move when a ticket
                        // is worked).
                        if let Some(block) = ticket::Store::open(&repository, None)
                            .ok()
                            .and_then(|s| ticket::ladder::tracker(&s).ok())
                            .and_then(|t| t.render())
                        {
                            let gap = if text.ends_with('\n') { "" } else { "\n" };
                            println!("{gap}{}", block.trim_end());
                        }
                        // OW-WAR-0148 M14: under a preset, whether each
                        // Warrant is official, after the projection.
                        let policy = preset::Policy::read_or_default(&repository.root);
                        if policy.configured() {
                            let standings = official::corpus(&repository, &policy);
                            println!(
                                "\n{}",
                                official::corpus_block(&standings, &policy).trim_end()
                            );
                        }
                    }
                },
            }
            Ok(EXIT_OK)
        }
        DailyCommand::Init {
            namespace,
            name,
            program,
            guided,
            non_interactive: _,
            baseline,
            vibe,
            team,
            regulated,
        } => {
            // OW-WAR-0148 M14: the preset is written after everything plain
            // init writes, so without a flag the bytes are today's.
            let preset = [
                (vibe, preset::Preset::Vibe),
                (team, preset::Preset::Team),
                (regulated, preset::Preset::Regulated),
            ]
            .into_iter()
            .find_map(|(on, p)| on.then_some(p));
            let chosen = baseline
                .as_deref()
                .map_or(init::Baseline::Head, init::Baseline::Named);
            // The conversation is opt-in (M9): only `--guided` asks, and only
            // at a real terminal. A script, a pipe or `--json` is refused by
            // name and never meets a prompt it cannot answer.
            if guided {
                if ctx.json || !sign::at_a_terminal() {
                    return Err(Box::new(repo::RepoError::Message(
                        "`war init --guided` asks the setup questions, so it needs a terminal. \
                         Plain `war init` asks nothing and derives a namespace; `--namespace \
                         <NS>` picks one, and `--program <name>` adds the sign-off scaffold"
                            .to_owned(),
                    )));
                }
                init::guided(ctx.root.clone(), program.as_deref())?;
                return Ok(EXIT_OK);
            }
            match (namespace, program) {
                (Some(namespace), Some(program)) => {
                    init::run_program_with(&program, &namespace, ctx.root.clone(), chosen)?;
                }
                (None, Some(program)) => {
                    // M9: `--program` without `--namespace` derives one from
                    // the program's name, letters only (it prefixes
                    // `<NS>-SAS-RQ-001`), instead of refusing.
                    let derived = init::derive_namespace(&program, true);
                    init::run_program_with(&program, &derived, ctx.root.clone(), chosen)?;
                    println!(
                        "namespace {derived}, from the program's name (`--namespace` picks \
                         another)"
                    );
                }
                (namespace, None) => {
                    // OW-WAR-0147 / t-67ed: most work here starts as a
                    // ticket; the start hint is the first line after
                    // `initialized`. Plain `init` only: the `--program`
                    // scaffold's three lines are pinned (99-init, 59-adoption).
                    // M9: no `--namespace` means one derived from the
                    // directory name, never a refusal and never a question.
                    let dir = init::init_root(ctx.root.clone())?;
                    let derived = namespace.is_none().then(|| {
                        init::derive_namespace(dir.file_name().unwrap_or_default(), false)
                    });
                    let ns = namespace.or_else(|| derived.clone()).unwrap_or_default();
                    init::run_with(&ns, name.as_deref(), ctx.root.clone(), chosen, true)?;
                    if let Some(d) = derived {
                        println!(
                            "namespace {d}, from the directory name (`--namespace` picks another)"
                        );
                    }
                }
            }
            if let Some(p) = preset {
                for line in init::apply_preset(ctx.root.clone(), p)? {
                    println!("{line}");
                }
            }
            Ok(EXIT_OK)
        }
        DailyCommand::Next => {
            let repository = ctx.open_repo()?;
            // One-shot and read-only: the tree does not move under it (t-eca6).
            gate_cmd::source::remember_tree_reads();
            let next = next::run(&repository)?;
            output::emit(
                mode,
                "next",
                next::render(&next).trim_end(),
                output::value(&next),
            );
            Ok(EXIT_OK)
        }
        DailyCommand::Check {
            pr: Some(pr),
            repo,
            summary,
            comment,
            ..
        } => {
            let repository = ctx.open_repo()?;
            let args = pr_gate::Args {
                pr,
                repo,
                summary,
                comment,
            };
            let (mut report, answer) = pr_gate::run(&repository, &args);
            pr_gate::publish(&repository, &args, &mut report, &answer);
            match mode {
                output::Mode::Human => {
                    for d in &report.diagnostics {
                        println!("{d}");
                    }
                    println!(
                        "PR #{} in {}: {}",
                        answer.pr,
                        if answer.repo.is_empty() {
                            "?"
                        } else {
                            answer.repo.as_str()
                        },
                        match answer.verdict {
                            "pass" => "passes",
                            "not_required" => "passes (the preset does not require it)",
                            "refused" => "refused",
                            _ => "UNKNOWN, never a pass",
                        }
                    );
                }
                output::Mode::Json => println!(
                    "{}",
                    output::envelope("check.pr", &report, Some(output::value(&answer)))
                ),
            }
            Ok(output::exit_code(&report))
        }
        DailyCommand::Check {
            alias, generated, ..
        } => {
            let repository = ctx.open_repo()?;
            // A ticket is checked for its structure only (OW-WAR-0147): never
            // for a signature, evidence or verification it does not need.
            if let Some(a) = alias.as_deref().filter(|a| ticket::is_ticket_ref(a)) {
                let mut report = diagnostic::Report::default();
                match ticket::Store::open(&repository, None) {
                    Ok(store) => ticket::check(&store, Some(a), &mut report)?,
                    Err(e) => report.push(diagnostic::Diagnostic::error(
                        "tickets.config",
                        init::CONFIG_FILE.to_owned(),
                        e.to_string(),
                    )),
                }
                return Ok(output::finish(mode, "check", &report, None));
            }
            // M10: a Warrant read in place is checked for what its folder
            // says, read where it is; nothing is written.
            if let Some(a) = alias
                .as_deref()
                .filter(|a| warrants::kind_of(a) == warrants::IdKind::ReadInPlace)
            {
                let mut report = diagnostic::Report::default();
                interop::adapters::check(&repository, Some(a), &mut report);
                return Ok(output::finish(mode, "check", &report, None));
            }
            // One-shot and read-only: the tree does not move under it (t-eca6).
            gate_cmd::source::remember_tree_reads();
            let mut report = check::run(&repository, alias.as_deref(), generated)?;
            if alias.is_none() {
                match ticket::Store::open(&repository, None) {
                    Ok(store) => ticket::check(&store, None, &mut report)?,
                    Err(e) => report.push(diagnostic::Diagnostic::error(
                        "tickets.config",
                        init::CONFIG_FILE.to_owned(),
                        e.to_string(),
                    )),
                }
                interop::adapters::check(&repository, None, &mut report);
                // OW-WAR-0148 M14: a [preset], [roles] or [notify] table that
                // does not read is refused by key; an absent one says nothing.
                if let Err(e) = preset::Policy::read(&repository.root) {
                    report.push(diagnostic::Diagnostic::error(
                        preset::CONFIG_RULE,
                        init::CONFIG_FILE.to_owned(),
                        e,
                    ));
                }
            }
            // A non-zero exit for an unsound Warrant is what lets CI gate on it.
            Ok(output::finish(mode, "check", &report, None))
        }
    }
}

fn run_plan(ctx: &Ctx, args: PlanArgs) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    let PlanArgs {
        command,
        request,
        profile,
        assurance,
        proposal,
        reviewed,
        draft,
        out,
        answers,
        apply,
        issue,
        issue_file,
        records,
        area,
    } = args;
    if let Some(command) = command {
        return run_plan_member(ctx, command);
    }
    let repository = ctx.open_repo()?;
    let mut answer_map: std::collections::BTreeMap<String, String> = answers
        .iter()
        .filter_map(|a| {
            a.split_once('=')
                .map(|(k, v)| (k.trim().to_owned(), v.to_owned()))
        })
        .collect();
    if records {
        let args = plan::records::PlanArgs {
            sentence: request,
            profile,
            area,
            proposal,
            draft,
            reviewed,
            apply,
            out,
            answers: answer_map,
        };
        return Ok(plan::records::run_plan(mode, &repository, &args)?);
    }
    // OW-WAR-0141: an issue in place of the sentence, and the answers a
    // human already gave to this input's intake questions.
    let intake = plan::resolve_intake(
        &repository,
        &request,
        issue.as_deref(),
        issue_file.as_deref(),
    )?;
    let request = intake.as_ref().map_or(request, |i| i.sentence.clone());
    if let Some(i) = &intake {
        for (id, text) in &i.answers {
            answer_map.entry(id.clone()).or_insert_with(|| text.clone());
        }
    }
    let answered: std::collections::BTreeSet<String> = answer_map.keys().cloned().collect();
    let req = plan::request(&repository, &request, &profile, &assurance, &answer_map)?;

    // Where the proposal comes from: a file, or the configured drafter.
    // A proposal written to the default scratch path is removed once
    // `--apply` has recorded it under plan/; one the user named is theirs.
    let scratch = out.is_none();
    let (proposal_path, drafter_run) = match (proposal, draft) {
        (Some(path), _) => (Some(path), None),
        (None, true) => {
            let (json, run) = plan::run_drafter(&repository, &req)?;
            let path = out.unwrap_or_else(|| plan::default_out(&repository));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    repo::RepoError::Message(format!("cannot create {parent}: {e}"))
                })?;
            }
            std::fs::write(&path, &json)
                .map_err(|e| repo::RepoError::Message(format!("cannot write {path}: {e}")))?;
            eprintln!("{}", plan::scratch_note(&path));
            (Some(path), Some(run))
        }
        (None, false) => (None, None),
    };

    let Some(path) = proposal_path else {
        // The request half: emit and stop.
        if request.trim().is_empty() {
            return Err(Box::new(repo::RepoError::Message(
                "war plan needs a request sentence, or --proposal <file>".to_owned(),
            )));
        }
        let human = serde_json::to_string_pretty(&req).expect("request serializes");
        output::emit(mode, "plan.request", &human, output::value(&req));
        if repository.config.plan.drafter_argv.is_empty() {
            eprintln!(
                "\nNo drafter is configured. Hand this request to an agent speaking \
                 `oh.war/agent-drafter/v1` (§75.2) and return its proposal with \
                 `war plan --proposal <file>`; or set [plan] drafter_argv and use --draft."
            );
        }
        return Ok(EXIT_OK);
    };

    // The return half: the §74.4 gauntlet, then --apply.
    let json = std::fs::read_to_string(&path)
        .map_err(|e| repo::RepoError::Message(format!("cannot read {path}: {e}")))?;
    let is_v1 = serde_json::from_str::<serde_json::Value>(&json)
        .ok()
        .and_then(|v| {
            v.get("api_version")
                .and_then(|a| a.as_str().map(str::to_owned))
        })
        .is_some_and(|a| a == "oh.war/draft-proposal/v1");
    if is_v1 {
        if apply {
            return Err(Box::new(repo::RepoError::Message(
                "plan.v1-has-no-payloads: a v1 proposal validates but its operations carry \
                 no payload, so it cannot be applied. Return an oh.war/draft-proposal/v2"
                    .to_owned(),
            )));
        }
        let known = plan::known_refs(&repository)?;
        let (parsed, pipeline) = show::plan::validate_proposal(&json, reviewed, &known)?;
        let mut report = diagnostic::Report::default();
        match pipeline.may_apply() {
            Ok(()) => report.push(diagnostic::Diagnostic::pass(
                "plan.applicable",
                format!(
                    "v1 proposal is applicable in principle: {} operation(s), {} ADR draft(s) — but v1 carries no payloads; --apply needs v2",
                    parsed.atom_operations.len(),
                    parsed.proposed_adr_drafts.len()
                ),
            )),
            Err(e) => report.push(diagnostic::Diagnostic::warn(
                "plan.not-applicable",
                path.to_string(),
                format!("validated, not applicable yet: {e}"),
            )),
        }
        let code = output::finish(mode, "plan.validate", &report, None);
        return Ok(if code == EXIT_OK && pipeline.may_apply().is_err() {
            EXIT_NOT_READY
        } else {
            code
        });
    }
    let known = plan::known_refs(&repository)?;
    let review = plan::review_of(
        &repository,
        intake.as_ref(),
        drafter_run.is_some(),
        reviewed,
    )?;
    let (parsed, mut pipeline) =
        match plan::validate_v2(&json, review.completes_the_step(), &answered, &known) {
            Ok(v) => v,
            Err(e) => match plan::record_questions(
                &repository,
                intake.as_ref(),
                drafter_run.as_ref(),
                &req,
                &json,
                &e,
            )? {
                Some((report, asked)) => {
                    if scratch && drafter_run.is_some() {
                        let _ = std::fs::remove_file(&path);
                    }
                    let code = output::finish(
                        mode,
                        "plan.interview",
                        &report,
                        Some(output::value(&asked)),
                    );
                    return Ok(if code == EXIT_OK {
                        EXIT_NOT_READY
                    } else {
                        code
                    });
                }
                None => return Err(Box::new(e)),
            },
        };
    if !apply {
        let mut report = diagnostic::Report::default();
        match pipeline.may_apply() {
            Ok(()) => report.push(diagnostic::Diagnostic::pass(
                "plan.applicable",
                format!(
                    "proposal is applicable: {} operation(s); run again with --apply",
                    parsed.operations.len()
                ),
            )),
            Err(e) => report.push(diagnostic::Diagnostic::warn(
                "plan.not-applicable",
                path.to_string(),
                format!("validated, not applicable yet: {e}"),
            )),
        }
        let ready = pipeline.may_apply().is_ok();
        let code = output::finish(
            mode,
            "plan.validate",
            &report,
            Some(output::value(&pipeline)),
        );
        return Ok(if ready { code } else { EXIT_NOT_READY });
    }
    let intake_record = intake.as_ref().and_then(plan::Intake::record);
    let applied = plan::apply_with(
        &repository,
        &parsed,
        &mut pipeline,
        &req,
        drafter_run.as_ref(),
        &json,
        review,
        intake_record.as_ref(),
    );
    if applied.is_err() && scratch && drafter_run.is_some() {
        // A refused apply writes nothing, the scratch proposal included.
        let _ = std::fs::remove_file(&path);
    }
    let (applied, report) = applied?;
    if scratch && drafter_run.is_some() {
        // Recorded verbatim under plan/proposal.json; the scratch copy
        // under generated/ would otherwise accumulate.
        let _ = std::fs::remove_file(&path);
    }
    Ok(output::finish(
        mode,
        "plan.apply",
        &report,
        Some(output::value(&applied)),
    ))
}

fn run_plan_member(ctx: &Ctx, command: PlanCommand) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        PlanCommand::Promote {
            ticket: target,
            actor,
        } => {
            let (repository, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "promote",
                &ticket::promote(&repository, &store, &target)?,
            ))
        }
        PlanCommand::New {
            title,
            preset,
            profile,
            parent,
        } => {
            // OW-WAR-0140: a profile is a name the repository's registry
            // resolves — the two core ones and every `profiles/<name>.toml`.
            let repository = ctx.open_repo()?;
            let profile: Option<Profile> = profile
                .map(|p| repository.profiles.resolve(&p))
                .transpose()?;
            // Without --preset the profile's preset is written, and the
            // presets are named: the `TODO` skeleton is retired. The refusal
            // OW-WAR-0113 D-031 asks for here is withheld — see its report:
            // plants and tests outside its declared set call `war new <title>`.
            let (preset, defaulted) = match preset {
                Some(p) => (p, false),
                None => (
                    new::default_preset(profile.as_ref().unwrap_or(&Profile::Delivery)).to_owned(),
                    true,
                ),
            };
            let Some(composes) = new::preset_profile(&preset) else {
                let mut report = diagnostic::Report::default();
                report.push(diagnostic::Diagnostic::error(
                    "new.unknown-preset",
                    preset.clone(),
                    format!(
                        "no preset is named {preset:?}; the presets are {}",
                        new::preset_names().join(", ")
                    ),
                ));
                output::finish(mode, "new", &report, None);
                return Ok(EXIT_DIAGNOSTIC);
            };
            // A profile that extends the preset's core profile composes from
            // it; the namespaced roles it adds are written from its definition.
            if let Some(p) = &profile
                && p.core() != composes.core()
            {
                let mut report = diagnostic::Report::default();
                report.push(diagnostic::Diagnostic::error(
                    "new.preset-profile",
                    preset.clone(),
                    format!("preset {preset:?} composes the `{composes}` profile, not `{p}`"),
                ));
                output::finish(mode, "new", &report, None);
                return Ok(EXIT_DIAGNOSTIC);
            }
            let profile = profile.unwrap_or(composes);
            let dir = new::run_profile(&repository, &title, &preset, &profile, parent.as_deref())?;
            let rel = repository.relative(&dir);
            let alias = dir.file_name().unwrap_or_default().to_owned();
            let mut data = serde_json::json!({"alias": alias, "dir": rel, "profile": profile.to_string(), "preset": preset, "presets": new::preset_names()});
            if let Some(parent) = &parent {
                data["parent"] = serde_json::json!(parent);
            }
            output::emit(
                mode,
                "new",
                &format!(
                    "created {rel} from the `{preset}` preset{}\nanswer each heading's question in its atoms, then run `war check`",
                    if defaulted {
                        format!(
                            " (no --preset given; the presets are {})",
                            new::preset_names().join(", ")
                        )
                    } else {
                        String::new()
                    }
                ),
                data,
            );
            Ok(EXIT_OK)
        }
        PlanCommand::Impact { record } => {
            let repository = ctx.open_repo()?;
            gate_cmd::source::remember_tree_reads();
            let corpus = corpus::Corpus::new(&repository);
            let (report, impact) = impact::build(&corpus, &record)?;
            match mode {
                output::Mode::Human => {
                    if let Some(i) = &impact {
                        print!("{}", impact::render(i));
                    }
                    for d in report
                        .diagnostics
                        .iter()
                        .filter(|d| d.severity != diagnostic::Severity::Pass)
                    {
                        eprintln!("{d}");
                    }
                    if let Some(i) = &impact {
                        println!("\n{}", impact::summary(i));
                    }
                    Ok(output::exit_code(&report))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "impact",
                    &report,
                    impact.as_ref().map(output::value),
                )),
            }
        }
        // ---- OW-WAR-0148 M4: declared states -----------------------------
        PlanCommand::State {
            record,
            name,
            note,
            actor,
        } => {
            let repository = ctx.open_repo()?;
            Ok(ticket_answer(
                mode,
                "state",
                &states::enter(
                    &repository,
                    &record,
                    &name,
                    note.as_deref(),
                    actor.as_deref(),
                )?,
            ))
        }
        PlanCommand::Render {
            projection,
            of,
            max_bytes,
        } => {
            let repository = ctx.open_repo()?;
            gate_cmd::source::remember_tree_reads();
            let corpus = corpus::Corpus::new(&repository);
            let (report, rendered) =
                render_cmd::run(&corpus, &projection, of.as_deref(), max_bytes)?;
            match mode {
                output::Mode::Human => {
                    if let Some(p) = &rendered {
                        print!("{}", p.content);
                    }
                    for d in report
                        .diagnostics
                        .iter()
                        .filter(|d| d.severity != diagnostic::Severity::Pass)
                    {
                        eprintln!("{d}");
                    }
                    Ok(output::exit_code(&report))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "render",
                    &report,
                    rendered.as_ref().map(output::value),
                )),
            }
        }
        PlanCommand::Model => {
            let repository = ctx.open_repo()?;
            gate_cmd::source::remember_tree_reads();
            let corpus = corpus::Corpus::new(&repository);
            let (report, model) = model::run(&corpus)?;
            match mode {
                output::Mode::Human => {
                    for d in &model.diagnostics {
                        println!("{}  {}  {}", d.rule, d.record, d.message);
                    }
                    println!("{}", model::summary(&model));
                    Ok(output::exit_code(&report))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "model",
                    &report,
                    Some(output::value(&model)),
                )),
            }
        }
        PlanCommand::Roadmap { command } => {
            let repository = ctx.open_repo()?;
            match command {
                None => {
                    let (report, view) = roadmap_cmd::view(&repository)?;
                    match mode {
                        output::Mode::Human => {
                            print!("{}", roadmap_cmd::render(&view));
                            Ok(EXIT_OK)
                        }
                        output::Mode::Json => Ok(output::finish(
                            mode,
                            "roadmap",
                            &report,
                            Some(output::value(&view)),
                        )),
                    }
                }
                Some(RoadmapCommand::Assign { alias, phase }) => Ok(output::finish(
                    mode,
                    "roadmap",
                    &roadmap_cmd::assign(&repository, &alias, &phase)?,
                    None,
                )),
                Some(RoadmapCommand::Edit) => Ok(roadmap_edit::run(&repository)?),
                Some(RoadmapCommand::Propose { note }) => Ok(output::finish(
                    mode,
                    "roadmap",
                    &roadmap_cmd::propose(&repository, note.as_deref())?,
                    None,
                )),
            }
        }
        PlanCommand::Ask {
            alias,
            stage,
            question,
            recommend,
            blocking,
        } => {
            let repository = ctx.open_repo()?;
            let mut report =
                questions::ask(&repository, &alias, &stage, &question, &recommend, blocking)?;
            // OW-WAR-0148 M14: a question waits on a person; `[notify]` says
            // so when it is configured.
            if report.is_ready()
                && let Some(d) = notify::human_waits(
                    &repository.root,
                    &notify::Wait {
                        event: "question.asked",
                        subject: &alias,
                        message: &format!("{alias}/{stage} asks: {question}"),
                        command: &format!("war plan questions {alias}"),
                    },
                )
            {
                report.push(d);
            }
            Ok(output::finish(mode, "ask", &report, None))
        }
        PlanCommand::Answer {
            alias,
            id,
            answer,
            actor,
        } => {
            let repository = ctx.open_repo()?;
            let report = questions::answer(&repository, &alias, &id, &answer, &actor)?;
            Ok(output::finish(mode, "answer", &report, None))
        }
        PlanCommand::Questions { alias, open } => {
            let repository = ctx.open_repo()?;
            let (report, list) = questions::list(&repository, alias.as_deref(), open)?;
            match mode {
                output::Mode::Human => {
                    print!("{}", questions::render(&list));
                    Ok(output::finish(mode, "questions", &report, None))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "questions",
                    &report,
                    Some(output::value(&list)),
                )),
            }
        }
        PlanCommand::Answers { alias, stage } => {
            let repository = ctx.open_repo()?;
            let answered = questions::answers_for(&repository, &alias, stage.as_deref())?;
            let human = if answered.is_empty() {
                "no answered question for this stage\n".to_owned()
            } else {
                answered
                    .iter()
                    .map(|q| {
                        format!(
                            "{} ({}): {}\n  → {}\n",
                            q.id,
                            q.stage,
                            q.question,
                            q.answer.as_ref().map_or("", |a| a.answer.as_str())
                        )
                    })
                    .collect()
            };
            output::emit(mode, "answers", human.trim_end(), output::value(&answered));
            Ok(EXIT_OK)
        }
        PlanCommand::Frontier { alias } => {
            let repository = ctx.open_repo()?;
            let (report, f) = frontier::run(&repository, alias.as_deref())?;
            match mode {
                output::Mode::Human => {
                    print!("{}", frontier::render(&f));
                    Ok(output::finish(mode, "frontier", &report, None))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "frontier",
                    &report,
                    Some(output::value(&f)),
                )),
            }
        }
    }
}

fn run_sign(ctx: &Ctx, args: SignArgs) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match args {
        SignArgs {
            command: Some(SignGroup::Member(command)),
            ..
        } => run_sign_member(ctx, command),
        // OW-WAR-0148 M14: a person's approval of a Warrant.
        SignArgs {
            command:
                Some(SignGroup::Approve {
                    id,
                    actor,
                    meaning,
                    ssh_sign,
                    dry_run,
                }),
            ..
        } => {
            let repository = ctx.open_repo()?;
            let store = ticket::Store::open(&repository, None)?;
            let args = official::ApproveArgs {
                actor,
                meaning,
                ssh_sign,
                dry_run,
            };
            Ok(ticket_answer(
                mode,
                "sign.approve",
                &official::approve(&repository, &store, &id, &args)?,
            ))
        }
        // OW-WAR-0148 M14: one batch of what the preset asks before a release.
        SignArgs {
            command:
                Some(SignGroup::Release {
                    tag,
                    actor,
                    ssh_sign,
                    dry_run,
                }),
            ..
        } => {
            let repository = ctx.open_repo()?;
            let (report, result, human) =
                release_cmd::run(&repository, &tag, actor, ssh_sign, dry_run)?;
            match mode {
                output::Mode::Human => {
                    if !human.is_empty() {
                        println!("{human}");
                    }
                    check::print(&report);
                }
                output::Mode::Json => println!(
                    "{}",
                    output::envelope("sign.release", &report, Some(result))
                ),
            }
            Ok(output::exit_code(&report))
        }

        // OW-WAR-0148 M13: a human's sign-off of one ticket item.
        SignArgs {
            target: Some(target),
            actor,
            meaning,
            ssh_sign,
            dry_run,
            ..
        } if ticket::is_ticket_ref(&target) => {
            let repository = ctx.open_repo()?;
            let store = ticket::Store::open(&repository, None)?;
            let args = ticket::acts::SignArgs {
                actor,
                meaning,
                ssh_sign,
                dry_run,
            };
            Ok(ticket_answer(
                mode,
                "sign",
                &ticket::acts::sign(&repository, &store, &target, &args)?,
            ))
        }
        SignArgs {
            command: None,
            target,
            list,
            all,
            actor,
            meaning,
            grounds,
            outcome,
            adr,
            independence,
            edit,
            show,
            dry_run,
            ssh_sign,
            verify,
            kind,
            batch,
            recover,
            revoke,
        } => {
            let repository = ctx.open_repo()?;
            if list {
                // OW-WAR-0137: `--as <actor>` is that actor's queue — the acts
                // they may sign now, assigned ones first — and `--json` carries
                // the same list.
                let waiting = sign::list_for(&repository, actor.as_deref())?;
                let by = actor
                    .as_deref()
                    .map(|a| format!(" by {a}"))
                    .unwrap_or_default();
                let text = if waiting.is_empty() {
                    format!("nothing awaits a signature{by}")
                } else {
                    let mut t = format!("{} awaiting a signature{by}:", waiting.len());
                    for p in &waiting {
                        t.push_str(&format!("\n  {}", sign::line(p)));
                    }
                    t
                };
                output::emit(
                    mode,
                    "sign",
                    &text,
                    sign::list_json(&waiting, actor.as_deref()),
                );
                return Ok(EXIT_OK);
            }
            let parse_err = |what: &str, v: &str, known: &str| {
                repo::RepoError::Message(format!("--{what} {v:?} is not one of {known}"))
            };
            let outcome = outcome
                .as_deref()
                .map(|s| {
                    s.parse::<openwarrant_core::resolution::CommonOutcome>()
                        .map_err(|_| parse_err("outcome", s, "not_satisfied, cancelled, blocked"))
                })
                .transpose()?;
            let independence = match independence.as_str() {
                "none" => openwarrant_core::Independence::None,
                "separate_role" => openwarrant_core::Independence::SeparateRole,
                "organizational" => openwarrant_core::Independence::Organizational,
                other => {
                    return Err(parse_err(
                        "independence",
                        other,
                        "none, separate_role, organizational",
                    )
                    .into());
                }
            };
            let kind = kind
                .as_deref()
                .map(|s| {
                    s.parse::<openwarrant_core::correction::CorrectionKind>()
                        .map_err(|e| repo::RepoError::Message(format!("--kind: {e}")))
                })
                .transpose()?;
            let opts = sign::Options {
                actor,
                // An invalidation's grounds travel as its meaning: they are
                // the signer's words, as a correction's reason is.
                meaning: grounds.or(meaning),
                outcome,
                adr_ref: adr,
                independence,
                edit,
                all,
                show,
                dry_run,
                ssh_sign,
                verify,
                kind,
                revoke,
            };
            if batch.is_some() || recover.is_some() {
                let mut targets: Vec<String> = batch
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| !t.is_empty())
                    .collect();
                targets.extend(target);
                // Named with anything else, `batch.recover-alone` refuses it.
                targets.extend(recover.map(|id| format!("{}{id}", batch_cmd::RECOVER_PREFIX)));
                let mut report = batch_cmd::run(&repository, &targets, &opts)?;
                signing_probe::finish(&repository, &mut report);
                return Ok(output::finish(mode, "sign", &report, None));
            }
            let report = sign::run(&repository, target.as_deref(), &opts)?;
            Ok(output::finish(mode, "sign", &report, None))
        }
    }
}

fn run_sign_member(ctx: &Ctx, command: SignCommand) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        SignCommand::Authority { command } => {
            let (report, result) = authority_cmd::run(command)?;
            Ok(output::finish(mode, "authority", &report, Some(result)))
        }
        SignCommand::Correct {
            alias,
            deliverable_id,
            response,
        } => {
            let repository = ctx.open_repo()?;
            match response {
                Some(path) => {
                    let report = correct::ingest(&repository, &alias, &deliverable_id, &path)?;
                    Ok(output::finish(mode, "correct", &report, None))
                }
                None => {
                    let request = correct::request(&repository, &alias, &deliverable_id)?;
                    output::emit(
                        mode,
                        "correct.request",
                        &toml::to_string_pretty(&request).map_err(|e| {
                            repo::RepoError::Message(format!("could not render the request: {e}"))
                        })?,
                        output::value(&request),
                    );
                    if !request.resolved {
                        eprintln!(
                            "# {alias} is NOT resolved: regenerate deliverables.toml instead. A \
                             correction is for a pin a resolution holds."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    if !request.drift {
                        eprintln!(
                            "# {alias}/{deliverable_id}: the file is at the digest on record; nothing to correct."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    eprintln!(
                        "# {alias}/{deliverable_id}: {} → {} (correction {}). Sign with `war sign {alias}/{deliverable_id}`.",
                        request.chain_head, request.current_digest, request.next_sequence
                    );
                    Ok(EXIT_OK)
                }
            }
        }
        SignCommand::Resolve {
            alias,
            dry_run,
            response,
        } => {
            let repository = ctx.open_repo()?;
            if dry_run {
                let report = resolve::run(&repository, &alias)?;
                return Ok(output::finish(mode, "resolve.dry_run", &report, None));
            }
            match response {
                Some(path) => {
                    let report = resolution_cmd::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "resolve", &report, None))
                }
                None => {
                    let request = resolution_cmd::request(&repository, &alias)?;
                    output::emit(
                        mode,
                        "resolve.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the resolution request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    if request.requirements_met {
                        // OW-WAR-0148 M14: `[notify]`, when configured.
                        notify::say(
                            notify::human_waits(
                                &repository.root,
                                &notify::Wait {
                                    event: "resolve.requested",
                                    subject: &alias,
                                    message: &format!("{alias} waits on a resolution"),
                                    command: &format!("war sign {alias}"),
                                },
                            )
                            .as_ref(),
                        );
                        eprintln!(
                            "# {alias}: all 13 §56.1 requirements met. Permitted outcomes: {}. \
                             Eligible resolvers: {}.",
                            request.permitted_outcomes.join(", "),
                            if request.eligible_resolvers.is_empty() {
                                "NOBODY — docs/authority/roles.toml grants `resolver` to no human"
                                    .to_owned()
                            } else {
                                request.eligible_resolvers.join(", ")
                            }
                        );
                        Ok(EXIT_OK)
                    } else {
                        eprintln!(
                            "# {alias}: {} of 13 §56.1 requirements unmet; a response would be refused.",
                            request.unmet.len()
                        );
                        Ok(EXIT_NOT_READY)
                    }
                }
            }
        }
        SignCommand::Standing { command } => {
            let repository = ctx.open_repo()?;
            match command {
                StandingCommand::Propose { file, dry_run } => Ok(output::finish(
                    mode,
                    "standing",
                    &standing_cmd::propose(&repository, &file, dry_run)?,
                    None,
                )),
                StandingCommand::Show { id } => {
                    let (report, views) = standing_cmd::show(&repository, id.as_deref())?;
                    Ok(output::finish(
                        mode,
                        "standing",
                        &report,
                        Some(output::value(&views)),
                    ))
                }
                StandingCommand::Apply {
                    alias,
                    class,
                    dry_run,
                } => Ok(output::finish(
                    mode,
                    "standing",
                    &standing_cmd::apply(&repository, &alias, class.as_deref(), dry_run)?,
                    None,
                )),
            }
        }
        SignCommand::Amend { alias, dry_run } => {
            let repository = ctx.open_repo()?;
            Ok(output::finish(
                mode,
                "amend",
                &amendment_id::amend(&repository, &alias, dry_run)?,
                None,
            ))
        }
        SignCommand::Sas { command } => {
            let repository = ctx.open_repo()?;
            let ready = |report: diagnostic::Report| Ok(output::finish(mode, "sas", &report, None));
            match command {
                SasCommand::Propose { version } => ready(sas::propose(&repository, &version)?),
                SasCommand::Repin {
                    alias,
                    all,
                    reason,
                    dry_run,
                } => ready(sas_repin::run(
                    &repository,
                    &sas_repin::Options {
                        alias,
                        all,
                        reason,
                        dry_run,
                    },
                )?),
                SasCommand::Accept { version, response } => match response {
                    Some(path) => ready(sas::accept_ingest(&repository, &version, &path)?),
                    None => {
                        let request = sas::accept_request(&repository, &version)?;
                        output::emit(
                            mode,
                            "sas.accept.request",
                            &toml::to_string_pretty(&request).map_err(|e| {
                                repo::RepoError::Message(format!(
                                    "could not render the request: {e}"
                                ))
                            })?,
                            output::value(&request),
                        );
                        eprintln!(
                            "# SAS {} at sha256:{} — architecture-changing: {}{}",
                            request.version,
                            &request.sha256[..12],
                            request.architecture_changing,
                            if request.adr_required {
                                " (an adr_ref is REQUIRED, §101.3)"
                            } else {
                                ""
                            }
                        );
                        if request.eligible_acceptors.is_empty() {
                            eprintln!(
                                "# NOBODY may accept this: docs/authority/roles.toml grants the authorizer\n\
                                 # role to no one. Only a human may write that file."
                            );
                        } else {
                            eprintln!(
                                "# May be accepted by: {}",
                                request.eligible_acceptors.join(", ")
                            );
                        }
                        eprintln!(
                            "# Fill in accepted_by, acting_role, meaning, effective_time, then:\n\
                             #   war sign sas accept {} --response <file>",
                            request.version
                        );
                        Ok(EXIT_OK)
                    }
                },
                SasCommand::Diff { candidate } => ready(sas::diff(&repository, &candidate)?),
                SasCommand::Status => ready(sas::status(&repository)?),
            }
        }
        SignCommand::Attest {
            target,
            verify,
            all,
            custody,
            record,
        } => {
            let repository = ctx.open_repo()?;
            if custody {
                let Some(t) = target else {
                    return Err(Box::new(repo::RepoError::Message(
                        "war sign attest --custody: name the resolved Warrant to audit".to_owned(),
                    )));
                };
                let (report, audit) = attest::custody(&repository, &t, record.as_deref())?;
                return Ok(output::finish(
                    mode,
                    "attest.custody",
                    &report,
                    audit.as_ref().map(output::value),
                ));
            }
            let report = match (target, all) {
                (_, true) => attest::verify_all(&repository)?,
                (Some(t), false) => attest::run(&repository, &t, verify)?,
                (None, false) => {
                    return Err(Box::new(repo::RepoError::Message(
                        "war sign attest: name a Warrant or SAS version, or pass --all".to_owned(),
                    )));
                }
            };
            Ok(output::finish(mode, "attest", &report, None))
        }
        SignCommand::Inbox { actor } => {
            let repository = ctx.open_repo()?;
            let inbox = inbox::run_as(&repository, actor.as_deref())?;
            output::emit(
                mode,
                "inbox",
                inbox::render(&inbox).trim_end(),
                output::value(&inbox),
            );
            Ok(EXIT_OK)
        }
        SignCommand::Authorize { alias, response } => {
            let repository = ctx.open_repo()?;
            match response {
                Some(path) => {
                    let report = authorize::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "authorize", &report, None))
                }
                None => {
                    let request = authorize::request(&repository, &alias)?;
                    output::emit(
                        mode,
                        "authorize.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the authorization request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    eprintln!(
                        "# {alias} at {} assurance, contract {}.",
                        request.assurance_level, request.contract_digest
                    );
                    if request.eligible_authorizers.is_empty() {
                        // Naming this here rather than at ingestion time saves a
                        // round trip through a signature that could never have
                        // been accepted.
                        eprintln!(
                            "# NOBODY may authorize this: docs/authority/roles.toml grants the\n\
                             # authorizer role to no one. Authority comes from that file, and\n\
                             # only a human may write it."
                        );
                    } else {
                        eprintln!(
                            "# May be authorized by: {}",
                            request.eligible_authorizers.join(", ")
                        );
                    }
                    eprintln!(
                        "# Fill in authorizer, acting_role, meaning and effective_time, then:\n\
                         #   war sign authorize {alias} --response <file>"
                    );
                    // OW-WAR-0148 M14: `[notify]`, when configured.
                    notify::say(
                        notify::human_waits(
                            &repository.root,
                            &notify::Wait {
                                event: "authorize.requested",
                                subject: &alias,
                                message: &format!("{alias} waits on an authorization"),
                                command: &format!("war sign {alias}"),
                            },
                        )
                        .as_ref(),
                    );
                    Ok(EXIT_OK)
                }
            }
        }
    }
}

fn run_evidence(ctx: &Ctx, command: EvidenceGroup) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        EvidenceGroup::Member(command) => run_evidence_member(ctx, command),
        EvidenceGroup::Record {
            alias,
            gate,
            evidence_ref,
        } => {
            let repository = ctx.open_repo()?;
            let report = evidence::record(
                &repository,
                &alias,
                gate.as_deref(),
                evidence_ref.as_deref(),
            )?;
            Ok(output::finish(mode, "evidence", &report, None))
        }
    }
}

fn run_evidence_member(
    ctx: &Ctx,
    command: EvidenceCommand,
) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        EvidenceCommand::Kpi {
            command: KpiCommand::Run { target, actor },
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "kpi",
                &ticket::ladder::kpi_run(&store, &target)?,
            ))
        }

        EvidenceCommand::Gate {
            action:
                Some(GateAction::Invalidate {
                    gate,
                    response: Some(path),
                    ..
                }),
            ..
        } => {
            let repository = ctx.open_repo()?;
            let report = invalidation::ingest(&repository, &gate, &path)?;
            Ok(output::finish(mode, "gate.invalidate", &report, None))
        }
        EvidenceCommand::Gate {
            action: Some(GateAction::Invalidate { gate, grounds, .. }),
            ..
        } => {
            let repository = ctx.open_repo()?;
            let grounds = grounds.unwrap_or_default();
            let (report, request) = invalidation::request(&repository, &gate, &grounds)?;
            match request {
                Some(req) => {
                    if matches!(mode, output::Mode::Human) {
                        println!(
                            "{}",
                            toml::to_string_pretty(&req).map_err(|e| repo::RepoError::Io {
                                context: "could not render the invalidation request".to_owned(),
                                source: std::io::Error::other(e.to_string()),
                            })?
                        );
                    }
                    Ok(output::finish(
                        mode,
                        "gate.invalidate",
                        &report,
                        Some(output::value(&req)),
                    ))
                }
                None => Ok(output::finish(mode, "gate.invalidate", &report, None)),
            }
        }
        EvidenceCommand::Gate {
            action: None,
            run,
            gate,
            record,
            subject_digests,
            evidence_refs,
        } => {
            if !record && (!subject_digests.is_empty() || !evidence_refs.is_empty()) {
                return Err(Box::new(repo::RepoError::Message(
                    "--subject-digest and --evidence-ref require --record".to_owned(),
                )));
            }
            let repository = ctx.open_repo()?;
            let report = gate_cmd::run(
                &repository,
                run,
                gate.as_deref(),
                record,
                &subject_digests,
                &evidence_refs,
                None,
            )?;
            let _ = output::finish(mode, "gate", &report, None);
            // §44.1 and RQ-054: an unaskable gate is NOT a pass, and is not a
            // failure either. `is_ready()` blocks on unknowns, so both land on a
            // non-zero exit without the two being conflated in the report.
            Ok(if report.is_ready() {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }
        EvidenceCommand::Prepare {
            aliases,
            all,
            jobs,
            no_commit,
            reverify,
            dry_run,
        } => {
            let repository = ctx.open_repo()?;
            let options = prepare::Options {
                aliases,
                all,
                jobs: jobs as usize,
                commit: !no_commit,
                reverify,
                dry_run,
            };
            let (report, result) = prepare::run(&repository, &options)?;
            match mode {
                output::Mode::Human => {
                    print!("{}", prepare::render(&report, &result));
                    Ok(output::exit_code(&report))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "prepare",
                    &report,
                    Some(output::value(&result)),
                )),
            }
        }
        EvidenceCommand::Run {
            alias,
            stage,
            prototype,
        } => {
            let repository = ctx.open_repo()?;
            let report = run_cmd::run(&repository, &alias, &stage, prototype)?;
            Ok(output::finish(mode, "run", &report, None))
        }
        EvidenceCommand::Submit { alias, file } => {
            let repository = ctx.open_repo()?;
            let report = run_cmd::submit(&repository, &alias, &file)?;
            Ok(output::finish(mode, "submit", &report, None))
        }
        EvidenceCommand::Document { command } => match command {
            DocumentCommand::Draft {
                draft_dir,
                output,
                resume,
            } => Ok(document::draft::run(&draft_dir, &output, resume, mode)),
            DocumentCommand::Review { alias } => {
                let repository = ctx.open_repo()?;
                let report = document::review(&repository, alias.as_deref())?;
                Ok(output::finish(mode, "document.review", &report, None))
            }
        },
        EvidenceCommand::Mark {
            alias,
            baseline,
            record,
            verify,
            file,
        } => {
            let repository = ctx.open_repo()?;
            let (report, evaluation) = if verify {
                mark::verify(&repository, &alias, baseline.as_deref(), file.as_deref())?
            } else if record {
                mark::record(&repository, &alias, baseline.as_deref())?
            } else {
                mark::evaluate(&repository, &alias, baseline.as_deref())?
            };
            if !matches!(mode, output::Mode::Json)
                && !verify
                && let Some(m) = &evaluation.mark
            {
                print!("{}", String::from_utf8_lossy(&m.to_bytes()));
            }
            Ok(output::finish(
                mode,
                "mark",
                &report,
                Some(output::value(&evaluation)),
            ))
        }
        EvidenceCommand::Perform {
            alias,
            stage,
            all,
            prototype,
        } => {
            let repository = ctx.open_repo()?;
            let report = match (alias.as_deref(), stage.as_deref(), all) {
                (Some(a), Some(st), false) => perform::run(&repository, a, st, prototype)?,
                (None, None, true) => perform::all(&repository, prototype)?,
                _ => {
                    return Err(Box::new(repo::RepoError::Message(
                        "war evidence perform: name a Warrant and a stage, or pass --all"
                            .to_owned(),
                    )));
                }
            };
            Ok(output::finish(mode, "perform", &report, None))
        }
        EvidenceCommand::Eval { command } => {
            let repository = ctx.open_repo()?;
            match command {
                EvalCommand::Run {
                    task,
                    drafter,
                    verifier,
                    out,
                    keep,
                    tasks_dir,
                } => {
                    let opts = eval::Options {
                        tasks_dir,
                        only: task,
                        drafter,
                        verifier,
                        out,
                        keep,
                    };
                    let (report, result, path) = eval::run(&repository, &opts)?;
                    let mut value = output::value(&result);
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "result_path".to_owned(),
                            serde_json::Value::String(repository.relative(&path)),
                        );
                    }
                    Ok(output::finish(mode, "eval.run", &report, Some(value)))
                }
                EvalCommand::Verify { result, baseline } => {
                    let report = eval::verify(&repository, &result, &baseline)?;
                    Ok(output::finish(mode, "eval.verify", &report, None))
                }
                EvalCommand::Ordinary {
                    agent,
                    scenario,
                    timeout_secs,
                    keep,
                } => {
                    let opts = eval_ordinary::Options {
                        scenario,
                        agent,
                        timeout_secs,
                        keep,
                    };
                    let (report, result) = eval_ordinary::run(&repository, &opts)?;
                    Ok(output::finish(mode, "eval.ordinary", &report, Some(result)))
                }
            }
        }

        // OW-WAR-0148 M13: an item's independent verification, the same seam
        // at the size of one tick.
        EvidenceCommand::Verify {
            alias, response, ..
        } if ticket::is_ticket_ref(&alias) => {
            let (_, store) = ctx.tickets(None)?;
            let outcome = match response {
                Some(path) => ticket::acts::verify_ingest(&store, &alias, &path)?,
                None => ticket::acts::verify_request(&store, &alias)?,
            };
            Ok(ticket_answer(mode, "verify", &outcome))
        }
        EvidenceCommand::Verify {
            alias,
            performer,
            response,
            bundle,
            run,
        } => {
            let repository = ctx.open_repo()?;
            if bundle {
                let (report, index) = bundle::emit(&repository, &alias, &performer)?;
                return Ok(output::finish(mode, "verify.bundle", &report, Some(index)));
            }
            if run {
                let report = bundle::run(&repository, &alias, &performer)?;
                return Ok(output::finish(mode, "verify.run", &report, None));
            }
            match response {
                // Ingest verdicts something else produced.
                Some(path) => {
                    let report = verify::ingest(&repository, &alias, &path)?;
                    Ok(output::finish(mode, "verify", &report, None))
                }
                // Emit the request and stop. §75.2: a seam with nothing on the
                // other side should say so rather than pretend.
                None => {
                    let request = verify::request(&repository, &alias, &performer)?;
                    output::emit(
                        mode,
                        "verify.request",
                        &toml::to_string_pretty(&request).map_err(|e| repo::RepoError::Io {
                            context: "could not render the verification request".to_owned(),
                            source: std::io::Error::other(e.to_string()),
                        })?,
                        output::value(&request),
                    );
                    eprintln!(
                        "# {} obligation(s) for {alias} at {} assurance.",
                        request.obligations.len(),
                        request.assurance_level
                    );
                    eprintln!(
                        "# Hand this to an INDEPENDENT verifier, then ingest with:\n\
                         #   war evidence verify {alias} --response <file>"
                    );
                    Ok(EXIT_OK)
                }
            }
        }
    }
}

fn run_view(ctx: &Ctx, command: ViewGroup) -> Result<u8, Box<dyn std::error::Error>> {
    match command {
        ViewGroup::Member(command) => run_view_member(ctx, command),
        // M12: `war status --timeline` under the name a reader looks for.
        ViewGroup::Timeline => run_daily(
            ctx,
            DailyCommand::Status {
                alias: None,
                timeline: true,
                pending: false,
            },
        ),
    }
}

fn run_view_member(ctx: &Ctx, command: ViewCommand) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        ViewCommand::Ready { actor } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(mode, "ready", &ticket::ready(&store)?))
        }
        ViewCommand::Prime {
            ticket: target,
            actor,
        } => {
            let (repository, store) = ctx.tickets(actor.as_deref())?;
            let mut outcome = ticket::prime(&store, target.as_deref())?;
            // M9: the first thing an agent runs says when the text it
            // follows is newer than this binary (a WARN, on stderr in
            // human mode, in the envelope under --json).
            outcome
                .report
                .diagnostics
                .extend(skew::findings(&repository.root));
            Ok(ticket_answer(mode, "prime", &outcome))
        }
        ViewCommand::Tickets {
            kind,
            labels,
            state,
            text,
            search,
            epic,
        } => {
            let (repository, store) = ctx.tickets(None)?;
            let filter = ticket::Filter {
                kind,
                labels,
                state,
                text,
                search,
                epic,
            };
            // M10: one list, every encoding. The envelope keeps the
            // command name it always had.
            let others = ticket::Others::of(&repository)?;
            Ok(ticket_answer(
                mode,
                "tickets",
                &ticket::list(&store, &others, &filter)?,
            ))
        }
        ViewCommand::Tui { panic_after_setup } => {
            if matches!(mode, output::Mode::Json) {
                return Ok(tui::refuse_json());
            }
            Ok(tui::run(ctx.root.clone(), panic_after_setup)?)
        }
        ViewCommand::Ui {
            command,
            port,
            page,
            actor,
            lan,
            name,
            cert,
            key,
            self_signed,
            pair_answer_secs,
            pair_code_secs,
            device_secs,
        } => {
            let repository = ctx.open_repo()?;
            if let Some(UiCommand::Devices { revoke }) = command {
                return Ok(webui::devices(repository.root, revoke, mode)?);
            }
            let secs = |v: Option<u64>, d: std::time::Duration| {
                v.map_or(d, std::time::Duration::from_secs)
            };
            let lan = lan.map(|addr| webui::LanOptions {
                addr,
                name,
                cert,
                key,
                self_signed,
                answer_ttl: secs(pair_answer_secs, webui::pairing::ANSWER_TTL),
                code_ttl: secs(pair_code_secs, webui::pairing::CODE_TTL),
                device_ttl: secs(device_secs, webui::pairing::DEVICE_TTL),
            });
            Ok(webui::run_with(
                repository.root,
                port,
                &page,
                actor,
                lan,
                mode,
            )?)
        }
        ViewCommand::Board { html } => {
            // One-shot reader, like `status`: evaluating the frontier, queue
            // and corpus may ask about the same receipt hundreds of times.
            // Keep one tree observation per basis, never refresh the index.
            gate_cmd::source::remember_tree_reads();
            let repository = ctx.open_repo()?;
            let (report, view) = board::build(&repository)?;
            if !matches!(mode, output::Mode::Json) {
                if html {
                    print!("{}", board::html(&view, &report));
                    return Ok(output::exit_code(&report));
                } else {
                    print!("{}", board::render(&view));
                }
            }
            Ok(output::finish(
                mode,
                "board",
                &report,
                Some(serde_json::to_value(&view)?),
            ))
        }
        ViewCommand::Console => {
            let repository = ctx.open_repo()?;
            // `--json` is a reader, not a screen: a harness asking what a human
            // owes gets the board and no prompt. Signing is a TTY act.
            if matches!(mode, output::Mode::Json) {
                let board = console::board(&repository)?;
                let report = diagnostic::Report::default();
                return Ok(output::finish(
                    mode,
                    "console",
                    &report,
                    Some(serde_json::to_value(&board)?),
                ));
            }
            let report = console::run(&repository)?;
            Ok(output::finish(mode, "console", &report, None))
        }
        ViewCommand::Watch {
            once,
            interval,
            notify_send,
            ticks,
        } => {
            let repository = ctx.open_repo()?;
            if once {
                let snap = watch::snapshot(&repository)?;
                output::emit(
                    mode,
                    "watch",
                    watch::render(&snap).trim_end(),
                    output::value(&snap),
                );
                return Ok(EXIT_OK);
            }
            watch::run(&repository, interval, notify_send, ticks)?;
            Ok(EXIT_OK)
        }
        ViewCommand::Overview {
            all,
            snapshot,
            html,
            serve,
            port,
            refresh_secs,
        } => {
            let repository = ctx.open_repo()?;
            if snapshot {
                // One-shot read-only snapshot. Never enable this memo for the
                // long-running server below: each refresh must observe edits.
                gate_cmd::source::remember_tree_reads();
                let view = progress_viewer::json_snapshot(&repository)?;
                output::emit(
                    mode,
                    "overview",
                    &serde_json::to_string_pretty(&view).unwrap(),
                    view,
                );
                return Ok(EXIT_OK);
            }
            if let Some(path) = html {
                progress_viewer::export(&repository, &path)?;
                output::emit(
                    mode,
                    "overview",
                    &format!("Progress: {}", path.display()),
                    serde_json::json!({"html":path}),
                );
                return Ok(EXIT_OK);
            }
            if serve {
                progress_viewer::serve(repository, port, refresh_secs, mode)?;
                return Ok(EXIT_OK);
            }
            // Plain overview is also one-shot and read-only. Export and serve
            // returned above without freezing their tree observations.
            gate_cmd::source::remember_tree_reads();
            let view = overview::build(status::build(&repository)?, all);
            output::emit(
                mode,
                "overview",
                &overview::render(&view),
                output::value(&view),
            );
            Ok(EXIT_OK)
        }
    }
}

/// `war admin preset [<name>] [--reset-roles]` (OW-WAR-0148 M14).
fn run_preset(
    ctx: &Ctx,
    name: Option<&str>,
    reset_roles: bool,
) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    let repository = ctx.open_repo()?;
    let path = repository.root.join(init::CONFIG_FILE);
    let refused = |rule: &str, why: String| {
        ticket_answer(
            mode,
            "preset",
            &ticket::Outcome::refused(rule, init::CONFIG_FILE, why),
        )
    };
    let Some(name) = name else {
        return Ok(match preset::Policy::read(&repository.root) {
            Ok(policy) => {
                output::emit(mode, "preset", &policy.render(), output::value(&policy));
                EXIT_OK
            }
            Err(e) => refused(preset::CONFIG_RULE, e),
        });
    };
    let to = if name == "none" {
        None
    } else if let Some(p) = preset::Preset::parse(name) {
        Some(p)
    } else {
        return Ok(refused(
            "preset.unknown",
            format!(
                "{name:?} is not a preset: `war admin preset vibe`, `team`, `regulated`, or \
                 `none`; nothing was written"
            ),
        ));
    };
    let text = std::fs::read_to_string(&path).map_err(|source| repo::RepoError::Io {
        context: format!("could not read {path}"),
        source,
    })?;
    let (updated, switched) = match preset::switch(&text, to, reset_roles) {
        Ok(v) => v,
        Err(why) => return Ok(refused(preset::CONFIG_RULE, why)),
    };
    if updated != text {
        std::fs::write(&path, &updated).map_err(|source| repo::RepoError::Io {
            context: format!("could not write {path}"),
            source,
        })?;
    }
    let policy = preset::Policy::from_text(&updated)
        .map_err(|e| repo::RepoError::Message(format!("{}: {e}", preset::CONFIG_RULE)))?;
    let roles = match switched.roles {
        "written" => "its [roles] written".to_owned(),
        "kept" => format!(
            "your [roles] kept (`war admin preset {name} --reset-roles` writes the preset's)"
        ),
        _ => "no [roles] table".to_owned(),
    };
    let human = format!(
        "preset {} -> {}; {roles}\n{}",
        switched.from.map_or("none", preset::Preset::as_str),
        switched.to.map_or("none", preset::Preset::as_str),
        policy.render()
    );
    Ok(ticket_answer(
        mode,
        "preset",
        &ticket::Outcome::ok(
            human,
            serde_json::json!({"switched": switched, "policy": policy}),
        ),
    ))
}

fn run_admin(ctx: &Ctx, command: AdminCommand) -> Result<u8, Box<dyn std::error::Error>> {
    let mode = ctx.mode;
    match command {
        AdminCommand::Bridge {
            command:
                BridgeCommand::ClaudeTasks {
                    dir,
                    file,
                    event,
                    apply,
                    actor,
                },
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            let source = match (dir, file, event) {
                (_, _, Some(e)) => bridge::Source::Event(e),
                (_, Some(f), None) => bridge::Source::File(f),
                (Some(d), None, None) => bridge::Source::Dir(d),
                (None, None, None) => bridge::Source::Default,
            };
            Ok(ticket_answer(
                mode,
                "bridge",
                &bridge::claude_tasks(&store, &source, apply)?,
            ))
        }
        AdminCommand::Release {
            target,
            if_rev,
            actor,
        } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "release",
                &ticket::release(&store, &target, if_rev.as_deref())?,
            ))
        }
        AdminCommand::MergeTicket {
            base,
            ours,
            theirs,
            path,
            install,
            probe,
        } => {
            if probe {
                return Ok(EXIT_OK);
            }
            let outcome = if install {
                let repository = repo::Repository::discover(ctx.root.clone())?;
                ticket::merge_install(&repository.root)
            } else {
                match (base, ours, theirs) {
                    (Some(base), Some(ours), Some(theirs)) => {
                        ticket::merge_ticket(&base, &ours, &theirs, path.as_deref())?
                    }
                    _ => unreachable!("clap requires the three files without --install"),
                }
            };
            if matches!(mode, output::Mode::Human)
                && !outcome.is_refused()
                && outcome.human.is_empty()
            {
                return Ok(EXIT_OK);
            }
            Ok(ticket_answer(mode, "merge-ticket", &outcome))
        }
        AdminCommand::Heartbeat { target, actor } => {
            let (_, store) = ctx.tickets(actor.as_deref())?;
            Ok(ticket_answer(
                mode,
                "heartbeat",
                &ticket::heartbeat(&store, target.as_deref())?,
            ))
        }
        AdminCommand::Sdk { request, output } => Ok(sdk::run(&request, output.as_deref())),
        // A hosted run opens no repository: the request is the basis.
        AdminCommand::Host {
            export: false,
            projections: _,
        } => Ok(host::run_stdin()),
        AdminCommand::Host {
            export: true,
            projections,
        } => Ok(host::run_export(&ctx.open_repo()?, &projections)?),

        AdminCommand::AgentsMd {
            stdout,
            force: _,
            block: true,
            files,
        } => {
            if stdout {
                println!(
                    "{}",
                    openwarrant_core::instruction::block_text(instructions::version())
                );
                return Ok(EXIT_OK);
            }
            let repository = ctx.open_repo()?;
            let root = &repository.root;
            let mut report = diagnostic::Report::default();
            let mut targets = Vec::new();
            for f in &files {
                let p = camino::Utf8Path::new(f);
                if p.is_absolute() || p.components().any(|c| c.as_str() == "..") {
                    report.push(diagnostic::Diagnostic::error(
                        "agents-md.file-outside",
                        f.clone(),
                        format!(
                            "{f}: --file names a path relative to the repository root, inside \
                             it. Nothing was written."
                        ),
                    ));
                } else {
                    targets.push(root.join(p));
                }
            }
            if !report.diagnostics.is_empty() {
                return Ok(output::finish(mode, "agents_md", &report, None));
            }
            if targets.is_empty() {
                targets = instructions::default_targets(root);
            }
            match instructions::write_blocks(root, &targets) {
                Ok(written) => {
                    let human = written
                        .iter()
                        .map(|w| {
                            format!(
                                "{}: {}",
                                w.path,
                                match w.change {
                                    "created" => "created, holding the openwarrant block",
                                    "inserted" => "added the openwarrant block at its end",
                                    "updated" => "updated the openwarrant block",
                                    "linked" => "a link to a file above; written through it",
                                    _ => "the openwarrant block is current; nothing changed",
                                }
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    output::emit(
                        mode,
                        "agents_md",
                        &human,
                        serde_json::json!({
                            "block": true,
                            "version": instructions::version(),
                            "files": written
                                .iter()
                                .map(|w| serde_json::json!({"path": w.path, "change": w.change}))
                                .collect::<Vec<_>>(),
                        }),
                    );
                    Ok(EXIT_OK)
                }
                Err(instructions::Refused::Blocks(report)) => {
                    Ok(output::finish(mode, "agents_md", &report, None))
                }
                Err(instructions::Refused::Io(message)) => Err(message.into()),
            }
        }
        AdminCommand::AgentsMd {
            stdout,
            force,
            block: false,
            files: _,
        } => {
            let repository = ctx.open_repo()?;
            let ns = repository.config.project.namespace.as_str().to_owned();
            if stdout {
                print!("{}", init::render_agents_md(&ns));
                return Ok(EXIT_OK);
            }
            match init::write_agents_md(&repository.root, &ns, force)? {
                true => {
                    output::emit(
                        mode,
                        "agents_md",
                        "wrote AGENTS.md",
                        serde_json::json!({"path": "AGENTS.md", "namespace": ns, "written": true}),
                    );
                    Ok(EXIT_OK)
                }
                false => {
                    let mut report = diagnostic::Report::default();
                    report.push(diagnostic::Diagnostic::error(
                        "agents-md.exists",
                        "AGENTS.md".to_owned(),
                        "AGENTS.md exists; pass --force to replace it, or --stdout to read the template",
                    ));
                    output::finish(mode, "agents_md", &report, None);
                    Ok(EXIT_DIAGNOSTIC)
                }
            }
        }

        AdminCommand::Migrate {
            corpus,
            commit,
            out,
            verify,
            attempt_promotion,
        } => {
            let artifact = migrate::import(&corpus, &commit, attempt_promotion)?;
            migrate::write_or_verify(&artifact, &out, verify)?;
            migrate::print(&artifact);
            // OBL-002 and OBL-003 are countable, so they decide the exit code.
            // §96.3's other half — that a gate cannot arrive qualified — is a type
            // invariant rather than a count, and has no way to be violated here.
            Ok(if migrate::obligations_met(&artifact) {
                EXIT_OK
            } else {
                EXIT_NOT_READY
            })
        }
        AdminCommand::Bonsai { command } => match command {
            BonsaiCommand::Check {
                warrant,
                base,
                head,
                bonsai: binary,
            } => {
                let repository = ctx.open_repo()?;
                let evidence = bonsai::check(&repository, &warrant, &base, &head, &binary)?;
                println!("{}", serde_json::to_string_pretty(&evidence)?);
                Ok(match evidence.verdict {
                    bonsai::EvidenceVerdict::Pass => EXIT_OK,
                    bonsai::EvidenceVerdict::Unknown => EXIT_NOT_READY,
                    bonsai::EvidenceVerdict::Fail => EXIT_DIAGNOSTIC,
                })
            }
            BonsaiCommand::VerifyEvidence { evidence } => {
                let repository = ctx.open_repo()?;
                bonsai::verify_evidence_file(&repository, &evidence)?;
                println!("Bonsai evidence is a valid passing v1 document");
                Ok(EXIT_OK)
            }
        },
        AdminCommand::Archive { cmd } => {
            let (human, result) = preservation::run(ctx.root.clone(), cmd)?;
            output::emit(mode, "archive", &human, result);
            Ok(EXIT_OK)
        }
        // M10: `war export beads`. An alias is `NS-WAR-NNNN`, so the word can
        // name nothing else.
        AdminCommand::Export {
            alias: Some(format),
            force,
            round_trip,
            reconnect,
            progress: None,
            verify_progress: None,
        } if format == "beads" => {
            if force || round_trip || reconnect {
                let mut report = diagnostic::Report::default();
                report.push(diagnostic::Diagnostic::error(
                    "export.beads-flags",
                    String::new(),
                    "--force, --round-trip and --reconnect are the §68 package's; \
                     `war admin export beads` takes none of them",
                ));
                return Ok(output::finish(mode, "export", &report, None));
            }
            let (_, store) = ctx.tickets(None)?;
            let (jsonl, warnings) = interop::beads::export(&store)?;
            let mut report = diagnostic::Report::default();
            for w in warnings {
                report.push(w);
            }
            match mode {
                output::Mode::Human => {
                    for d in &report.diagnostics {
                        eprintln!("{d}");
                    }
                    print!("{jsonl}");
                    Ok(output::exit_code(&report))
                }
                output::Mode::Json => Ok(output::finish(
                    mode,
                    "export",
                    &report,
                    Some(serde_json::json!({
                        "schema": "oh.war/export-beads/v1",
                        "issues": jsonl.lines().count(),
                        "jsonl": jsonl,
                    })),
                )),
            }
        }
        AdminCommand::Export {
            alias,
            force,
            round_trip,
            reconnect,
            progress,
            verify_progress,
        } => {
            if let Some(dir) = verify_progress {
                let report = progress::verify(&dir)?;
                return Ok(output::finish(
                    mode,
                    "export.progress.verify",
                    &report,
                    None,
                ));
            }
            let repository = ctx.open_repo()?;
            if let Some(dir) = progress {
                let report = progress::export(&repository, &dir)?;
                return Ok(output::finish(mode, "export.progress", &report, None));
            }
            let Some(alias) = alias else {
                // clap: `alias` is required unless a progress flag was given,
                // and both of those returned above.
                return Err(Box::new(repo::RepoError::Message(
                    "war admin export: a Warrant alias is required".to_owned(),
                )));
            };
            if round_trip {
                let rt = export::round_trip(&repository, &alias, reconnect)?;
                match rt.verify() {
                    Ok(()) => {
                        println!(
                            "{alias}: §68.3 round trip verified ({})",
                            rt.original_digest
                        );
                        return Ok(EXIT_OK);
                    }
                    Err(e) => {
                        return Err(Box::new(repo::RepoError::Message(format!("{alias}: {e}"))));
                    }
                }
            }
            let (package, missing) = export::assemble(&repository, &alias)?;
            match package.validate() {
                Ok(()) => {
                    println!(
                        "{alias}: §68.2 export complete ({} record(s))",
                        package.embedded_record_count
                    );
                    Ok(EXIT_OK)
                }
                Err(e) => {
                    println!(
                        "{alias}: §68.2 export INCOMPLETE — {} of {} required contents absent:",
                        missing.len(),
                        openwarrant_core::journal::EXPORT_CONTENTS.len() - 1
                    );
                    for m in &missing {
                        println!("  · {m}");
                    }
                    if force {
                        println!(
                            "\n--force: the package would be written anyway. It is NOT a valid \
                             §68 export and is not reported as one."
                        );
                        return Ok(EXIT_NOT_READY);
                    }
                    Err(Box::new(repo::RepoError::Message(format!("{alias}: {e}"))))
                }
            }
        }
        AdminCommand::Import {
            format,
            path,
            actor,
        } => {
            let (repository, store) = ctx.tickets(actor.as_deref())?;
            let path = interop::resolve(&repository, &path);
            let read = match format.as_str() {
                "beads" => interop::beads::read(&repository, &store, &path),
                "openspec" => interop::openspec::import(&repository, &store, &path),
                "speckit" => interop::speckit::import(&repository, &store, &path),
                other => {
                    return Ok(ticket_answer(
                        mode,
                        "import",
                        &interop::write::refused(&[interop::Fault::new(
                            "import.format-unknown",
                            "",
                            0,
                            format!(
                                "{other:?} is not a format this build imports: beads, \
                                 openspec, speckit"
                            ),
                        )]),
                    ));
                }
            };
            let outcome = match read {
                Ok(import) => interop::write::apply(&repository, &store, &import)?,
                Err(faults) => interop::write::refused(&faults),
            };
            Ok(ticket_answer(mode, "import", &outcome))
        }

        AdminCommand::Kf { cmd } => match cmd {
            KfCommand::Health { base } => {
                let client = kf::Client::new(&base)?;
                println!("{}", client.health()?);
                Ok(EXIT_OK)
            }
            KfCommand::Act {
                base,
                action_type,
                actor,
                acting_role,
                organization,
                reason,
                idempotency_key,
                target_ids,
                payload,
                confirm_write,
            } => {
                if !confirm_write {
                    return Err(Box::new(repo::RepoError::Message(
                        "refusing to POST a §67 action without --confirm-write. This writes \
                         to an authoritative external record, and a seam that is easy to \
                         reach by accident is how a diagnostic becomes a fabrication."
                            .to_owned(),
                    )));
                }
                let client = kf::Client::new(&base)?;
                let body = client.post_action(
                    &action_type,
                    &kf::Actor {
                        actor,
                        acting_role,
                        organization,
                    },
                    &kf::ActionEnvelope {
                        target_ids,
                        payload: serde_json::from_str(&payload).map_err(|e| {
                            repo::RepoError::Message(format!("--payload is not JSON: {e}"))
                        })?,
                        reason,
                        idempotency_key,
                    },
                )?;
                println!("{body}");
                Ok(EXIT_OK)
            }
        },

        AdminCommand::Telemetry {
            commit,
            out,
            verify,
            attach,
            warrant,
            reviewer,
        } => {
            let repository = ctx.open_repo()?;
            if let Some(scope) = attach {
                let human = telemetry::attach(&scope, &warrant, &reviewer)?;
                output::emit(
                    mode,
                    "telemetry",
                    &human,
                    serde_json::json!({
                        "operation": "attach",
                        "scope": scope,
                        "warrant": warrant,
                        "reviewer": reviewer,
                        "message": human,
                    }),
                );
                return Ok(EXIT_OK);
            }
            let baseline = telemetry::take(&repository, &commit)?;
            let rendered = telemetry::render(&baseline)?;
            if verify {
                let existing = std::fs::read_to_string(&out)
                    .map_err(|e| repo::RepoError::Message(format!("cannot read {out}: {e}")))?;
                // Compared EXACTLY. Trimming would let whitespace drift through
                // while the doc claims byte-for-byte agreement.
                if existing == rendered {
                    let human = format!(
                        "telemetry baseline at {commit} is unchanged (untracked work read from {})",
                        telemetry::history_read(&baseline)
                    );
                    output::emit(
                        mode,
                        "telemetry",
                        &human,
                        serde_json::json!({
                            "operation": "verify",
                            "commit": commit,
                            "path": out.as_str(),
                            "unchanged": true,
                        }),
                    );
                    return Ok(EXIT_OK);
                }
                return Err(Box::new(repo::RepoError::Message(format!(
                    "the committed baseline differs from a fresh one at {commit}. A baseline \
                     is a record of one moment; if the corpus moved, take a NEW one rather \
                     than overwriting the old."
                ))));
            }
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    repo::RepoError::Message(format!("cannot create {parent}: {e}"))
                })?;
            }
            std::fs::write(&out, &rendered)
                .map_err(|e| repo::RepoError::Message(format!("cannot write {out}: {e}")))?;
            let untaken = baseline
                .measures
                .values()
                .filter(|m| matches!(m, telemetry::Measure::NotYet { .. }))
                .count();
            let human = format!(
                "telemetry baseline written to {out}\n  {} of {} §94 measures taken; {untaken} \
                 recorded `not_measurable_yet` with a reason\n  {} §95 untracked-work \
                 candidate(s), read from {}\n  {} §100 metrics, every one `no baseline` — one measurement \
                 supports no delta",
                baseline.measures.len() - untaken,
                baseline.measures.len(),
                baseline.untracked_work_candidates.len(),
                telemetry::history_read(&baseline),
                baseline.success_metrics.len()
            );
            output::emit(
                mode,
                "telemetry",
                &human,
                serde_json::json!({
                    "operation": "record",
                    "path": out.as_str(),
                    "baseline": baseline,
                }),
            );
            Ok(EXIT_OK)
        }
        AdminCommand::Blut {
            alias,
            verify,
            emit,
        } => {
            let repository = ctx.open_repo()?;
            let report = blut::lower(&repository, &alias, verify.as_deref(), emit.as_deref())?;
            Ok(output::finish(mode, "blut", &report, None))
        }
        AdminCommand::DispatchBundle { command } => {
            let (report, result) = dispatch_bundle_cmd::run(ctx.root.clone(), command)?;
            Ok(output::finish(
                mode,
                "dispatch-bundle",
                &report,
                Some(result),
            ))
        }
        AdminCommand::Dispatch {
            alias,
            stage,
            attempt_kind,
            prior_failure,
            emit,
            emit_context,
            prototype,
        } => {
            let repository = ctx.open_repo()?;
            let kind = attempt_kind
                .parse::<openwarrant_core::execution::AttemptKind>()
                .map_err(|e| repo::RepoError::Message(e.to_string()))?;
            let report = dispatch::run(
                &repository,
                &alias,
                &stage,
                dispatch::Options {
                    attempt_kind: kind,
                    prior_failure_evidence: &prior_failure,
                    emit_to: emit.as_deref(),
                    emit_context_to: emit_context.as_deref(),
                    prototype,
                },
            )?;
            // The packet is the only thing on stdout when it goes there. An
            // actor piping `war dispatch` into a parser must get canonical JSON
            // (oh.war/stage-dispatch/v1, its own schema) and nothing else — so
            // the report goes to STDERR in that case: human lines in human
            // mode, one envelope in --json mode. The packet's printer lives in
            // dispatch.rs, a pinned deliverable of resolved OW-WAR-0056; moving
            // the packet inside the envelope is a correction for another day.
            if emit.is_none() {
                match mode {
                    output::Mode::Human => {
                        for d in &report.diagnostics {
                            eprintln!("{d}");
                        }
                    }
                    output::Mode::Json => {
                        eprintln!("{}", output::envelope("dispatch", &report, None));
                    }
                }
                Ok(output::exit_code(&report))
            } else {
                // With --emit the packet is on disk, so the report may take
                // stdout — as an envelope under --json.
                Ok(output::finish(mode, "dispatch", &report, None))
            }
        }
        AdminCommand::Journal { alias, backfill } => {
            let repository = ctx.open_repo()?;
            if backfill {
                let report = journal_cmd::backfill(&repository, &alias)?;
                Ok(output::finish(mode, "journal", &report, None))
            } else {
                let text = journal_cmd::show(&repository, &alias)?;
                match mode {
                    output::Mode::Human => print!("{text}"),
                    output::Mode::Json => output::emit(
                        mode,
                        "journal",
                        &text,
                        serde_json::json!({"alias": alias, "rendered": text}),
                    ),
                }
                Ok(EXIT_OK)
            }
        }
        AdminCommand::Projects { add, forget } => {
            let (report, rows) = projects::run(add.as_deref(), forget.as_deref());
            if matches!(mode, output::Mode::Human) {
                print!("{}", projects::render(&rows));
            }
            Ok(output::finish(
                mode,
                "projects",
                &report,
                Some(serde_json::json!({ "projects": rows })),
            ))
        }
        AdminCommand::Pins {
            refresh,
            alias,
            resolved_only,
            ..
        } if refresh => {
            let repository = ctx.open_repo()?;
            let report = pins::refresh(&repository, alias.as_deref())?;
            let _ = resolved_only;
            Ok(output::finish(mode, "pins", &report, None))
        }
        AdminCommand::Pins {
            candidate: Some(candidate),
            base,
            resolved_only,
            ..
        } => {
            let repository = ctx.open_repo()?;
            let (report, result) =
                pins::candidate(&repository, &candidate, base.as_deref(), resolved_only)?;
            Ok(output::finish(mode, "pins", &report, Some(result)))
        }
        AdminCommand::Pins {
            history: Some(path),
            ..
        } => {
            let repository = ctx.open_repo()?;
            let text = pins::history(&repository, &path)?;
            output::emit(
                mode,
                "pins",
                text.trim_end(),
                serde_json::json!({ "schema": "oh.war/pins-history/v1", "path": path, "rendered": text }),
            );
            Ok(EXIT_OK)
        }
        AdminCommand::Pins { resolved_only, .. } => {
            let repository = ctx.open_repo()?;
            let pins = pins::list(&repository, resolved_only)?;
            output::emit(
                mode,
                "pins",
                pins::render(&pins).trim_end(),
                output::value(&pins),
            );
            Ok(EXIT_OK)
        }
        AdminCommand::Deliver {
            alias,
            ids,
            producer,
            method,
            dry_run,
        } => {
            let repository = ctx.open_repo()?;
            let report = deliver::run(
                &repository,
                &alias,
                &deliver::Options {
                    ids: &ids,
                    producer: producer.as_deref(),
                    method: method.as_deref(),
                    dry_run,
                },
            )?;
            Ok(output::finish(mode, "deliver", &report, None))
        }
        AdminCommand::Commit { write } => {
            let repository = ctx.open_repo()?;
            let (report, result) = commit::run(&repository, write, mode)?;
            Ok(output::finish(mode, "commit", &report, result))
        }
        #[cfg(feature = "schema")]
        AdminCommand::Schemas { check } => {
            let repository = ctx.open_repo()?;
            let report = schemas::run(&repository, check)?;
            Ok(output::finish(mode, "schemas", &report, None))
        }
        AdminCommand::Mcp { describe } => {
            // Discover BEFORE any runtime exists: outside a repository this
            // is an ordinary CLI refusal, never a half-started server.
            let repository = ctx.open_repo()?;
            if describe {
                print!("{}", mcp::describe(repository));
                return Ok(EXIT_OK);
            }
            mcp::run(repository)?;
            Ok(EXIT_OK)
        }
        AdminCommand::Preflight { alias } => {
            let repository = ctx.open_repo()?;
            let (result, report) = preflight_cmd::run(&repository, &alias)?;
            match mode {
                output::Mode::Human => print!("{}", preflight_cmd::render(&result)),
                output::Mode::Json => println!(
                    "{}",
                    output::envelope("preflight", &report, Some(output::value(&result)))
                ),
            }
            Ok(output::exit_code(&report))
        }
        AdminCommand::Diff { alias, from, to } => {
            let repository = ctx.open_repo()?;
            let report = if let Some(to) = to {
                diff_target::compare(&repository, &alias, from.as_deref(), &to)?
            } else {
                if from
                    .as_ref()
                    .is_some_and(|value| value.as_str().starts_with("contract:"))
                {
                    return Err(repo::RepoError::Message(
                        "contract baseline requires explicit --to target".into(),
                    )
                    .into());
                }
                show::diff(&repository, &alias, from.as_ref())?
            };
            // A diff is information, not a verdict: exit 0 whatever it found.
            let _ = output::finish(mode, "diff", &report, None);
            Ok(EXIT_OK)
        }
        AdminCommand::Version { probe: Some(dir) } => {
            // The facts `build.rs` would read from `dir`, then the verdict:
            // one JSON object, non-zero when a release build would be refused.
            let facts = build_identity::gather(dir.as_std_path(), env!("CARGO_PKG_VERSION"));
            let verdict = build_identity::classify(&facts);
            let facts_json = serde_json::json!({
                "version": facts.version,
                "tag": facts.tag,
                "git": facts.git.as_ref().map(|g| serde_json::json!({"commit": g.commit, "dirty": g.dirty})),
                "cargo_vcs_info": facts.vcs_info.as_ref().map(|v| serde_json::json!({"sha1": v.sha1, "dirty": v.dirty})),
            });
            let out = match &verdict {
                Ok(id) => serde_json::json!({
                    "facts": facts_json,
                    "class": id.class.as_str(),
                    "commit": id.commit,
                    "dirty": id.dirty,
                    "line": format!("war {}", id.version_text(&facts.version, build_identity::embedded_profile() == "debug")),
                }),
                Err(why) => serde_json::json!({"facts": facts_json, "refused": why}),
            };
            println!("{}", serde_json::to_string_pretty(&out)?);
            Ok(if verdict.is_ok() {
                EXIT_OK
            } else {
                EXIT_DIAGNOSTIC
            })
        }
        AdminCommand::Version { probe: None } => {
            let install = install::observe();
            let report = install.report();
            Ok(output::finish(
                mode,
                "version",
                &report,
                Some(install.json()),
            ))
        }
        AdminCommand::Update {
            check,
            preview,
            to,
            force,
        } => {
            // The running build's channel by default (OW-WAR-0143): every
            // release so far is a prerelease, and a stable default found none.
            let channel = if preview || to.is_some() {
                install::Channel::Preview
            } else {
                install::default_channel()
            };
            let report = if check {
                install::check(channel)
            } else {
                install::update(channel, to.as_deref(), force)
            };
            Ok(output::finish(mode, "update", &report, None))
        }
        AdminCommand::Doctor {
            fix_signing: true, ..
        } => {
            // The terminal gate comes before the repository is read, so a
            // pipe or an agent's shell learns one thing and nothing moves.
            if !sign::at_a_terminal() || ctx.json {
                let mut report = diagnostic::Report::default();
                report.push(diagnostic::Diagnostic::new(
                    diagnostic::Severity::Error,
                    "doctor.fix-needs-tty",
                    None,
                    "`war admin doctor --fix-signing` asks questions, so it runs only at a terminal; \
                     `war admin doctor` alone reports the same findings without asking"
                        .to_owned(),
                ));
                return Ok(output::finish(mode, "doctor", &report, None));
            }
            let repository = ctx.open_repo()?;
            let report = signing_probe::fix(&repository)?;
            Ok(output::finish(mode, "doctor", &report, None))
        }
        AdminCommand::Doctor {
            alias,
            generated,
            fix_signing: false,
        } => {
            let (report, result) = doctor::run(ctx.root.clone(), alias.as_deref(), generated);
            Ok(output::finish(mode, "doctor", &report, Some(result)))
        }
        AdminCommand::Renumber { alias, new } => {
            let repository = ctx.open_repo()?;
            let done = alias::renumber(&repository, &alias, &new)?;
            match mode {
                output::Mode::Human => {
                    for d in &done.report.diagnostics {
                        if d.severity == diagnostic::Severity::Error {
                            eprintln!("refused ({}): {}", d.rule, d.message);
                        }
                    }
                    if done.report.is_ready() {
                        println!("{}", done.human);
                    }
                }
                output::Mode::Json => println!(
                    "{}",
                    output::envelope("renumber", &done.report, Some(done.result.clone()))
                ),
            }
            Ok(output::exit_code(&done.report))
        }

        AdminCommand::Compile { alias } => {
            let repository = ctx.open_repo()?;
            // Compile writes projections only, which the tree rule excludes:
            // it runs no gate and writes no source, so its reads of the tree
            // and the authority records hold for the whole run (t-eca6,
            // t-f815).
            gate_cmd::source::remember_tree_reads();
            let summary = compile::run(&repository, alias.as_deref(), mode)?;
            if mode == output::Mode::Json {
                output::emit(
                    mode,
                    "compile",
                    "",
                    serde_json::json!({"alias": alias, "written": summary.written, "skipped": summary.skipped}),
                );
            }
            Ok(EXIT_OK)
        }
    }
}

#[cfg(test)]
mod surface_tests {
    use super::*;

    /// The tree on an 8 MiB stack, as the binary builds it (main.rs says why).
    fn tree() -> clap::Command {
        std::thread::Builder::new()
            .stack_size(8 << 20)
            .spawn(command)
            .expect("spawn")
            .join()
            .expect("the command tree builds")
    }

    /// `GROUP_MEMBERS` is what each group's help lists (with aliases), and
    /// `DAILY` is exactly what the top level shows: the lists code reads a
    /// suggested command by cannot drift from the parser.
    #[test]
    fn the_member_lists_are_the_clap_tree() {
        let cmd = tree();
        for (group, members) in GROUP_MEMBERS {
            let g = cmd.find_subcommand(group).expect("a group");
            let mut listed: Vec<String> = g
                .get_subcommands()
                .filter(|s| !s.is_hide_set() && s.get_name() != "help")
                .flat_map(|s| {
                    std::iter::once(s.get_name().to_owned())
                        .chain(s.get_visible_aliases().map(str::to_owned))
                })
                .collect();
            let mut want: Vec<String> = members.iter().map(|m| (*m).to_owned()).collect();
            // `schemas` exists only in a build with the `schema` feature.
            if cfg!(not(feature = "schema")) {
                want.retain(|m| m != "schemas");
            }
            listed.sort();
            want.sort();
            assert_eq!(listed, want, "war {group}");
        }
        let shown: Vec<&str> = cmd
            .get_subcommands()
            .filter(|s| !s.is_hide_set())
            .map(clap::Command::get_name)
            .collect();
        assert_eq!(shown, DAILY, "the top level lists the daily verbs alone");
        assert!(DAILY.len() <= 12);
        // Refusal side: a hidden spelling is still a subcommand, and a word
        // that names nothing is not a member of any group.
        assert!(
            cmd.find_subcommand("board")
                .is_some_and(clap::Command::is_hide_set)
        );
        assert!(!group_member("view", "compile") && !group_member("sign", "OW-WAR-0001"));
    }
}
