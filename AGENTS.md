# AGENTS.md

Math Workbench is a Rust desktop application for working with mathematical objects in a dynamic,
graphical environment: tiles that compute, display, and respond to interaction, connected by
typed wires. It aims for efficient core computations, an ergonomic interface, and
publication-quality visual output.

## Working model

You're working with a human developer, who sets direction, accepts design docs and architecture
decisions, and reviews every PR. Each session works on one GitHub issue, on its own branch in its
own git worktree, and other sessions may be working on other parts of the codebase at the same
time. The [workflow guide](docs/guides/workflow.md) has the full process.

Stop and ask a human developer when:

- the issue or design doc is ambiguous, or the work is growing beyond its scope;
- code disagrees with an ADR, the architecture overview, or an accepted design doc;
- an approved test looks wrong;
- the change seems to need a new architecture decision, a new dependency, or `unsafe` code.

## Commands

Run everything through `just`. `just --list` shows every recipe.

| Command | What it does |
|---|---|
| `just check` | Every CI check except the branch-notes check. Run it while working. |
| `just ci` | Exactly what CI runs. Must pass before a PR is ready for review. |
| `just test` | Tests (`cargo nextest`) and doctests |
| `just lint` | Clippy, with warnings as errors |
| `just fmt` | Format code (a hook also runs this after edits) |
| `just doc` | Build API docs, with warnings as errors |
| `just setup` | Install the cargo tools the checks use |

## Repository map

- `crates/`: the application's crates, one directory each, named like the crate. The
  [architecture overview](docs/architecture/overview.md) describes each one.
- `xtask/`: repository checks for dependency levels, docs consistency, and branch notes.
- `docs/README.md`: what each doc is for, and what to read before different kinds of work.
- `docs/architecture/`: the overview, and the ADRs in `decisions/`.
- `docs/design/`: one design doc per major change, named by issue number.
- `docs/guides/`: workflow and testing.
- `docs/glossary.md`: project terms. Check it before naming things.
- `docs/wip/`: notes for the current branch only.
- `.agents/skills/`: skills for common workflows.

## Change tiers

| Tier | Examples | Process |
|---|---|---|
| Minor | typo, dependency bump, rename | Just the change |
| Standard | bug fix, small feature in one crate | Failing tests first, then the fix |
| Major | new tile category, new crate, format change, public API change | Accepted design doc, then failing tests reviewed by a human developer, then implementation |

## Hard rules

- Never weaken, delete, or `#[ignore]` an approved test, and never accept snapshot updates in
  bulk.
- When code disagrees with an ADR, the architecture overview, or an accepted design doc, report
  the mismatch. Don't decide which one is right.
- Never edit an accepted ADR, and draft new ADRs only when asked.
- Justify any new dependency or `unsafe` code in the PR description.
- Read GitHub Project fields, but don't change them.
- Keep `docs/wip/` notes on your branch, and empty the directory before the PR is ready.
- Don't loosen lints, CI, or repository checks to get a change through.

## Code conventions

- Dependency versions live in the root `Cargo.toml` under `[workspace.dependencies]`, and crates
  refer to them with `name.workspace = true`.
- A new crate goes in `crates/<name>/`, inherits the workspace package settings and lints
  (`[lints] workspace = true`), and declares `[package.metadata.mw] level = N`. It depends only
  on crates at strictly lower levels.
- Every public item has a doc comment, and each crate's `lib.rs` opens with `//!` docs on its
  purpose.
- Clippy runs at `pedantic`. Fix warnings rather than allowing them. A workspace-wide allow goes
  in the root `Cargo.toml` with a one-line reason.
- PR titles follow Conventional Commits, scoped by crate, as in
  `feat(mw-graph): validate wire types on connect`.

## Before you change…

- **A crate:** read its `lib.rs` docs and the ADRs in force for its area
  ([index](docs/architecture/decisions/README.md)).
- **Tests:** read the [testing guide](docs/guides/testing.md).
- **Workflow, CI, or repository checks:** read the [workflow guide](docs/guides/workflow.md) and
  the [conventions design doc](docs/design/0001-repository-conventions.md).

## Definition of done

`just ci` passes, public items are documented, tests cover the acceptance criteria, living docs
are updated, `docs/wip/` is empty, the design doc status is set, and any new dependency or
`unsafe` code is justified in the PR. The `pre-pr` skill walks through it.
