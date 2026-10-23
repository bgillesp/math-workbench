---
status: draft
---

# ADR Directories and the `adr` Command

> Design doc for [#10](https://github.com/bgillesp/math-workbench/pull/10), which closes
> [#3](https://github.com/bgillesp/math-workbench/issues/3).

## Context

The [conventions design doc](0002-repository-conventions.md) keeps each ADR's status in its front
matter. Superseding an ADR means changing the old ADR's status to `superseded by ADR-NNNN`, adding
`supersedes:` to the new one, and moving entries between sections of the hand-maintained index in
`decisions/README.md`. `cargo xtask check-docs` then verifies that these copies agree. Most of its
15 ADR and index checks exist only to keep the copies consistent, and every supersession edits an
accepted ADR in place.

The only fact about an ADR that changes because of another ADR is whether it's still in force.
This design records that fact as the ADR's location rather than in its contents. Front matter
holds what never changes, and is written once, when the ADR is created. Tags, which classify ADRs
by what they affect and may be revised as the architecture grows, live in a single file beside
the ADRs. A new `cargo xtask adr` command reads both to print the views the index used to
provide, and creates new ADRs. [#4](https://github.com/bgillesp/math-workbench/issues/4) builds
agent context on these views and on the tags.

No ADRs exist yet, and bootstrap step 6 will write the first ones, so landing this first avoids a
migration.

## Goals and non-goals

Goals:

- Whether an ADR is in force is visible from its path, without reading it.
- Front matter is written once, when the ADR is created, and never refers to later ADRs.
- Tags can change without editing any ADR.
- The front matter schema and the tags file are explicit and checked, and every view of the ADRs
  is computed from them, so nothing has to be kept in sync.
- One command creates an ADR and moves any it supersedes, so agents do it right by default.
- `xtask` gains no dependencies.

Non-goals:

- Proposed, rejected, or deprecated ADRs on `main`.
- Checking ADR prose: headings, citations of other ADRs, or links. These are conventions, and a
  mistake is a typo to fix when it's noticed.
- A registry of allowed tags, or descriptions of what each tag covers.
- Changing design docs. They keep their `status` field, which changes as their own work
  progresses rather than when other documents change.
- Enforcing in CI that accepted ADRs aren't edited.
- Renumbering ADRs whose numbers collide across branches.
- Context that combines ADRs with crate docs, which is #4.

## Design

### Layout

```text
docs/architecture/decisions/
├── tags.yaml       # the tags of every ADR
├── active/         # ADRs in force
│   ├── 0002-versioned-project-files.md
│   └── 0003-wire-types-checked-on-connect.md
└── superseded/     # ADRs replaced by a later ADR, kept for reference
    └── 0001-wires-are-dynamically-typed.md
```

ADR files are named `NNNN-slug.md` in both directories, and a missing directory counts as empty.

No index file is committed. A generated index would hold nothing the front matter and tags don't,
and it would need regenerating whenever an ADR is added. `cargo xtask adr` prints the same views
on demand. On GitHub, `active/` and `superseded/` still list the ADRs by number and slug, and each
ADR's front matter renders as a table.

An ADR is merged only once it's accepted: its PR is the proposal, and merging the PR accepts it. A
rejected proposal isn't merged. Its closed PR is the record, and anything worth keeping goes in
the accepted ADR's "Alternatives considered". To retire a decision without replacing it, a short
new ADR supersedes it and records why. So an ADR leaves `active/` only by being superseded, and
`superseded/` holds exactly the ADRs that a later ADR replaces.

A superseded ADR moves to `superseded/` in the PR that adds its successor, and its content doesn't
change. Git sees the move as a rename, so `git log --follow` and blame keep its history.

### Front matter

```yaml
---
id: ADR-0003
title: "Wire types: checked on connect"
summary: "Every wire has a static type, and connecting two ports checks compatibility, so a
  mistyped connection is rejected before anything is evaluated."
date: 2026-11-02
supersedes: ADR-0001
---
```

| Field | Required | Format |
|---|---|---|
| `id` | Yes | `ADR-NNNN`, matching the number in the file name |
| `title` | Yes | Double-quoted, non-empty |
| `summary` | Yes | Double-quoted, non-empty, at most 300 characters; a sentence or two |
| `date` | Yes | `YYYY-MM-DD`, the local date the ADR was created |
| `supersedes` | No | Comma-separated ids of the ADRs this one replaces |

The front matter is a flat subset of YAML, so GitHub still renders it as a table. A plain YAML
value can't contain `: `, which titles often do, so `title` and `summary` are always
double-quoted. A quoted value may continue on indented lines, and the line breaks fold into
single spaces, as in YAML. Quoted values can't contain `"` or `\`, so the parser needs no escape
handling. Unknown keys are disallowed.

### Tags

`tags.yaml` gives each ADR, in force or superseded, one or more tags saying broadly what it
affects:

```yaml
ADR-0001: graph
ADR-0002: file-format
ADR-0003: graph, wires
```

It uses the same flat subset of YAML as the front matter, with one entry per ADR, sorted by id.
A tag is lowercase letters, digits, and hyphens. Unlike the ADRs, `tags.yaml` is a living file:
tags can be added, renamed, or merged in any PR as the architecture grows, and a superseded ADR
keeps its tags.

Two branches that each add an ADR both add an entry here, so they conflict on this file. They
already collide on the ADR number, though, and resolving the conflict is part of renumbering. Two
branches retagging the same ADR at once is the only other source of conflicts.

There's no registry of allowed tags. To keep near-duplicates such as `graph` and `graphs` from
creeping in, `adr new` notes any tag that no other ADR uses, and lists the existing tags.

### Checks

`check-docs` verifies, for the ADRs in both directories:

1. Every file is named `NNNN-slug.md`.
2. The front matter follows the schema above.
3. `id` matches the number in the file name, and ids are unique across both directories.
4. Each `supersedes` entry names an existing ADR with a lower number, which rules out
   self-references and cycles.
5. No ADR appears in more than one `supersedes` list, so a superseded ADR has one successor.
6. An ADR is in `superseded/` if and only if some ADR lists it in `supersedes`.
7. `tags.yaml` has exactly one entry for each ADR and none for an ADR that doesn't exist, and each
   entry has at least one well-formed tag. A missing `tags.yaml` counts as empty.

These replace the current ADR and index checks, and the design doc and link checks are unchanged.
The new checks follow the pattern `docs.rs` already uses: each kind of problem is an enum variant,
and messages are formatted only for output.

### The `adr` command

`cargo xtask adr` handles all work with ADRs, and prints Markdown to stdout.

| Command | Prints or does |
|---|---|
| `adr list [--all] [--tags[=<tags>]] [--verbose]` | ADRs by number, or grouped by tag |
| `adr tags` | Each tag, with the number of ADRs in force that use it |
| `adr print <N>...` | The named ADRs in full |
| `adr new <title> --tags=<tags> [--supersedes=<N>,...]` | Creates an ADR, as described below |

An option that takes a value uses the attached form, as in `--tags=graph,wires`, and a list is
comma-separated. `--tags graph`, with a space, is an error that suggests the attached form, since
`adr list --tags` also stands alone. An ADR number may be given as `3`, `0003`, or `ADR-0003`,
since agents copy ids from prose.

`adr list` prints the ADRs in force by number, each with its tags:

```markdown
- ADR-0002: Versioned project files (file-format)
- ADR-0003: Wire types: checked on connect (graph/wires)
```

`--all` adds the superseded ADRs, each noting its successor, so a reader can look up any ADR cited
by id:

```markdown
- ADR-0001: Wires are dynamically typed (graph), superseded by ADR-0003
```

`--tags` groups the ADRs under each tag instead, with tags sorted alphabetically, and
`--tags=graph,wires` groups them under just those tags, in the order given. An ADR with several
tags appears under each of them, and a tag that no ADR uses is an error that lists the existing
tags:

```markdown
## graph

- ADR-0003: Wire types: checked on connect (graph/wires)

## wires

- ADR-0003: Wire types: checked on connect (graph/wires)
```

`--verbose` adds each ADR's summary below its entry, in either form, wrapped at 80 columns with a
two-space indent so it stays part of the list item:

```markdown
- ADR-0003: Wire types: checked on connect (graph/wires)
  Every wire has a static type, and connecting two ports checks compatibility,
  so a mistyped connection is rejected before anything is evaluated.
```

The width is fixed because the standard library can't query the terminal's, and 80 columns reads
well in a typical terminal. The wrap is greedy, counts characters rather than display
width, and gives a word longer than the line a line of its own.

`adr tags` lists each tag in `tags.yaml` with the number of ADRs in force that use it, sorted
alphabetically:

```markdown
- file-format (1)
- graph (1)
- wires (1)
```

`adr print` prints each ADR's file in full, preceded by a line giving its path and whether it's in
force or superseded, and by which ADR. A number with no ADR is an error.

Entries aren't wrapped, since they're short. With no ADRs, `adr list` and `adr tags` print "None
yet." The views don't run the checks. An ADR whose front matter can't be read is left out with a
warning on stderr naming its file, an ADR with no entry in `tags.yaml` is listed without tags, and
`check-docs` reports the details.

### Creating and superseding ADRs

```sh
cargo xtask adr new "Wire types: checked on connect" --tags=graph,wires --supersedes=1
```

`adr new` first checks its arguments. The title must be non-empty without `"` or `\`, there must
be at least one tag and each must be well-formed, and each ADR to supersede must exist and be in
`active/`. It then:

1. takes the next number, one more than the highest in either directory;
2. derives the slug from the title, lowercasing ASCII letters and digits and turning every other
   run of characters into a single hyphen;
3. writes `active/NNNN-slug.md`: front matter with `id`, `title`, today's date, any `supersedes`,
   and an empty `summary`, followed by the body of `docs/templates/adr.md` after its instructions
   comment, with the number and title filled into the heading;
4. adds the new ADR's entry to `tags.yaml`;
5. moves each superseded ADR to `superseded/`;
6. prints the new file's path and the remaining steps, and notes any tag that no other ADR uses,
   with the list of existing tags.

Until the summary is written, `check-docs` reports it, which serves as the reminder. The slug
isn't checked against the title, so the file can be renamed if the derived slug reads badly.

Two branches that each add an ADR take the same number. The ruleset requires a branch to be up
to date with `main` before it merges, so the second branch fails check 3 once it's updated, and
its ADR is renumbered by hand.

The full workflow for a new ADR is:

1. Run `cargo xtask adr new`.
2. Write the summary and the body. A superseding ADR restates whatever still holds from the ADRs
   it replaces, so the current decision can be read from one document.
3. Open a PR. Merging it accepts the ADR.

### Conventions that aren't checked

The template's heading, `# ADR-NNNN: Title`, keeps the formatting consistent, but `check-docs`
doesn't verify it. ADRs and design docs cite ADRs by id in prose, as in `ADR-0003`, rather than
with relative links. Such a link breaks when its target moves, and fixing it would mean editing a
frozen document. Living docs may link to ADR files. The link check then catches a link that a
move breaks, and the fix is usually to point at the successor. Typo fixes to accepted ADRs are
fine.

### Implementation notes

- The ADR schema, the tags file, the checks, the views, and `adr new` go in a new
  `xtask/src/adr.rs`. `docs.rs` keeps the design doc and link checks.
- No new dependencies. The front matter subset is parsed by hand, and `tags.yaml` is read with the
  same parser.
- Today's date is the local date. The standard library has no time zones, but
  `git var GIT_COMMITTER_IDENT` ends with the current Unix time and the local UTC offset, and
  `xtask` already runs `git`. `adr new` reads just those last two fields, adds the offset to the
  time, and converts the days to a civil date. It passes a placeholder identity with
  `-c user.name=xtask -c user.email=xtask`, since `git var` otherwise fails when no identity is
  configured. Git also honors `GIT_COMMITTER_DATE`, which lets a test pin the date. If git can't
  run at all, `adr new` uses the UTC date from `SystemTime` and says so.
- `main.rs` gains `adr`, which takes a subcommand. Like the top-level commands, the subcommands
  are listed in one place that also generates the usage text. `lexopt` parses each one's
  arguments, reading option values with `optional_value`, so only the attached form is accepted.
- `adr` runs as `cargo xtask adr` directly, without a `just` recipe. `just` joins variadic
  arguments with spaces, so a quoted title would arrive in pieces, while `cargo xtask` passes
  arguments through the same way in every shell. Recipes stay for the checks that `just ci` runs.

### Docs to update

- `docs/architecture/decisions/README.md` is removed, and `tags.yaml` is added with no entries.
- `docs/templates/adr.md`: the body `adr new` copies, with a comment saying to create ADRs with
  `cargo xtask adr new` rather than by copying the template.
- `docs/guides/architecture-and-design.md`: the ADR section gains the directories, the tags, the
  workflow with `adr new`, the views, and the conventions that aren't checked. It should make
  clear that `adr new` moving a superseded ADR doesn't count as editing it, that tags can change
  freely, and that ADRs are cited by id. Its references to the index, in the ADR section and in
  the list of where knowledge lives, point to `cargo xtask adr list` instead.
- `docs/guides/repository.md`: the "Commands" section, which should say that `adr` runs as
  `cargo xtask adr` rather than through a recipe; the `xtask` commands table; and the description
  of `check-docs`.
- `docs/guides/development.md`: step 1 of the workflow, where a superseding ADR is accepted
  first, which now means merged in its own PR, and the link to the index under "Starting work",
  which becomes `cargo xtask adr list --tags` until #4 replaces it.
- `docs/README.md`: the ADR row in "What lives where"; the "Statuses" section, which keeps design
  doc statuses, describes the ADR directories in place of ADR statuses, and no longer says ADRs
  keep a status in front matter; and the links to the index in "Read this before…".
- `docs/templates/design-doc.md`: the Context prompt asks to link the ADRs the work operates under.
  It should ask instead to cite by id any ADRs the design relies on.
- `docs/architecture/overview.md`: the `xtask` section, and its link to the index.
- `docs/glossary.md`: the ADR entry's link to the index, and a mention of `active/` and tags if
  useful.

The conventions design doc is frozen and isn't edited. Its banner already defers to the living
docs.

## Alternatives considered

**Status in front matter, with both ends of each supersede link checked.** The current design. It
works, but most of its checks exist to keep copies consistent, and every supersession edits the
old ADR.

**Status in front matter, with a tool writing the copies.** A `sync-docs` command would write
`superseded by` into old ADRs and regenerate the index, and `check-docs` would verify that nothing
is stale. That removes the hand-maintained copies, but a generator still writes into accepted
ADRs, and status still has to be read from file contents.

**An `area` field in front matter.** An earlier version of this design gave each ADR one area in
its front matter, to group the views. But a classification changes as the architecture grows,
and front matter is written once, so an area couldn't be corrected or split without editing an
accepted ADR. One area per ADR was also too narrow for decisions that span several parts of the
system.

**A committed, generated index.** An earlier version of this design generated `index.md` with a
command and checked in CI that it was current. `Cargo.lock` is committed because it records
dependency resolutions that `Cargo.toml` alone can't reproduce, but the index would record nothing
beyond the front matter and tags. It would need regenerating whenever an ADR is added. Its one
advantage is a rendered view on GitHub without a development environment, and the people who most
need the view are working in one.

**All metadata in one file**, such as `decisions.toml`. It's the cleanest database, but an ADR
file would no longer describe itself. Only the tags move to a shared file, since they're the one
piece of metadata that changes.

**JSON or TOML for the tags.** Either would need a new dependency, or a hand-written parser more
complex than the flat YAML subset the front matter already needs, and JSON is clumsy to edit by
hand.

**A full YAML or TOML parser.** `serde_yaml` is archived and its forks are uneven, and GitHub
doesn't render TOML front matter. A flat YAML subset with quoting rules covers the schema.

**Formal schema tooling**, such as JSON Schema validation or loading the records into SQLite. It's
more machinery, often from outside the Rust ecosystem, than a handful of constraints needs.

**Checking prose citations and links to ADRs.** It would catch mistyped ids, but policing Markdown
prose adds friction for little gain. A wrong citation is a typo to fix when it's noticed.

## Acceptance criteria

`check-docs`:

- passes for empty or missing ADR directories and a missing or empty `tags.yaml`;
- passes for a consistent set of ADRs that includes a chain of supersessions (ADR-0001
  superseded by ADR-0002, which is superseded by ADR-0003);
- reports each of the following, naming the file:
  - a file in `active/` or `superseded/` not named `NNNN-slug.md`;
  - a missing or empty required field, an unknown key, or a malformed `id`, `date`, or
    `supersedes` entry;
  - a `title` or `summary` that isn't double-quoted or contains `"` or `\`, and a `summary` over
    300 characters;
  - an `id` that doesn't match the file name, and an id used twice across both directories;
  - a `supersedes` entry naming an ADR that doesn't exist or doesn't have a lower number;
  - an ADR listed in more than one `supersedes`;
  - an ADR in `superseded/` that no ADR supersedes, and an ADR in `active/` that one does;
  - an ADR with no entry in `tags.yaml`, an entry for an ADR that doesn't exist, two entries for
    one ADR, and an entry with no tags or a malformed tag.

`adr list`, `adr tags`, and `adr print`:

- `adr list` prints the ADRs in force by number, each with its tags;
- `--all` adds the superseded ADRs, each with its successor;
- `--tags` groups the ADRs under every tag, alphabetically, and `--tags=<tags>` under just the
  given tags, in order, with an ADR appearing under each of its tags;
- `--tags=<tags>` naming a tag that no ADR uses fails, listing the existing tags;
- `--verbose` adds each ADR's summary below its entry, flat or grouped, wrapped at 80 columns with
  a two-space indent;
- `adr tags` prints each tag with the number of ADRs in force that use it;
- `adr list` and `adr tags` print "None yet." when there are no ADRs;
- `adr print` prints each named ADR in full after a line giving its path and status, and fails
  for a number with no ADR;
- each accepts ADR numbers as `3`, `0003`, or `ADR-0003` where it takes them;
- an option value given with a space instead of `=` fails with a message suggesting the attached
  form;
- an ADR whose front matter can't be read is left out, with a warning naming its file.

`adr new`:

- creates `active/NNNN-slug.md` with the next number across both directories, the derived slug,
  the filled-in fields and heading, today's local date, and an empty summary;
- adds the ADR's entry to `tags.yaml`, keeping the entries sorted by id;
- moves each superseded ADR to `superseded/` unchanged;
- notes any tag that no other ADR uses, listing the existing tags;
- refuses an empty title, a title with `"` or `\`, a missing `--tags`, a malformed tag, or a
  `--supersedes` target that doesn't exist or is already superseded, and changes nothing when it
  refuses;
- leaves the ADRs in a state where `check-docs` reports only the empty summary.

A quoted summary that continues on indented lines parses as a single line with single spaces.

The docs listed above describe the new layout, the tags, and the commands.

## Test plan

Unit tests in `adr.rs` cover the front matter parser (quoting, continuation lines, and each kind
of malformation), reading `tags.yaml`, each check (comparing the full list of problems, as
`docs.rs` does), the views against expected Markdown for a small set of ADRs, argument parsing
for each subcommand, number parsing, slug derivation, next-number selection, wrapping (a line
that fits, a break at the width, and a word longer than the line), reading the time and offset
from git's output (including negative offsets and a time near midnight), and the date conversion,
including the epoch and leap days.

Tests of `adr new` run against a directory under `std::env::temp_dir()`, since `xtask` avoids a
`tempfile` dependency.

A manual run in a scratch copy of the repository covers the workflow end to end: create an ADR,
supersede it, print each view, and run `just ci`.
