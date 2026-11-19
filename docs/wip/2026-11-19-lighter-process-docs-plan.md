# Lighter Process Docs (PR 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land PR 1 of the lighter agent guidelines design: the process docs, the style guide
seeds, and the removal of shipped agent config, plus the local setup and the removed-guidelines
summary outside the repository.

**Architecture:** Almost all docs. The development guide becomes a guide to repository usage, its
style rules move into two seeded style guides, and `AGENTS.md` shrinks to description, roles,
commands, and pointers. The only code change is dropping `doctor`'s symlink warning. PR 2 (ADR and
design doc conventions, xtask deletion) gets its own plan once this PR is up.

**Tech Stack:** Markdown docs, `just`, `cargo xtask` (Rust), GitHub CLI.

**Spec:** `docs/design/2026-11-19-lighter-agent-guidelines.md` (sections "PR 1: process" and
"Outside the repository").

**Worktree:** `~/Workspaces/math-workbench/worktrees/lighter-process-docs`, branch
`lighter-process-docs`, no upstream until the first push.

## Global Constraints

- `just check` passes after every task. `just ci` fails while this plan is in `docs/wip/`, and
  passes once Task 7 empties it.
- Guides and agent instructions never name superpowers or any other skill pack.
- Say "a human developer", never "the human" or a solo-developer framing.
- Clear, fairly concise prose. Paragraphs when points form one line of thought; bullets and bold
  lead-ins only for items that each carry their own idea. Wrap Markdown at 100 columns, like the
  existing docs.
- Every relative link must resolve (`check-docs`). Paths outside the repository go in code spans,
  never links.
- Out of scope for this PR, left as they are: the architecture and design guide, the design doc
  template, the ADR template, `docs/architecture/`, `docs/README.md`'s "What lives where" table and
  Statuses section (except the one `pre-pr` clause in Task 2), the glossary's ADR, design doc, and
  branch notes entries, every xtask check, the justfile, CI workflows, lints, and `deny.toml`.
- Content moved between files keeps its wording nearly verbatim, apart from the deletions the spec
  names.

## Review Focus

1. **Stale references.** After the deletions, no guide, template, or `AGENTS.md` mentions
   `pre-pr`, the format hook, `.agents/`, `.claude/skills`, change tiers, "approved test", or
   acceptance criteria as a test requirement, outside the PR 2 scope above. Task 7 greps for them.
2. **A re-encoded workflow.** The merge checklist and the seeds mustn't quietly require tests per
   criterion, human approval of tests, or a design doc for a class of change. Task 3 greps the
   new files.
3. **`check-ready`'s link.** Its error message points to `docs/guides/development.md#branch-notes`
   until PR 2, so the rewritten guide keeps a `## Branch notes` heading. Task 3 checks it.
4. **Windows checkout.** With the `.claude/skills` symlink gone, the repository has no symlinks,
   and `README.md` no longer tells Windows users to enable them. Task 2 checks it.
5. **Formatting reaching review.** With the hook gone, the only guard before CI is the guideline to
   run `just fmt` before committing, so it appears in `AGENTS.md` and the repository guide. Task 4
   checks it.

---

### Task 1: Commit the design doc and this plan

**Files:**
- Create: `docs/design/2026-11-19-lighter-agent-guidelines.md` (already copied into the worktree,
  with a temporary `status: accepted` front matter block)
- Create: `docs/wip/2026-11-19-lighter-process-docs-plan.md` (this file)
- Delete, outside the repo: `~/Workspaces/math-workbench/in-progress/2026-11-19-lighter-agent-guidelines-design.md`,
  the pre-worktree copy, so the design has one home

- [ ] **Step 1: Check the design doc passes the current checks**

Run: `just check-repo`
Expected: `check-docs: ok`. The name reads as number 2026, and the status is `accepted`.

- [ ] **Step 2: Commit**

```bash
git add docs/design/2026-11-19-lighter-agent-guidelines.md docs/wip/2026-11-19-lighter-process-docs-plan.md
git commit -m "docs: add the lighter agent guidelines design and PR 1 plan"
```

- [ ] **Step 3: Delete the copy in `../in-progress/`**

---

### Task 2: Remove the shipped agent config

