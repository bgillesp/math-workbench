---
status: implemented
---

# Repository Structure and Development Conventions

> Design doc for the repository bootstrap,
> [#1](https://github.com/bgillesp/math-workbench/issues/1). It records the conventions as
> planned at the start of development. The parts that stay current (layout, workflow, rules) are
> maintained in [`AGENTS.md`](../../AGENTS.md), [`docs/README.md`](../README.md), and
> [`docs/guides/`](../guides/), which take precedence where they differ from this doc.

Math Workbench is a Rust desktop application for working with mathematical objects in a dynamic
and graphical environment.  The repository is designed to support efficient development using
modern AI coding agents -- enforcing industry best-practices for small-to-medium scale projects,
and avoiding deterioration of the codebase into unmaintainable "AI Slop".  The main techniques
used to achieve this are the inclusion of structured and legible design docs contained within
the repository, tooling-enforced code quality checks, test-driven development, and, especially,
good old fashioned human elbow grease in the planning, design, and review phases of development.

This document outlines in reasonable detail the repository structure and development conventions
we will start the first rounds of development with.  Conventions outlined here are subject to
change as the codebase and available coding tools evolve.

## Overview

Work is tracked as GitHub Issues and prioritized in GitHub Projects. Each issue is implemented on
a short-lived branch and squash-merged into `main` through a pull request. Much of the
implementation is done with coding agents. Several sessions can run in parallel on separate parts
of the codebase, each in its own git worktree. The conventions in this document exist so that this
pace of change doesn't erode the codebase's structure or its institutional knowledge. They rest
on three things.

**Human review where understanding is built.** A human developer takes part at fixed points in
the work, not only at the end. For a major change, a design doc is drafted from the issue
and accepted before implementation begins. The failing tests derived from its acceptance criteria
are then reviewed before any implementation code is written. Together, the design doc and tests
are the specification, and the implementation must satisfy the tests without weakening them.
Architecture decisions take effect only once a human developer accepts them, and every change is
reviewed in its pull request after CI and an AI review pass. Smaller changes need less ceremony:
a bug fix needs tests first but no design doc, and a typo needs neither. The review points are
placed so that a human developer's understanding of the system keeps pace with the code.

**Context that stays legible to people and agents.** Institutional knowledge lives in the
repository as plain Markdown, and each kind of fact has a single home. Architecture decision
records (ADRs) capture significant decisions and the alternatives that were weighed. An ADR is
never edited once accepted; a changed decision gets a new ADR that supersedes the old one.
Design docs record each major change as it was planned, and are frozen once it ships. The
architecture overview, crate-level docs, and glossary describe the system as it currently stands,
and are updated in the same pull request as the code. Working notes stay on their branch and are
moved into these homes before merge. A short `AGENTS.md` maps all of this, so an agent loads only
what a task needs. Someone reading the history years later finds the same record. When code and
documentation disagree, agents report the mismatch instead of deciding which one is right.

**Rules that tools enforce.** Anything that matters is checked mechanically, because conventions
written only in prose drift from one session to the next. A single command, `just ci`, runs
formatting, strict lints, tests, documentation builds, and dependency auditing. It also checks
that each crate depends only on crates at strictly lower levels of the architecture, and that the
docs are internally consistent. CI runs the same command. Agent-specific skills and hooks, for
Claude Code and Codex, are conveniences on top of these checks and never the only enforcement.

Most of this is standard practice for a well-run Rust project: a Cargo workspace, trunk-based
development with squash merges and Conventional Commits, conventional ADRs, RFC-style design docs,
test-driven development, and CI with dependency auditing. What differs is emphasis. Review starts
earlier, at the specification, because implementation is fast and cheap. Knowledge that a team
would usually carry informally is written down and indexed, because agents begin each session
without memory of the last. And the checks are stricter than usual, down to lint levels many teams
find too noisy, because agents resolve such warnings at little cost.

## Process

### Planning and issues

- **GitHub Projects** is the planning layer: roadmap, milestones, priorities, and the status
  board. It's maintained by hand or in a dedicated planning session. Agents working on issues
  read it but don't change it.
- **GitHub Issues** hold work items:
  - Labels: `kind/{bug,feature,chore,question}`, `area/<crate>`, `needs-triage`.
  - Status lives in the Project's Status field rather than in labels, so there's one source for
    it.
- **What agents may do with issues:** read them, comment on progress, link PRs with
  `Closes #N`, and file `needs-triage` issues for out-of-scope findings.

### Change tiers

| Tier | Examples | Design doc | Tests first |
|---|---|---|---|
| Minor | typo, dependency bump, rename | No | n/a |
| Standard | bug fix, small feature within one crate | No (the issue is enough) | Yes |
| Major | new tile category, new crate, format change, public API change | Yes, accepted before implementation | Yes |

### Feature flow

Each session takes one issue on its own branch, in its own git worktree. For major
changes:

1. **Design.** A design doc (`docs/design/NNNN-*.md`) is drafted from the issue, citing the
   relevant ADRs, and is accepted by a human developer after review. Acceptance criteria must be
   concrete enough to turn into tests.
2. **Tests.** Failing tests are written from the acceptance criteria, as a separate commit, and a
   human developer reviews the test diff. This is the executable spec, and the most valuable
   review in the PR.
3. **Implement** until the tests pass. Agents never weaken, delete, or `#[ignore]` an approved
   test. If a test looks wrong, they stop and flag it.
4. **Document.** Update the architecture overview, glossary, and crate docs. Mark the design doc
   `implemented`. Clear `docs/wip/`.
5. **Merge.** Open a PR with `Closes #N`. It then goes through CI, AI review, and human review,
   and is squash-merged into `main`.

**Definition of done** (in AGENTS.md and the PR template):
- `just ci` passes.
- Public items are documented.
- Tests cover the acceptance criteria.
- Living docs are updated.
- `docs/wip/` is empty.
- The design doc status is set.
- Any new dependency or `unsafe` code is justified in the PR.

### Branch notes (`docs/wip/`)

`docs/wip/` holds a branch's working notes (plan, progress, findings), so a task can continue
across sessions. Before merge, each kind of content moves to its permanent home and the
directory is emptied (enforced by CI):

| Content | Destination |
|---|---|
| Plan and progress log | Deleted; the PR description summarizes the outcome |
| Out-of-scope bug or improvement | New `needs-triage` issue |
| Question needing a decision | The PR, or a `kind/question` issue if it outlives the PR |
| Dead end or rejected approach | Alternatives considered in the design doc |
| Context about specific code | A comment at that code |
| Idea for later | An issue, or a draft item in the Project |
| How the system works | Architecture overview or crate docs |

If content keeps turning up that fits none of these, add a dedicated place for it then.

### Branches, commits, and releases

- **Trunk-based on `main`.** Short-lived branches are squash-merged, with the PR title as the
  commit subject and the description as the body. History stays linear.
- **[Conventional Commits](https://www.conventionalcommits.org/)** for PR titles, scoped by crate:
  `feat(mw-graph): validate wire types on connect`.
- **Releases** are tags on `main` (`vX.Y.Z`) with GitHub Releases, and release notes are
  generated from the commit history. A workflow that builds release binaries comes later.

## Documentation

| Kind | Location | Changes | Sign-off |
|---|---|---|---|
| Vision | `docs/vision.md` | Rarely | Human developer |
| Roadmap, priorities | GitHub Projects + Issues | Continuously | Human developer |
| Decision records (ADRs) | `docs/architecture/decisions/NNNN-*.md` | Never edited once accepted; replaced by a new ADR | Human developer; agents draft only on request |
| Design docs | `docs/design/NNNN-*.md` | Frozen once implemented | Human developer, before implementation |
| Architecture overview | `docs/architecture/overview.md` | Updated in the PR that changes behavior | PR review |
| Glossary, guides | `docs/glossary.md`, `docs/guides/` | Living | PR review |
| API docs | `//!` and `///` in source | Living; `missing_docs` enforces coverage | PR review |
| Scratch | `docs/wip/` | Branch only | None |

### Decision records

Architecture Decision Records (ADRs) follow standard practice. Each records one decision that is
hard to reverse and shapes downstream work, for example the wire type system, the layer model, or
serialization versioning. A question that's still open stays in an issue until it's
decided. A choice that's local or easy to reverse belongs in a feature's design doc. Expect a
handful to a few dozen ADRs over the project's life, not one per PR. They're numbered in sequence
and referred to as `ADR-0003`, which keeps them distinct from design docs (`#42`).

- **Format:** a few paragraphs. Context, Decision, Alternatives considered, Consequences. The
  Consequences section includes known risks, and any conditions under which the decision
  should be revisited.
- **Status:** `proposed`, `accepted`, `rejected`, `superseded by ADR-NNNN`, or `deprecated` (no
  longer applies and has no replacement).
- **Superseding.** A revisited decision gets a new ADR with a `Supersedes: ADR-NNNN` line. The
  new ADR restates whatever still holds, so each area's current decision can be read from one
  document. The old ADR's status changes to `superseded by ADR-MMMM`. Apart from typo and link
  fixes, that is the only edit an accepted ADR ever gets.
- **Index.** `decisions/README.md` lists the ADRs in force, grouped by area, followed by the
  rest. Agents read only the ADRs in force unless they need the history. `cargo xtask check-docs`
  verifies unique numbering, valid statuses, matching supersede links, and a complete index.
- **Ownership.** Agents draft ADRs only when asked.

### Design docs

Each major change gets one design doc, named after its GitHub issue
(`docs/design/0042-typed-wire-validation.md`). Issues and PRs share one number sequence, so names
never collide.

- **Sections:** Context (linking the issue and the ADRs it works under), Goals / Non-goals,
  Design, Alternatives considered, Acceptance criteria, Test plan.
- **Status:** `draft`, `accepted`, `implemented`, or `abandoned`. Once implemented, the doc is
  frozen: a record of what was built and why, given what was known at the time.
- **Changing a decision:** if a feature requires one, a superseding ADR is accepted before
  implementation starts.

### Current-state docs

- **`architecture/overview.md`** is a codemap: one paragraph per crate, plus the invariants that
  span crates. It's the first stop for anyone getting oriented, human or agent.
- **Crate docs (`//!`)** hold per-crate detail, next to the code they describe.
- **`glossary.md`** defines the project's overloaded terms (tile, wire, signal, layer, level,
  frame, type) and its notation conventions.

**Conflict rule.** If code disagrees with an ADR, the overview, or an accepted design doc, the
agent stops and reports the mismatch. It doesn't decide on its own which one is right.

## Codebase and checks

### Repository layout

```
.
├── AGENTS.md                 # agent entry point: map, commands, hard rules (~100–150 lines)
├── CLAUDE.md                 # one line: @AGENTS.md
├── README.md                 # human entry point
├── Cargo.toml                # virtual workspace manifest
├── Cargo.lock                # committed
├── justfile                  # task runner; `just ci` is exactly what CI runs
├── rust-toolchain.toml       # pinned toolchain + components (rustfmt, clippy)
├── rustfmt.toml
├── deny.toml                 # cargo-deny: advisories, licenses, bans, sources
├── .editorconfig / .gitattributes
├── .cargo/config.toml        # `cargo xtask` alias
├── .agents/
│   └── skills/               # canonical skills (Agent Skills format)
├── .claude/
│   ├── settings.json         # permissions + hook wiring (settings.local.json is gitignored)
│   └── skills -> ../.agents/skills
├── .codex/
│   └── hooks.json            # hook wiring
├── .github/
│   ├── workflows/            # ci.yml, audit.yml, extended.yml
│   ├── dependabot.yml
│   ├── pull_request_template.md
│   └── ISSUE_TEMPLATE/       # issue forms (bug, feature), each applying its kind/* label
├── crates/                   # flat: one directory per crate, directory name == crate name
│   └── mw-<name>/
├── xtask/                    # repository checks (dependency levels, docs), run through just
└── docs/
    ├── README.md             # index: what lives where, status conventions, "read X when doing Y"
    ├── vision.md             # why the project exists; rarely changes
    ├── glossary.md           # domain terms and notation
    ├── architecture/         # the current state of the system
    │   ├── overview.md       # codemap and invariants that span crates
    │   └── decisions/        # ADRs: numbered, never edited once accepted; README.md indexes them
    ├── design/               # one design doc per major change; historical
    │   ├── TEMPLATE.md
    │   └── NNNN-slug.md      # NNNN = GitHub issue number
    ├── guides/               # living how-tos: workflow, testing, code conventions
    └── wip/                  # branch-local scratch; only its README remains at merge
```

Development works on Linux, macOS, and Windows. Recipes in the `justfile` stay short calls into
`cargo` or `xtask` so they run in any shell, and anything with real logic goes in `xtask` rather
than a shell script. The `.claude/skills` link is the only symlink in the repository. On Windows it
needs Developer Mode and `core.symlinks=true`. Without them, git checks it out as a plain text
file and Claude Code finds no project skills, though nothing else breaks. `just setup` warns
when this happens.

### Cargo workspace

- **Flat `crates/` layout** with a common prefix (`mw-` as a placeholder). The dependency graph
  carries the hierarchy, so the directories don't need to.
- **Shared settings live in the root manifest.** That covers package metadata (edition 2024,
  `rust-version`, license, `publish = false`), every external dependency version, and the lint
  baseline. Crates inherit them with `key.workspace = true`.
- **Pinned toolchain** in `rust-toolchain.toml`, bumped in its own PR. `rust-version` matches it.
  As an application, the project only needs to build with its pinned toolchain, so there's no
  older minimum supported Rust version (MSRV) to test.
- **Strict dependency levels.** Each crate under `crates/` declares
  `[package.metadata.mw] level = N`. It may depend only on crates at strictly lower levels, and
  `cargo xtask check-levels` enforces this. The workspace can then be drawn top to bottom by
  level. For example, `mw-math` sits at level 0 and `mw-app` at the top. The actual
  decomposition belongs in a crate-structure ADR. "Level" is used rather than "layer", which
  already has a meaning in the domain model.

Lint baseline (CI runs with `-D warnings`):

```toml
[workspace.lints.rust]
unsafe_code = "deny"          # deny, not forbid: an FFI/GPU module can opt out locally, with justification
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
undocumented_unsafe_blocks = "warn"
# curated allows for noisy pedantic lints go here, each with a one-line reason
```

`pedantic` is often considered too noisy, but agents fix lint warnings cheaply, and the stricter
lints raise the floor of generated code.

### Testing

Adopt each technique when the relevant code appears:

- **Unit tests** in-module. **Integration tests** in each crate's `tests/`.
- **Property tests** (`proptest`) for the math library: algebraic laws, round-trips, invariants.
  A property is a compact, reviewable spec that's hard to satisfy by special-casing.
- **Snapshot tests** (`insta`) compare structured output (evaluated graphs, serialized projects,
  exports) against approved copies stored in the repository. Updating a snapshot changes what the
  test expects, so updates are reviewed like test changes, never accepted in bulk.
- **Format compatibility corpus.** Fixture files for every released serialization version, with
  tests proving they still load. This can't be backfilled later.
- **Golden images** compare rendered output against approved reference images, within a
  tolerance. Start using them once rendering stabilizes.
- **Fuzzing** (`cargo-fuzz`) for deserializers and parsers.
- **Benchmarks** (`criterion` or `divan`) for core computations.
- **Mutation testing** (`cargo-mutants`), run periodically, to check that the tests actually
  catch broken code.

Runner: `cargo nextest`, plus `cargo test --doc` for doctests.

### CI

`just ci` runs:
- fmt check
- clippy
- nextest and doctests
- `cargo doc`
- `cargo deny check` for bans, licenses, and sources (advisories run in `audit.yml`)
- the level and docs checks
- the `docs/wip/`-empty check

CI calls the same recipes.

| Workflow | Trigger | Runs |
|---|---|---|
| `ci.yml` | Non-draft PRs to `main`, and pushes to `main` | `just ci` on Linux |
| `audit.yml` | Weekly, and PRs touching `Cargo.lock` | `cargo deny check advisories` (new advisories appear without code changes) |
| `extended.yml` | Weekly, manual, and before releases | Build and tests on macOS and Windows, extended property tests, a short fuzz run, benchmarks, mutation testing |

- **Draft PRs skip CI.** Work in progress stays in draft, and marking a PR ready runs the checks.
- **macOS and Windows run in `extended.yml`**, not on every PR, while iteration is fast. Run it
  manually on branches that touch platform code. If platform breakage starts reaching `main`,
  move those jobs into `ci.yml`.
- **Branch protection on `main`:** required checks, linear history, and branches must be up to
  date before merge. Both branch protection and standard runners are free for public repos.
- **`cargo-deny`** covers RustSec advisories, plus a license allowlist compatible with
  Apache-2.0, duplicate and banned crates, and allowed sources.
- **Dependabot** runs weekly for `cargo` and `github-actions`, with updates grouped.
  Third-party actions are pinned to commit SHAs.

## Agent tooling

The core is tool-neutral, with a thin adapter for each harness. Claude Code is the primary
harness, and Codex is also supported. Anything that must hold is enforced by `just ci`, never only
by a hook or skill. Harness features are conveniences, so keeping two sets of adapters is cheap.

| Concern | Tool-neutral source | Claude Code | Codex |
|---|---|---|---|
| Instructions | `AGENTS.md` | `CLAUDE.md` imports it (`@AGENTS.md`) | Reads it natively |
| Skills | `.agents/skills/<name>/SKILL.md` | `.claude/skills` symlink | Reads `.agents/skills` natively |
| Hooks | `just` recipes | `.claude/settings.json` | `.codex/hooks.json` |
| Permissions | n/a | `.claude/settings.json` allowlist | `.codex/config.toml` as needed |

Starting set:
- **Skills:** `start-feature` (creates the branch, worktree, wip notes, and a draft PR),
  `new-design-doc`, and `pre-pr` (walks through the definition of done). Each is a thin wrapper
  around a guide in `docs/guides/`.
- **Hook:** run `just fmt` after file edits.

### AGENTS.md

Short, written as instructions, and kept current:

1. What the project is (2–3 sentences, with a link to the vision).
2. The working model: where review happens, and when to stop and ask a human developer.
3. Commands: `just ci`, `just test`, `just fmt`, and so on.
4. Repository map: crates and docs, one line each.
5. Change tiers and feature flow, linking `docs/guides/workflow.md`.
6. Hard rules:
   - Don't weaken approved tests.
   - Follow the conflict rule.
   - Don't edit accepted ADRs.
   - Flag any new dependency or `unsafe` code.
   - Don't change Project fields.
   - Keep `docs/wip/` on branches.
7. Context map. For example: "Before changing crate X, read its `lib.rs` docs." "Before changing
   the file format, read the serialization ADR in force."

Use nested `AGENTS.md` files only for crates with unusual local rules, such as unsafe GPU code.

## Getting started

### Bootstrap order

1. Open a bootstrap issue (#1). Accept this doc and move it to
   `docs/design/0001-repository-conventions.md`.
2. **Skeleton:**
   - `rust-toolchain.toml`, workspace `Cargo.toml`, `.gitignore`, `.editorconfig`,
     `rustfmt.toml`, `deny.toml`, `justfile`.
   - `xtask` with the level and docs checks.
   - A `just setup` recipe that installs the cargo tools the checks use (`cargo-nextest`,
     `cargo-deny`, `cargo-insta`).
   - One placeholder crate, so every check has something to run on.
3. **Docs scaffolding:**
   - `docs/README.md`, the design doc and ADR templates, the ADR index, and a starter glossary.
   - `AGENTS.md` and `CLAUDE.md`.
   - PR and issue templates.
   - Workflow and testing guides.
4. **Agent tooling:**
   - `.agents/skills/` (start with `pre-pr`).
   - The `.claude/skills` symlink and `settings.json`. Confirm Claude Code follows the symlink;
     otherwise, symlink each skill separately.
   - `.codex/hooks.json`, then trust it in Codex.
5. **GitHub:**
   - `ci.yml`, `audit.yml`, and Dependabot.
   - `extended.yml` with only the macOS and Windows jobs for now.
   - Labels and the Project board.
   - Squash-only merging, with "Pull request title and description" as the default commit
     message.
   - `gh auth refresh -s project`, so agents can read the Project.
   - Branch protection once the repo is public.
6. **Restructure the existing docs.** Split `architecture.md` into `architecture/overview.md` and
   ADRs for whatever is already decided. One example is platform abstraction: OS resources sit
   behind traits, only backend crates and the app binary depend on windowing, GPU, or audio
   libraries, and native desktop is the only execution target, with web as an export format.
   Turn the exploratory notes and open questions into issues.
7. **Crate-structure ADR and first feature.** Write the ADR, then take the first feature
   end-to-end through the workflow. That will show what this doc gets wrong.
8. Fill in the rest of `extended.yml` once there's enough code for it to mean something.
