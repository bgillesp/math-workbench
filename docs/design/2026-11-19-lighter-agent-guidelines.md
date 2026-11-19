# Lighter Agent Guidelines

## Why

The repository's guides specify a development workflow in detail: change tiers, a five-step
feature flow, failing tests written from acceptance criteria and approved by a human developer
before implementation, a design doc lifecycle with statuses, and ADR tooling that checks a
supersede graph and a tag index. In practice, the TDD outline produced tests that pinned
unimportant behavior and missed the important part of the code, and approving every test before
implementation meant heavy review on every design iteration, before and after the first test
suite. Polished agent skill packs such as obra/superpowers handle requirements gathering, design,
TDD, and agent hand-offs better than the in-repository procedures did.

So the repository keeps a baseline of structure, style, and usage guidance plus CI, and each
developer's agent setup decides how work gets done, as long as the results follow the guidelines
and pass CI.

## Goals, non-goals, and anti-goals

Goals:

- Agent-facing docs hold only project description, repository usage, light conventions for docs,
  and style guidance.
- Doc structure stays: ADRs, design docs, guides, and `docs/wip/`.
- Programmatic checks shrink to what CI genuinely needs: relative links, and an empty `docs/wip/`
  at merge.
- Everything removed is recorded outside the repository for later review.
- The human developer's own workflow (superpowers, design log, merge procedure) lives in local
  config.

Non-goals:

- Writing the final style guides. They're seeded from existing content, and a human developer
  rewrites them.
- Changing CI, lints, `cargo-deny`, Dependabot, or GitHub settings.

Anti-goals:

- Re-encoding a workflow in the repository under another name, such as a merge checklist that
  quietly requires per-criterion tests or human test approval.
- Making the repository depend on one harness or skill pack. Its guides and agent instructions
  never name superpowers, which appears only in this design doc, as background.
- Losing removed knowledge. Anything cut that might be wanted again is in the summary.

## Constraints

- `just ci` passes after each PR, including the link check on every tracked Markdown file. Anchors
  aren't checked.
- No ADRs exist yet, so ADR changes migrate nothing.
- Docs follow the project's framing: "a human developer", never a solo-developer framing, and
  clear, fairly concise prose.
- Formatting stays enforced by `fmt-check` in CI. Unformatted code shouldn't reach review, where a
  "fix formatting" commit would add a changeset for the reviewer to read.

## Approaches

- **Two PRs, process first (chosen).** PR 1 changes how work is described and drops the shipped
  agent configuration. PR 2 changes ADR and design doc conventions and deletes the
  checks behind them. Each is one idea and passes CI alone, the docs review splits into two halves
  of about 400 lines, and the process half lands first so the next session runs under it.
  `docs/README.md` and the glossary are touched in both.
- **One PR.** Docs and checks never disagree on `main`, but the review is about 800 changed doc
  lines plus about 2,500 lines of deletions.
- **A third PR for the xtask deletion**, ruled out: the current check reads every dated design doc
  name as number 2026, so renaming the existing docs by date fails its duplicate number check, and
  the conventions and the check removal land together.

## Design

### PR 1: process

**`AGENTS.md`.** Keeps the project description, the application overview, the commands, and the
pointers to the guides. The roles paragraph becomes: a human developer sets direction, approves
architecture decisions, and reviews PRs before they merge to `main`; Issues record problems to solve
later, and work doesn't need an issue to start. Removed: sessions addressing a selected Issue,
planning in a GitHub Project, sessions in their own worktrees, reporting mismatches between code and
docs, and never editing an accepted ADR (the architecture and design guide still says it until PR
2). The repository design paragraph drops test-driven development and "explicit detailed guidance on
development procedures". The `.agents/` bullet goes, the `just fmt` description loses "an agent hook
also runs this after edits" and says to run it before committing, and Resources lists the two new
style guides.

**`docs/guides/development.md`** becomes a guide to repository usage:

- Issues: what Issues are for, and the `kind/`, `area/`, and `needs-triage` labels.
- Branches and PRs: a short slug for branch names, with no issue number required; drafts skip
  CI; a PR is marked ready once the merge checklist holds; squash merge, branch protection, and a
  link to the issues-and-PRs guide for titles, descriptions, and commit messages.
- Testing mechanics: where unit, integration, and doc tests live, the command table, nextest
  running each test in its own process, and long-running checks belonging in the extended
  workflow.
- Branch notes: `docs/wip/` holds a branch's working notes, which are removed before merge, as
  `just ci` enforces. The section keeps its `Branch notes` heading, since `check-ready`'s message
  links to that anchor.
- Merge checklist: `just ci` passes; living docs (the architecture overview, glossary, crate docs,
  and guides) are updated as the change requires; a design doc, if there is one, reflects the
  decisions made during development; `docs/wip/` is empty; and any new dependency, including one
  between workspace crates, or `unsafe` code is justified in the PR.