**Files:**
- Delete: `.agents/skills/pre-pr/SKILL.md`, `.claude/settings.json`, `.claude/skills` (symlink),
  `.codex/hooks.json`, so no `.agents/`, `.claude/`, or `.codex/` directory remains
- Modify: `xtask/src/doctor.rs:1-3` (module doc) and `:29-37` (the symlink warning)
- Modify: `xtask/src/main.rs:50` (the `doctor` summary)
- Modify: `README.md:19-20` (the Windows symlink sentence)
- Modify: `docs/guides/repository.md`: the principles paragraph (`:9-14`), the layout (`:16-47`),
  "Adding a crate" (`:94`), formatting (`:125-127`), the `doctor` row (`:138`), and Agent tooling
  (`:252-279`)
- Modify: `docs/README.md:42-43` (the `pre-pr` clause)
- Keep: the `.gitignore` entry for `.claude/settings.local.json`

**Interfaces:**
- Produces: the repository guide's Agent tooling section, which later tasks and `AGENTS.md` link
  to for the setup note, at `docs/guides/repository.md#agent-tooling`

- [ ] **Step 1: Delete the four files with `git rm`**

- [ ] **Step 2: Simplify `doctor`**

In `doctor.rs`, remove the `.claude/skills` block, so `run` checks only `TOOLS`. The module doc
says it warns about missing tools the checks run. In `main.rs`, `Command::Doctor`'s summary
becomes `"warn about missing tools"`.

- [ ] **Step 3: Update `README.md`**

The Windows sentence about Developer Mode and `core.symlinks=true` goes. "Development works on
Linux, macOS, and Windows." stays.

- [ ] **Step 4: Update the repository guide**

- Principles: "Hooks and skills are conveniences on top, never the only enforcement" becomes:
  agent configuration is up to each developer, and `just ci` is the enforcement.
- Layout: remove the `.agents/`, `.claude/`, and `.codex/` lines. Keep `AGENTS.md` and
  `CLAUDE.md`.
- Adding a crate: "A new crate is a major change, so it follows an accepted design doc." goes. The
  steps stay.
- Formatting: replace "The agent hooks run `just fmt` after each edit" with: run `just fmt` before
  committing, so unformatted code never reaches review, and `just fmt-check` fails CI on
  unformatted code.
- `doctor` row: "Warns about missing tools".
- Agent tooling, rewritten as two short paragraphs. First: `AGENTS.md` holds the instructions,
  and `CLAUDE.md` imports it for Claude Code, while Codex reads it directly. Everything else about
  an agent setup is the developer's own, and personal Claude Code settings go in
  `.claude/settings.local.json`, which git ignores, or user settings. Second, the setup note:
  developers will likely want to allow `just:*` and the cargo subcommands `build`, `check`,
  `clippy`, `doc`, `fmt`, `metadata`, `nextest`, `test`, `tree`, and `xtask` to run without a
  prompt, and the repository doesn't expect commits or PR descriptions to credit agentic
  contributions. Keep the existing line about a nested `AGENTS.md` for a crate with unusual local
  rules. Don't mention `gh`.
- Changing this setup: "Keep the recipes, the workflows, and the commands in `AGENTS.md` in
  agreement" stays. Drop any mention of hooks or skills.

- [ ] **Step 5: Update `docs/README.md`**

The Statuses paragraph's last sentence ends after "`cargo xtask check-ready` checks in `just ci`."
Drop "The check can't tell whether a PR completes an implementation, so the `pre-pr` skill covers
setting `implemented`." The rest of that section is PR 2's.

- [ ] **Step 6: Verify**

Run: `just check`
Expected: passes, including `check-docs: ok`.

Run: `git ls-files -s | awk '$1 == "120000"'`
Expected: no output, so there are no symlinks.

Run: `cargo xtask doctor`
Expected: `doctor: ok`, or only warnings for tools missing locally, and none about symlinks.

- [ ] **Step 7: Commit**

```bash
git commit -am "chore: remove the shipped agent skill and harness config"
```

---

### Task 3: Rewrite the development guide and seed the style guides

**Files:**
- Rewrite: `docs/guides/development.md`
- Create: `docs/guides/issues-and-prs.md`
- Create: `docs/guides/code-and-tests.md`
- Modify: `.github/pull_request_template.md`
- Modify: `docs/guides/repository.md:199` (the test techniques link) and `:208` (the commit
  messages link)

