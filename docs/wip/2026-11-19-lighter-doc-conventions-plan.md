# Lighter Doc Conventions (PR 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this
> plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land PR 2 of the lighter agent guidelines design. Design docs are named by date with no
status, ADRs live in `active/` and `inactive/` with no tooling, `tags.yaml` and the design doc
template are deleted, and xtask's ADR tooling and design doc checks are deleted. That leaves
`check-docs` checking links and `check-ready` checking `docs/wip/`.

**Architecture:** First delete the code and the docs that describe it (Task 1), then the design doc
conventions (Task 2), then the ADR conventions (Task 3), then revise the design doc to what was
built and finish (Task 4). PR 1 (#26) is merged, and its review notes listed the leftovers this PR
cleans up.

**Tech Stack:** Rust (`xtask`, with `lexopt` and `pulldown-cmark`), Markdown, `just`.

**Spec:** `docs/design/2026-11-19-lighter-agent-guidelines.md`, section "PR 2: ADR and design doc
conventions".

**Worktree:** `~/Workspaces/math-workbench/worktrees/lighter-doc-conventions`, branch
`lighter-doc-conventions`, no upstream. Don't push, open a PR, or touch `main`.

## Global Constraints

- `just check` passes after every task. `just ci` passes once Task 4 empties `docs/wip/`.
- Say "a human developer", never "the human" or a solo-developer framing. Write clear, fairly
  concise prose, using paragraphs when points form one line of thought. Wrap Markdown at 100
  columns, except table rows.
- Every relative link must resolve (`check-docs`). Paths outside the repository go in code spans.
- **Historical design docs** (the three renamed ones) change in three ways only: the rename,
  removing the `status` front matter block, and fixing link targets the renames break. Nothing else
  in their text changes.
- **The architecture and design guide** has passages in a human developer's own voice, including
  double spaces after periods. Keep the intro and the first paragraph of each section as written,
  except for the specific clauses this plan names. Rewrite the rest of each section.
- Don't change CI workflows, lints, `deny.toml`, `AGENTS.md`, the development guide, or the two
  style guides.
- The guides and agent instructions never name superpowers.

## Review Focus

1. **Stale references.** No live doc mentions `adr list`, `adr tags`, `just adr`, `tags.yaml`,
   `superseded/`, design doc statuses, PR-numbered design docs, or acceptance criteria as a design
   doc requirement. The three historical design docs and the spec are exempt. Task 4 greps for
   these.
2. **Anchors.** CI doesn't check anchors. Links to `architecture-and-design.md#architecture-decision-records`,
   `#design-documents`, and `development.md#branch-notes` must still hit real headings. Task 4
   checks them.
3. **The vanished directory.** Once `tags.yaml` is deleted, `docs/architecture/decisions/` is gone
   from git. Links that pointed at it are retargeted to the guide's ADR section, with no
   placeholder files. Task 3 checks this.
4. **`check-ready`'s message.** It still names a heading that exists, and it matches the
   development guide ("removed before merge"). Task 1 tests it.
5. **The human developer's voice.** In the architecture and design guide, the intro and opening
   paragraphs read as before, apart from the named clauses. Task 3 diffs them.

---

### Task 1: Delete the ADR tooling and design doc checks

**Files:**
- Delete: `xtask/src/adr/` (the whole directory)
- Modify: `xtask/src/main.rs`: drop `mod adr;`, `Command::Adr` and its arms, and the special case in
  `parse_args`. `Command::ALL` becomes `[Self; 3]`. The module doc and `parse_args`'s doc lose
  their mentions of `adr`.
- Modify: `xtask/src/docs.rs`: drop `DESIGN_DIR`, `DESIGN_STATUSES`, `read_files`, `doc_number`,
  `front_matter`, `check_design_docs`, the three design doc `ProblemKind` variants, the
  `adr::check_docs` call, and their tests (`doc_numbers`, `front_matter_fields`, `design_docs`,
  and the design rows in `problems_display_as_messages`). The module doc says it checks relative
  links in Markdown files. Remove imports that become unused.
- Modify: `xtask/src/ready.rs`: drop the design doc check, so `check(notes: &[String]) ->
  Vec<Problem>` and `Problem` has only `BranchNotes { name }`. Drop the `unaccepted_design_docs_fail`
  test and the `design_doc` helper. The module doc says it checks that `docs/wip/` is empty or
  missing.
- Modify: `justfile`: delete the `adr` recipe and its comment. `check-repo`'s comment becomes
  `# Check relative links in docs`.
- Modify: `docs/guides/repository.md`:
  - Layout: the `xtask/` comment becomes `# repository checks`.
  - Commands: delete the `just adr` row.
  - The paragraph after the table: "a branch in progress is expected to have branch notes and a
    `draft` design doc" becomes "a branch in progress is expected to have branch notes".
  - Repository checks: the table has rows `check-docs` "Checks relative links in Markdown files",
    `check-ready` "Checks that `docs/wip/` is empty or missing", and `doctor`. The `adr` row goes.
  - Replace the three-bullet `check-docs` list and the statuses sentence with one paragraph, kept
    from the links bullet.
  - Delete the sentence about `adr` being a directory module.
- Modify: `docs/architecture/overview.md`: the `xtask` section says "Repository checks, run through
  `just`: relative links in docs and merge readiness". Drop the `adr` sentence.

**Interfaces:**
- Produces: `check-ready`'s message, exactly
  `docs/wip/<name>: remove branch notes before merge (see docs/guides/development.md#branch-notes)`

- [ ] **Step 1: Update `ready.rs`'s message test first**

In `problems_display_as_messages`, the `BranchNotes` case expects
`"docs/wip/plan.md: remove branch notes before merge (see docs/guides/development.md#branch-notes)"`.
Delete the two `DesignNotAccepted` assertions.

- [ ] **Step 2: Run it and watch it fail**

Run: `cargo nextest run -p xtask problems_display_as_messages`
Expected: the `ready` test fails on the message, or fails to compile, if the variant is already gone.

- [ ] **Step 3: Make the code changes above**

Change the `BranchNotes` message to `"{WIP_DIR}/{name}: remove branch notes before merge (see docs/guides/development.md#branch-notes)"`,
then make the deletions in `ready.rs`, `docs.rs`, and `main.rs`, and delete `xtask/src/adr/`.

- [ ] **Step 4: Run the xtask tests**

Run: `cargo nextest run -p xtask`
Expected: all pass.

- [ ] **Step 5: Update the justfile, repository guide, and overview as listed**

- [ ] **Step 6: Verify**

Run: `just check`
Expected: passes. `check-docs: ok` no longer checks design doc names.

Run: `cargo xtask --help`
Expected: lists `check-docs`, `check-ready`, and `doctor` only.

- [ ] **Step 7: Commit**

```bash
git add -A xtask justfile docs/guides/repository.md docs/architecture/overview.md
git commit -m "refactor(xtask)!: remove the ADR tooling and design doc checks"
```

---

### Task 2: Date-named design docs without status

**Files:**
- Rename with `git mv`, deleting the leading `---`/`status: …`/`---` block and the blank line after
  it:
  - `docs/design/0002-repository-conventions.md` → `2026-10-23-repository-conventions.md`
  - `docs/design/0010-adr-directories.md` → `2026-11-04-adr-directories.md`
  - `docs/design/0019-simplify-adr-commands.md` → `2026-11-05-simplify-adr-commands.md`
- Fix the links these renames break: `docs/guides/repository.md:6`, `0010`'s link to `0002`, and
  `0019`'s link to `0010`. The link check finds any others.
- Modify `docs/design/2026-11-19-lighter-agent-guidelines.md`: delete the `status: accepted` front
  matter block.
- Delete: `docs/templates/design-doc.md`
- Modify: `docs/guides/architecture-and-design.md`, the `## Design Documents` section
- Modify: `docs/README.md`: the Design docs row of "What lives where", and the whole `## Statuses`
  section
- Modify: `docs/glossary.md`: the Design doc entry

- [ ] **Step 1: Rename and strip the four docs, delete the template, and fix links**

Run: `just check-repo`
Expected: `check-docs: ok`. Any broken link it reports gets fixed in place.

- [ ] **Step 2: Rewrite the guide's Design Documents section**

It covers:
- docs are named `docs/design/YYYY-MM-DD-<topic>.md`, dated the day the doc was first written
- whether a change calls for a design doc is up to the developer
- there's no template, front matter, or status, so a doc takes whatever shape suits the task
- a doc changes freely during development, and before its PR merges it reflects the decisions made
- a design whose work isn't finished opens with a short note on what's happening, and the note is
  removed once the work is done
- after that the doc is left as a record, and a later change to the same area gets a new dated doc,
  which can link the old one
- design docs cite ADRs by id

Drop "begins with the creation and approval", acceptance criteria, numbering by PR, statuses, and
freezing.

- [ ] **Step 3: Update `docs/README.md` and the glossary**

- `docs/README.md`, the Design docs row: Changes becomes "Revised during development to reflect
  its decisions by merge, then left as a record", and Sign-off becomes "PR review". Delete the
  `## Statuses` section. The ADR paragraph in it is restated in the guide in Task 3.
- The glossary's Design doc entry becomes: the design for a change that calls for one, revised
  during development to reflect its decisions by merge and then left as a record. It's named by the
  date it was first written, as in `docs/design/2026-11-19-lighter-agent-guidelines.md`.

- [ ] **Step 4: Verify and commit**

Run: `just check`
Expected: passes.

```bash
git add -A docs
git commit -m "docs: name design docs by date and drop their statuses and template"
```

---

### Task 3: ADR conventions without tooling

**Files:**
- Delete: `docs/architecture/decisions/tags.yaml`
- Rewrite: `docs/templates/adr.md`
- Modify: `docs/guides/architecture-and-design.md`: the intro's first bullet, and the
  `## Architecture Decision Records` section
- Modify: `docs/README.md`: the ADR row of "What lives where", and the "Changing a crate" and
  "Proposing an architecture decision" rows of "Read this before"
- Modify: `docs/architecture/overview.md:5-6`
- Modify: `docs/glossary.md`: the ADR and Branch notes entries

- [ ] **Step 1: Delete `tags.yaml` and retarget the links to `decisions/`**

`docs/README.md`'s ADR row: the Location becomes `` `architecture/decisions/active/` and
`inactive/` `` as code spans, and Changes becomes "A changed decision gets a new ADR; retired ADRs
move to `inactive/`", with Sign-off "Human developer". In `overview.md`, the sentence linking `decisions/` and naming
`cargo xtask adr list` becomes: the
reasoning lives in the ADRs in force, in `decisions/active/`, linking to
`../guides/architecture-and-design.md#architecture-decision-records`.

Run: `just check-repo`
Expected: `check-docs: ok`.

- [ ] **Step 2: Rewrite the ADR template**

```markdown
# ADR-NNNN: Decision title

Date: YYYY-MM-DD
Supersedes: ADR-NNNN (only for an ADR that replaces earlier ones; otherwise delete this line)

## Context

<one-line prompt from the current template>

## Decision

<one-line prompt>

## Alternatives considered

<one-line prompt>

## Consequences

<one-line prompt>
```

Use the current template's section prompts, trimmed to one line each. Drop the front matter and
the seven-step comment.

- [ ] **Step 3: Rewrite the guide's ADR content**

- Intro bullet: "`cargo xtask adr list` for a listing of the architecture decisions in force"
  becomes "`docs/architecture/decisions/active/` for the architecture decisions in force".
- First paragraph, which keeps its voice: "the decision of an ADR is modified or nullified by
  creating a superseding ADR" becomes: a decision is changed by a new ADR, or retired by moving its
  ADR to `inactive/`. "ADRs are approved and merged in their own PR" becomes: a human developer
  approves an ADR in its PR. "Apart from typo fixes, an accepted ADR is never edited." becomes the
  guideline: an ADR's decision isn't rewritten, though typos and links can be fixed, and a changed
  decision gets a new ADR. Delete "Agents draft a new ADR only when a human developer asks for
  one."
- The examples paragraph stays.
- Replace everything after it, up to `## Design Documents`, with a short description:
  - ADRs live in `docs/architecture/decisions/`: the ones in force in `active/`, and retired ones
    in `inactive/`
  - files are named `NNNN-short-title.md`, with ids like `ADR-0003`, numbered in sequence across
    both directories, and two branches that take the same number renumber one by hand
  - retiring an ADR means moving it to `inactive/`, with or without a successor, and an ADR that
    replaces earlier ones names them in its `Supersedes:` line
  - to write an ADR, copy the template
  - ADRs and design docs cite ADRs by id in prose, so citations survive a move, and architecture
    docs cite the ADRs they rest on
  - no tooling checks ADRs

- [ ] **Step 4: Update the rest**

- `docs/README.md`, "Read this before": "Changing a crate" reads its `lib.rs` docs and the ADRs in
  force for its area, in `architecture/decisions/active/`. "Proposing an architecture decision"
  reads the ADR workflow link and the ADRs in force. Drop both `cargo xtask adr` mentions.
- Glossary: the ADR entry ends "…Cited as `ADR-0003`. The ADRs in force are in
  `architecture/decisions/active/`, and retired ones in `inactive/`." The Branch notes entry
  becomes "Working notes in `docs/wip/`, kept only on a branch and removed before merge."

- [ ] **Step 5: Verify**

Run: `just check`
Expected: passes.

Run: `git ls-files docs/architecture/decisions`
Expected: no output.

Run: `git diff HEAD -- docs/guides/architecture-and-design.md | head -40`
Expected: only the named clauses change in the intro and the ADR section's first paragraph.

- [ ] **Step 6: Commit**

```bash
git add -A docs
git commit -m "docs: keep ADRs in active and inactive directories without tooling"
```

---

### Task 4: Revise the design doc, complete the summary, and finish

**Files:**
- Modify: `docs/design/2026-11-19-lighter-agent-guidelines.md`
- Modify, outside the repo: `~/Workspaces/math-workbench/in-progress/removed-guidelines.md`
- Delete: `docs/wip/2026-11-19-lighter-doc-conventions-plan.md`

- [ ] **Step 1: Revise the design doc to what was built**

The work is done, so delete the opening `>` note. Then fix where the doc differs from what was
built:
- In "PR 1: process", the repository guide paragraph: "'A new crate is a major change, so it
  follows an accepted design doc' drops its tier clause" becomes "… goes".
- Anything Tasks 1 to 3 did differently from the "PR 2" section, such as the exact `check-ready`
  message, or `overview.md` linking to the guide.

Leave the rest as written.

- [ ] **Step 2: Complete the summary's section 9**

Retitle it "9. PR 2: ADR and design doc conventions". Turn each "still to come" bullet into an
entry in the same form as sections 1 to 8, quoting from `main` at `c33506c` with
`git show c33506c:<path>`: what it said, where it lived, and where it went, if anywhere. Cover the
design doc lifecycle and statuses (`docs/README.md`'s Statuses, the guide's Design Documents
section, and the glossary), the design doc template, ADR front matter and its checks, `tags.yaml`,
the supersede checks, `adr list`, `adr tags`, and `just adr`, "Agents draft a new ADR only when…",
the never-edit rule, the ADR template's comment, and `check-ready`'s old message. Update the intro's
"PR 1 … covers sections 1 to 8" to cover both PRs.

- [ ] **Step 3: Sweep for stale references**

Run: `git grep -n -i -E 'adr list|adr tags|just adr|tags\.yaml|superseded/|status: |acceptance criteria|NNNN-short|permanent homes' -- ':!docs/design/2026-10-*' ':!docs/design/2026-11-0*' ':!docs/design/2026-11-19-*' ':!docs/wip'`
Expected: no output. Fix any match.

Run: `grep -n '^## Architecture Decision Records$\|^## Design Documents$' docs/guides/architecture-and-design.md; grep -n '^## Branch notes$' docs/guides/development.md`
Expected: three matches.

- [ ] **Step 4: Empty `docs/wip/`, run `just ci`, and commit**

```bash
git rm docs/wip/2026-11-19-lighter-doc-conventions-plan.md
just ci
git add -A docs && git commit -m "docs: revise the lighter guidelines design to what was built"
```
Expected: `just ci` passes, ending with `check-ready: ok`.