- Releases, unchanged.

Removed, and recorded in the summary: the change tier table, the five-step workflow, tests first
and their human approval, "never weaken, delete, or `#[ignore]` an approved test", the list of
when to request clarification, the branch notes destination table, the agent review pass, and the
agent merge procedure (merge only on request, draft the message and show it first).

**`docs/guides/issues-and-prs.md`**, a new seed, carrying existing wording nearly verbatim:
writing issues (pointing to the bug and feature forms), Conventional Commits PR titles, PR
descriptions following the template, justifying new dependencies and `unsafe` code, `Closes #N`
when a PR resolves an issue, and the squash commit message format.

**`docs/guides/code-and-tests.md`**, a new seed: doc comments that say what an item is for rather
than restating its name (from `pre-pr`), a doc comment on every field and variant of a non-trivial
struct or enum (issue #11), and checking the glossary before naming something new. It also seeds a
test style note from a human developer's earlier feedback: prefer fewer tests, each pinning behavior
whose failure would be a real regression, and avoid pinning exact wording, incidental output, or
choices a later change could reasonably make. The development guide's list of test techniques isn't
carried over, and is recorded in the removed-guidelines summary instead.

**Agent config.** Delete all of it: `.agents/` (the `pre-pr` skill), the `.claude/skills`
symlink, `.claude/settings.json` (the format hook, the permission allowlist, and the attribution
setting), and `.codex/hooks.json` (the format hook), leaving no `.agents/`, `.claude/`, or `.codex/`
directory. The `.gitignore` entry for `.claude/settings.local.json` stays, since developers may
still keep local settings there. In place of the hook, the guidelines say to run `just fmt` before
committing; if that isn't enough to keep formatting commits out of review, other enforcement can
be explored later. `xtask doctor` drops its symlink warning, and its summary in `main.rs` becomes
"warn about missing tools".

**`docs/guides/repository.md`.** The layout loses `.agents/`, `.claude/`, and `.codex/`. The
`doctor` row loses the symlink. The principle that hooks and skills are conveniences on top of the
checks becomes: agent configuration is up to each developer, and `just ci` is the enforcement. "A
new crate is a major change, so it follows an accepted design doc" goes. The
formatting paragraph replaces the hook with "run `just fmt` before committing". The agent tooling
section shrinks to `AGENTS.md` and `CLAUDE.md`, plus a setup note: developers will likely want to
allow `just:*` and the cargo subcommands `build`, `check`, `clippy`, `doc`, `fmt`, `metadata`,
`nextest`, `test`, `tree`, and `xtask` without a prompt, and personal settings go in
`.claude/settings.local.json` or user settings. The note doesn't mention `gh`. It also says the
repository doesn't expect commits or PR descriptions to credit agentic contributions.

**Smaller pieces.**

- `.github/pull_request_template.md` keeps Summary, Review notes, and Dependencies and `unsafe`,
  dropping `Closes #` and the tests-and-docs section.
- `README.md` drops the Windows symlink instructions, since no symlink remains.
- `docs/README.md`'s "Read this before" table points to the new guides and drops tier language.
  The "What lives where" table and Statuses wait for PR 2.
- The glossary drops "Change tier".
- The GitHub Project goes from every doc: `AGENTS.md`, the development guide, `docs/README.md`'s
  "What lives where" table, and the repository guide's GitHub settings. It's a human developer's
  own organization, not something a reader of the repository needs. Historical design docs keep
  their mentions.

**This design doc** lands in PR 1 as `docs/design/2026-11-19-lighter-agent-guidelines.md`. The
current name check already accepts it, since it reads `2026` as a four-digit number, so the only
accommodation is a temporary `status: accepted` front matter line, and no CI code changes. A second
dated design doc before PR 2 would fail the duplicate number check.

Until PR 2, the ADR checks, the design doc name and status checks, the statuses description, and
the design doc template remain. PR 1's description lists these as intermediate states.

### PR 2: ADR and design doc conventions

**Design docs.** Named `docs/design/YYYY-MM-DD-<topic>.md`, dated by the day the doc was first
written, with no template, front matter, or status. A design doc can change freely during
development, and before a PR merges it reflects the decisions made. A design whose work isn't
finished opens with a short note on what's happening, removed once the work is done. After that the
doc stops changing, and a later change to the same area gets a new dated doc. The existing docs
are renamed to the dates they first landed, with their `status` front matter removed and their text
otherwise left as historical record, apart from fixing links the renames break:

| Now | After |
|---|---|
| `0002-repository-conventions.md` | `2026-10-23-repository-conventions.md` |
| `0010-adr-directories.md` | `2026-11-04-adr-directories.md` |
| `0019-simplify-adr-commands.md` | `2026-11-05-simplify-adr-commands.md` |

This doc loses its temporary `status` line and is revised to the decisions as built.

**ADRs.** ADRs in force live in `docs/architecture/decisions/active/`, and retired ones in
`inactive/`, renamed from `superseded/`. Moving an ADR to `inactive/` retires it, with or without
a successor. An ADR that replaces earlier ones names them in a `Supersedes:` line for the
history. ADRs keep `ADR-NNNN` ids and `NNNN-short-title.md` file names, numbered in sequence across
both directories, and are cited by id in prose so citations survive a move. Architecture docs cite
the ADRs they rest on. As a guideline rather than a check, an ADR's decision isn't rewritten:
typos and links can be fixed, and a changed decision gets a new ADR. A human developer approves an
ADR in its PR. `tags.yaml` and the YAML front matter go.

**Templates.** `docs/templates/design-doc.md` is deleted. `docs/templates/adr.md` becomes a heading
(`# ADR-NNNN: Decision title`), a `Date:` line, an optional `Supersedes:` line, and the four
sections (Context, Decision, Alternatives considered, Consequences), each with a one-line prompt,
without the seven-step comment.

**xtask.** Delete `xtask/src/adr/` and the `adr` command. `check-docs` keeps only the relative
link check, dropping the design doc name, number, and status checks and `front_matter`.
`check-ready` checks only that `docs/wip/` is empty or missing, and its message becomes
`docs/wip/<name>: remove branch notes before merge (see docs/guides/development.md#branch-notes)`,
matching the development guide and still pointing to its branch notes section. Tests for the
removed checks go, and the tests for what remains stay. Nothing new needs testing. Dependencies
are unchanged, since `lexopt` and `pulldown-cmark` are still used. The `adr` recipe leaves the
justfile, and `check-repo`'s comment says it checks links.

**Docs.**

- The architecture and design guide's ADR and design doc sections are rewritten to the
  conventions above, without tooling.
- `docs/README.md`'s "What lives where" table: ADR changes become "a changed decision gets a new
  ADR; retired ADRs move to `inactive/`", with sign-off by a human developer; design doc changes
  become "revised during development to reflect its decisions by merge, then left as a record",
  with sign-off in PR review. The Statuses section is removed, and ADR rows in "Read this before"
  point to `active/` instead of `cargo xtask adr list`.
- The glossary's ADR, design doc, and branch notes entries are updated.
- The architecture overview's `xtask` section and its ADR sentence lose `adr list`.
- The repository guide's command table, layout, and Repository checks section follow the reduced
  xtask.
- With `tags.yaml` gone and no ADRs yet, `docs/architecture/decisions/` disappears from git, with
  no placeholder files. The architecture overview's link to that directory points at the guide's
  ADR section instead, and `docs/README.md`'s ADR row names `active/` and `inactive/` in code
  spans.

### Outside the repository

**Removed-guidelines summary**, at `../in-progress/removed-guidelines.md`, written alongside PR 1
and completed with PR 2. It's grouped by theme: workflow and tiers, TDD and test approval, the
design doc lifecycle, ADR tooling, issue linkage and branch naming, branch notes, clarification
triggers, agent tooling, the merge procedure, and smaller directives. For each item it records
what the guideline said, quoted or closely paraphrased, where it lived, why it was removed, and
where it went, if anywhere.

**Local `CLAUDE.md`**, at `~/Workspaces/math-workbench/CLAUDE.md`, which Claude Code loads for
the main clone and every worktree under `../worktrees/`. It holds:

- Use the superpowers skills. Brainstorming specs go to `docs/design/YYYY-MM-DD-<topic>.md`, and
  plans go to `docs/wip/`.
- The design log: during development, record changes to design decisions in `docs/wip/`, and fold
  them into the design doc before the PR is ready.
- Each session works on its own branch in its own worktree, and keeps its changes there, since
  other sessions may run in parallel. The location is already in the global `CLAUDE.md`.
- When code disagrees with an ADR or the architecture overview, report the mismatch rather than
  deciding which is right.
- The merge procedure: merge only when asked, draft the squash commit message, show it, and merge
  only once it's approved, with `gh pr merge <N> --squash --subject <subject> --body-file <file>`.
- Three habits from the old branch notes table, provisionally: out-of-scope findings become
  `needs-triage` issues, context about specific code goes in a comment at that code, and dead ends
  go in the design doc.

**User settings.** Add the former allowlist, including the read-only `gh` commands, to
`permissions.allow` in `~/.claude/settings.json`, where it applies to every project. Attribution is
already off there.

**Issues.** PR 1 closes #11. #5 and #12 are obsolete after PR 1, and #4 after PR 2. They're closed
with a short comment, after checking with a human developer.

## Left to the builder

- The wording of every rewritten doc and both seeds, within the decisions above.
- Branch names, commit structure within each PR, and the PR descriptions.
- How the summary phrases each item, and which removed details are worth quoting in full.
- Whether PR 2 branches from `main` after PR 1 merges, or from PR 1's branch while it's in review.