**Interfaces:**
- Produces: these headings, which other docs link to:
  - `development.md`: `## Issues and the Project`, `## Branches and pull requests`,
    `## Testing`, `## Branch notes`, `## Merge checklist`, `## Releases`
  - `issues-and-prs.md`: `## Issues`, `## Pull request titles`, `## Pull request descriptions`,
    `## Commit messages`
  - `code-and-tests.md`: `## Code`, `## Tests`

- [ ] **Step 1: Rewrite `development.md`**

Title "Development Guide". A short intro says this is how the repository is used: Issues, branches,
PRs, tests, and what has to hold before merge. How a developer and their agents plan and carry out
the work is up to them. Then, in order:

- **Issues and the Project.** Issues record problems to solve later, and work doesn't need an issue
  to start. Keep the three label groups verbatim (`kind/…`, `area/<crate>`, `needs-triage`). A
  human developer manages the GitHub Project.
- **Branches and pull requests.** Branch names are a short slug, with no issue number required.
  Drafts skip CI, and a PR is marked ready once the merge checklist holds, which runs CI. PRs are
  squash-merged. Keep the branch protection sentence verbatim. Point to `issues-and-prs.md` for
  titles, descriptions, and commit messages.
- **Testing.** Keep verbatim: where unit, integration, and doc tests live, the command table, the
  nextest note, and "Long-running checks … belong in the extended workflow". Point to
  `code-and-tests.md` for test style and techniques. Drop the TDD sentence and "never weaken …
  an approved test".
- **Branch notes.** `docs/wip/` holds a branch's working notes, committed on the branch so work
  can continue across sessions, and removed before merge, which `just ci` enforces. No destination
  table.
- **Merge checklist.** Exactly these five: `just ci` passes; the living docs (the architecture
  overview, glossary, crate docs, and guides) are updated as the change requires; a design doc, if
  there is one, reflects the decisions made during development; `docs/wip/` is empty; and any new
  dependency, including one between workspace crates, or `unsafe` code is justified in the PR.
- **Releases.** Verbatim.

Removed with no new home: the tier table, "Starting work" with its worktree example, the
five-step workflow and the paragraphs after it, the clarification list, the branch notes table,
"Public items are documented" (`missing_docs` enforces it), "Tests cover the acceptance
criteria", the `pre-pr` line, and the agent merge procedure ("Merge a PR yourself only when…").

- [ ] **Step 2: Create `issues-and-prs.md`**

Title "Issues and Pull Requests", opening with a one-line note that this is a seed, carried over
from earlier guides for a human developer to rewrite. Sections:

- **Issues:** an issue records a problem to handle later. Point to the bug and feature forms in
  `.github/ISSUE_TEMPLATE/`.
- **Pull request titles:** the Conventional Commits bullet from the old guide, verbatim.
- **Pull request descriptions:** follow the template, which has a summary of what changed and why,
  review notes on what to look at first, and new dependencies and `unsafe` code, each with its
  justification. Add `Closes #N` when the PR resolves an issue. Leave out sections that don't
  apply.
- **Commit messages:** the old guide's first paragraph, verbatim (subject, body wrapped at 72,
  what to call out, what to leave out). Add one sentence: a human developer writes or approves the
  squash message when merging. Not the agent procedure.

- [ ] **Step 3: Create `code-and-tests.md`**

Title "Code and Test Style", with the same seed note. Sections:

- **Code:**
  - Doc comments say what an item is for, not just its name restated.
  - A non-trivial struct or enum gets a short doc comment on every field and variant, so coverage
    is complete rather than patchy. An obvious type, such as `Point3 { x, y, z }`, needs none, and
    the fields of a struct-like variant can be covered by the variant's comment.
  - Check the glossary before naming something new.
  - Lints and `unsafe`: see the repository guide's section.
- **Tests:** a short style paragraph: prefer fewer tests, each pinning behavior whose failure would
  be a real regression, and avoid pinning exact wording, incidental output, or choices a later
  change could reasonably make. Then the old "Test techniques" list, nearly verbatim, with the
  snapshot bullet reduced to "Updating a snapshot changes what the test expects, so review each
  update with `cargo insta review`." (drop "like any other test change. Never accept snapshots in
  bulk.").

- [ ] **Step 4: Update the PR template**

Three sections:
- `## Summary`, with placeholder text: what changed and why, and `Closes #N` if it resolves an
  issue.
- `## Review notes`, unchanged.
- `## Dependencies and `unsafe``, unchanged.

Drop the bare `Closes #` line and `## Tests and docs`.

- [ ] **Step 5: Fix the repository guide's two links**

`development.md#testing` (the extended workflow paragraph) → `code-and-tests.md#tests`.
`development.md#commit-messages` (GitHub settings) → `issues-and-prs.md#commit-messages`.

- [ ] **Step 6: Verify**

Run: `just check`
Expected: passes.

Run: `grep -n '^## Branch notes$' docs/guides/development.md`
Expected: one match.

Run: `grep -n -i -E 'acceptance criteria|approved test|tier|tests first|worktree|pre-pr' docs/guides/development.md docs/guides/issues-and-prs.md docs/guides/code-and-tests.md .github/pull_request_template.md`
Expected: no output.

- [ ] **Step 7: Commit**

```bash
git add docs/guides .github/pull_request_template.md
git commit -m "docs: rewrite the development guide and seed the style guides"
```

---

### Task 4: Rewrite the entry points

**Files:**
- Modify: `AGENTS.md`
- Modify: `docs/README.md:50-56` ("Read this before" rows)
- Modify: `docs/glossary.md:25-26` (delete "Change tier")

**Interfaces:**
- Consumes: the guide paths and headings from Task 3, and `docs/guides/repository.md#agent-tooling`
  from Task 2

- [ ] **Step 1: Rewrite `AGENTS.md`**

- Paragraph 1, the project description: unchanged.
- Paragraph 2, roles: a human developer sets direction, approves architecture decisions, and
  reviews PRs before they merge to `main`. Issues record problems to solve later, and work doesn't
  need an issue to start. Planning happens in a GitHub Project that a human developer manages.
- Delete paragraph 3 entirely: worktrees, mismatch reporting, and "Never edit an accepted ADR".
- Repository design: keep the aim of a clean, legible Rust project that works well for agentic
  coding, now via tool-driven validation, in-repository docs for design and architecture, and
  concise style guides. Drop "test-driven development" and "explicit detailed guidance on
  development procedures". Keep the code style sentence.
- Commands: `just fmt` formats code, and you run it before committing. Drop the hook parenthetical.
  The rest is unchanged.
- Important elements: delete the `.agents/` bullet. The others are unchanged.
- Application overview: unchanged.
- Resources: Development (repository usage: issues, branches, PRs, tests, and the merge
  checklist), Issues and PRs (style), Code and tests (style), Architecture and Design, and
  Repository and Infrastructure (including agent setup), each linking its guide.

- [ ] **Step 2: Update "Read this before" in `docs/README.md`**

- "Writing or changing tests" → the code and test style guide, and the development guide's
  testing section.
- Add "Writing an issue or PR" → the issues and PRs guide.
- "Planning a major change" → "Writing a design doc", reading the ADRs in force. Drop the template
  link, since the template is deleted in PR 2.
- "Changing the checks, CI, or agent tooling" → "Changing the checks or CI".
- Leave the two ADR rows as they are, for PR 2.

- [ ] **Step 3: Delete the glossary's "Change tier" entry**

- [ ] **Step 4: Verify**

Run: `just check`
Expected: passes.

Run: `grep -n 'just fmt' AGENTS.md docs/guides/repository.md`
Expected: both files say to run it before committing.

- [ ] **Step 5: Commit**

```bash
git commit -am "docs: slim AGENTS.md and point the entry points at the new guides"
```

---

### Task 5: Local setup (outside the repository)

**Files:**
- Create: `~/Workspaces/math-workbench/CLAUDE.md`
- Modify: `~/.claude/settings.json`

- [ ] **Step 1: Write the local `CLAUDE.md`**

A short note that these are a human developer's local conventions for this project, loaded for the
main clone and every worktree. Then:

- Use the superpowers skills. Brainstorming specs go to `docs/design/YYYY-MM-DD-<topic>.md`, with
  no `-design` suffix, and plans go to `docs/wip/`.
- The design log: during development, record changes to design decisions in `docs/wip/`, and fold
  them into the design doc before the PR is marked ready.
- Each session works on its own branch in its own worktree (location per the global `CLAUDE.md`),
  and keeps its changes there, since other sessions may run in parallel.
- When code disagrees with an ADR or the architecture overview, report the mismatch rather than
  deciding which is right.
- Merging: merge only when asked; draft the squash commit message following the issues and PRs
  guide, show it, and merge only once it's approved, with
  `gh pr merge <N> --squash --subject <subject> --body-file <file>`.
- Provisional habits from the old branch notes table: out-of-scope findings become `needs-triage`
  issues, context about specific code goes in a comment at that code, and dead ends go in the
  design doc.

- [ ] **Step 2: Add the allowlist to `~/.claude/settings.json`**

Add `permissions.allow` with exactly the entries removed from `.claude/settings.json` in Task 2:
`Bash(just:*)`; `Bash(cargo <sub>:*)` for `build`, `check`, `clippy`, `doc`, `fmt`, `metadata`,
`nextest`, `test`, `tree`, and `xtask`; and `Bash(gh <cmd>:*)` for `issue list`, `issue view`,
`pr checks`, `pr diff`, `pr view`, `project item-list`, and `project view`. Leave every existing
key as it is.

- [ ] **Step 3: Verify**

Run: `python3 -m json.tool ~/.claude/settings.json > /dev/null && echo ok`
Expected: `ok`.

---

### Task 6: Removed-guidelines summary (outside the repository)

**Files:**
- Create: `~/Workspaces/math-workbench/in-progress/removed-guidelines.md`

**Interfaces:**
- Consumes: `git diff main...HEAD` on this branch, the source of what was removed

- [ ] **Step 1: Write the summary for PR 1**

An intro: why the guidelines were removed (from the design doc's "Why"), and the date. Then one
section per theme: workflow and tiers; TDD and test approval; issue linkage and branch naming;
branch notes; clarification triggers; agent tooling (the `pre-pr` skill, the symlink, the format
hook, permissions, attribution); the merge procedure; and smaller directives (worktrees, mismatch
reporting, never editing an accepted ADR in `AGENTS.md`, the agent review pass). For each item:
what it said, quoted where the wording matters; where it lived (file and section); why it went;
and where it went, if anywhere (local `CLAUDE.md`, a style seed, or nowhere). End with a "PR 2"
section listing the themes still to come: the design doc lifecycle and statuses, the design doc
template, the ADR front matter, tags, and supersede checks, and `adr list` and `adr tags`.

- [ ] **Step 2: Check coverage against the diff**

Every removed line of guidance in `git diff main...HEAD -- AGENTS.md docs .github .agents .claude .codex`
is covered by an item, or is a pure rewording of text that's still present.

---

### Task 7: Finish the branch

- [ ] **Step 1: Sweep for stale references**

Run: `git grep -n -i -E 'pre-pr|\.agents|\.claude/skills|hook|symlink|change tier|approved test|tests first' -- ':!docs/design' ':!docs/wip'`
Expected: no output. Fix any match outside PR 2's scope (the architecture and design guide and
the templates).

- [ ] **Step 2: Empty `docs/wip/`, then run `just ci`**

```bash
git rm docs/wip/2026-11-19-lighter-process-docs-plan.md
just ci
```
Expected: passes, ending with `check-ready: ok`.

- [ ] **Step 3: Commit**

```bash
git commit -m "docs: remove the PR 1 plan from branch notes"
```

- [ ] **Step 4: Push and open the PR** (only once a human developer has said to)

Title: `docs: lighten the agent guidelines and drop shipped agent config`. The description follows
the new template:

- **Summary:** the why, in two sentences, with `Closes #11`.
- **Review notes:**
  - Review order: the development guide and the seeds, then `AGENTS.md`, then the rest.
  - The intermediate states until PR 2: the ADR checks, the design doc name and status checks, the
    Statuses section, the design doc template, and this design doc's temporary `status` line.
- **Dependencies and `unsafe`:** none.

Draft first, marked ready when a human developer agrees.

- [ ] **Step 5: Ask before closing #5 and #12**

These are obsolete once this PR merges. Close each with a one-line comment pointing to the PR, but
only after a human developer confirms. #4 waits for PR 2.
