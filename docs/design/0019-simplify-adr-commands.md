---
status: accepted
---

# Simplify the ADR Commands

> Design doc for [#19](https://github.com/bgillesp/math-workbench/pull/19), part of
> [#16](https://github.com/bgillesp/math-workbench/issues/16). Two follow-up PRs finish the work,
> and the last closes #16.

## Context

[#16](https://github.com/bgillesp/math-workbench/issues/16) asks to shrink `xtask::adr`. ADRs are
written rarely, but the [ADR directories design](0010-adr-directories.md) gave them 2,790 lines of
tooling: 1,646 of code and 1,144 of tests and fixtures. The weight comes from four places:

- **Commands that save little typing.** `adr new` is 589 lines, with slug derivation, `tags.yaml`
  editing, and a calendar conversion from git's clock, to automate something done a few times a
  year. `adr print` prints a file.
- **Partial records.** `check` reports everything in one run, so it works on half-read ADRs
  (`Fields`, `Checked`), and tracks which keys and entries exist even when they're unreadable, so
  one mistake doesn't cascade. Reading therefore has two paths: `parse_front_matter` for the views,
  and `read_front_matter` for the checks.
- **A stricter format than it needs.** `title` and `summary` must be quoted whether or not they need
  it, the reader keeps plain and quoted values distinct, and its errors are mapped to problems
  separately for front matter and for `tags.yaml`.
- **Tags keyed by ADR.** `tags.yaml` needs exactly one non-empty entry for each ADR, which takes
  several checks, and the tags in use are scattered through the file.

No ADRs exist yet and `tags.yaml` is empty, so the format can change without a migration.

## Goals and non-goals

Goals:

- Writing, superseding, retagging, and renumbering an ADR are ordinary edits to text files, guided
  by a template and by the messages from `check-docs`.
- `xtask` reads ADRs and never writes them.
- An ADR's metadata stays in its front matter, as flat YAML that a real parser reads the same way.
- `check-docs` still validates the metadata that readers and views rely on: file names, the front
  matter schema, the supersede links, the directories, and the references in `tags.yaml`.
- `check-docs` and the views share one loading path.
- `xtask` gains no dependencies.

Non-goals:

- Changing the `active/` and `superseded/` layout, or how ADRs are accepted and frozen.
- Parsing an ADR's body. Its heading stays a convention that isn't checked.
- Changing design doc front matter.
- Automating anything `adr new` did. It can come back in a smaller form if it's missed.
- `cargo xtask context` ([#4](https://github.com/bgillesp/math-workbench/issues/4)), though this
  design changes what it builds on, as described below.

## Design

### Approach

The tool reads and validates, and people write. Three rules follow, which also serve as the test
for later additions:

- **Only authors write ADRs.** Every change is an ordinary edit, which a template lays out and
  `check-docs` verifies.
- **Metadata stays in structured places.** The tool reads file names, directories, front matter, and
  `tags.yaml`, and never parses an ADR's body.
- **Checks guard what would mislead a reader or a view,** and don't police style. A check's message
  says what to do, since messages now carry the guidance `adr new` used to give.

### Front matter

```yaml
---
id: ADR-0003
title: "Wire types: checked on connect"
summary: Every wire has a static type, and connecting two ports checks compatibility, so a
  mistyped connection is rejected before anything is evaluated.
date: 2026-11-02
supersedes: ADR-0001
---
```

| Field | Required | Format |
|---|---|---|
| `id` | Yes | `ADR-NNNN`, matching the number in the file name |
| `title` | Yes | Non-empty |
| `summary` | Yes | Non-empty, at most 300 characters; a sentence or two |
| `date` | Yes | A real date, written as `YYYY-MM-DD` |
| `supersedes` | No | Ids of the ADRs this one replaces, separated by spaces |

Unknown keys are disallowed. `supersedes` uses spaces rather than commas to match `tags.yaml`, so
both use one helper for lists of ids, and an empty one lists no ADRs. `date` is there for someone
reading the file, and no view prints it for now, though a later one could.

### Reading flat YAML

`yaml.rs` reads the flat subset of YAML that front matter and `tags.yaml` use, with one set of rules
for every value:

- Each entry is `key: value`, and the value may be empty. Lines indented with spaces continue the
  value, whether it's quoted or not, and fold into single spaces. A blank line or `#` comment line
  ends the value and is otherwise skipped. Anything else is malformed, including a line indented
  with a tab, and an indented line with no value to continue.
- A repeated key is a problem, since YAML 1.2 requires keys to be unique.
- A value may be wrapped in double quotes, and means the same either way, so `date: "2026-11-02"`
  reads as a date.
- No value contains `"` or `\` once any wrapping quotes are removed, so there are no escapes to
  handle.
- A non-empty unquoted value can't contain `:` or `#`, and starts with a letter or digit.

The last rule covers what a YAML parser does differently from plain text:

| Unquoted value | What a YAML parser does |
|---|---|
| `Wire types: checked on connect` | Rejects it, and GitHub shows an error in place of the table |
| `Follows up on #16` | Silently ends the value at ` #` |
| `` `wgpu` for rendering `` | Rejects it, as it does most other leading punctuation |

YAML treats only `: ` and ` #` as special, so `ratio 3:1` and `C#` would be legal, but a
single-character rule is simpler to state and quoting is always available. Backslashes are safe in
plain YAML values, but in double quotes `\t` silently becomes a tab, so banning them everywhere
keeps the two forms identical.

The line rules cover the other differences. YAML keeps a blank line inside a value as a line break,
and rejects a comment line inside one, and it rejects tab indentation in unquoted values. Treating
all of these as malformed means the reader rejects what it can't represent rather than misreading
it.

These rules were compared against PyYAML on random values built from YAML's special characters, on
one line and across several, and everything they accept reads as the same text. The exception is a
whole value that reads as a number, boolean, or date, such as `title: 123`, which is harmless here.
The blank line, comment, and tab cases were checked by hand against PyYAML and Psych.

The reader builds a map from key to value as it goes, which is how it finds a repeated key, and
reports problems as `yaml::Error`, with variants for a malformed line, a repeated key, a bad closing
quote, a stray quote or backslash, and a value that needs quotes. Both callers pass these errors
through unchanged as structural problems, so a repeated key or bad value in front matter fails that
ADR like any other malformed header. A value the reader can't read stays out of the map, so a
required field with a bad value is reported as missing too. Two problems for one mistake is simpler
than keeping track of the keys whose values couldn't be read.

### `tags.yaml`

```yaml
# Each line is a tag, then the ADRs it applies to, by id, separated by spaces.
file-format: ADR-0002
graph: ADR-0001 ADR-0003
wires: ADR-0003
```

A tag is lowercase letters, digits, and hyphens. Keying by tag means the existing tags are in view
while tagging an ADR, which takes over from the note `adr new` printed about near-duplicates, and
renaming a tag is a one-line edit. An ADR doesn't need a tag, but one with no tags can't be found by
tag, so the guide asks for at least one by convention. Order isn't checked, and neither is a tag
with no ADRs.

Two branches that add an ADR to one tag conflict on its line. If they add the same new tag at
different places in the file, the merge is clean, but the repeated key then fails the check rather
than silently dropping an ADR from the tag.

### Loading

Checks fall into two groups, which are also the two stages of loading:

- **Structural checks** establish everything a well-formed `Adr` needs, with every field present and
  in the right format. A field that becomes a number has to parse, while a field stored as a string
  can hold anything for now, including nothing. The same goes for `tags.yaml`.
- **Consistency checks** validate the `Adr`s against the semantic rules: `title` and `summary`
  aren't empty, `date` is a real date, and the ADRs agree with their file names and with each other.

The ADR files and `tags.yaml` each have a step that reads them and a step that runs their structural
checks, and `Records` brings the results together:

1. `load_adr_files` reads the files in `active/` and `superseded/`. A missing directory counts as
   empty.
2. `parse_adr_files` parses each file into an `Adr`, or finds every structural problem with them.
3. `load_tags_file` reads `tags.yaml`. A missing file counts as empty.
4. `parse_tags_file` parses it into a map from tag to ADR numbers, or finds every structural problem
   with it.
5. `Records::new` puts the `Adr`s in order by number and gives each one its tags.

`load_records` runs all of these, and is the one path into the ADRs. It returns either every record
or every structural problem, and a problem in the ADR files doesn't stop `tags.yaml` being parsed,
so one run reports all of them. `load_tags` runs just steps 3 and 4.

```rust
/// An ADR whose file passed the structural checks.
pub(super) struct Adr {
    /// The directory it's in, which says whether it's in force.
    dir: Dir,
    /// The file name, such as `0003-wire-types-checked-on-connect.md`, which the consistency
    /// checks compare with `number`.
    name: String,
    /// The number in `id`, which identifies the ADR.
    number: u32,
    /// The value of `title`, which may be empty.
    title: String,
    /// The value of `summary`, on one line, which may be empty.
    summary: String,
    /// The value of `date`, which the consistency checks require to be a real date.
    date: String,
    /// The numbers in `supersedes`.
    supersedes: Vec<u32>,
    /// The tags in `tags.yaml` that list this ADR, sorted.
    tags: Vec<String>,
}

/// Each tag in `tags.yaml`, with the numbers of the ADRs it lists.
pub(super) type Tags = BTreeMap<String, BTreeSet<u32>>;

/// Every ADR and tag, once they've all passed the structural checks.
pub(super) struct Records {
    /// Every ADR, by number.
    adrs: Vec<Adr>,
    /// Each tag in `tags.yaml`, with the numbers of the ADRs it lists.
    tags: Tags,
}

impl Records {
    /// The records for `adrs` and `tags`, with the ADRs by number, each given its tags.
    pub(super) fn new(adrs: Vec<Adr>, tags: Tags) -> Self;
}

/// Reads the files in `active/` and `superseded/`, in that order and sorted by name.
fn load_adr_files(root: &Path) -> io::Result<Vec<AdrFile>>;

/// Parses each of `files` into an `Adr`, or finds every structural problem with them.
pub(super) fn parse_adr_files(files: &[AdrFile]) -> Result<Vec<Adr>, Vec<Problem>>;

/// Reads `tags.yaml`.
fn load_tags_file(root: &Path) -> io::Result<String>;

/// Parses the text of `tags.yaml` into a map from tag to ADR numbers, or finds every structural
/// problem with it.
pub(super) fn parse_tags_file(text: &str) -> Result<Tags, Vec<Problem>>;

/// Reads every ADR and `tags.yaml`, or finds every structural problem with them.
pub(super) fn load_records(root: &Path) -> io::Result<Result<Records, Vec<Problem>>>;

/// Reads just `tags.yaml`, or finds every structural problem with it.
pub(super) fn load_tags(root: &Path) -> io::Result<Result<Tags, Vec<Problem>>>;

/// Runs the consistency checks.
pub(super) fn check(records: &Records) -> Vec<Problem>;
```

| Caller | Reads | Fails on |
|---|---|---|
| `check-docs` | `load_records`, then `check` | Structural or consistency problems |
| `adr list` | `load_records` | Structural problems |
| `adr tags` | `load_tags` | Structural problems in `tags.yaml` |

Because `check` takes `Records`, it can't run on a partial set. That's the main reason the
partial-record machinery can go. Consistency checks over an incomplete set report false problems: if
ADR-0004 fails to parse, an ADR it supersedes appears to have no successor, and a tag naming it
appears to point at nothing. So `Fields`, `Checked`, and the sets that track unreadable keys and
entries all go. The cost is a second run when a branch has problems of both kinds.

`adr list` refuses to show a partial list, since an ADR that's silently missing could mislead a
reader. It doesn't run the consistency checks, and it handles records that break them, so an ADR in
the wrong directory, with a mismatched file name, or with an empty summary still lists. That's why
the checks for non-empty fields are consistency checks: a new ADR with its summary still unwritten
doesn't block `adr list`, and `check-docs` still reports it.

### Checks

Structural, in `records.rs`:

1. The file starts with a front matter block that reads as flat YAML under the rules above.
2. Its keys are all known, and `id`, `title`, `summary`, and `date` are present.
3. `id` is an id like `ADR-0003`.
4. Each `supersedes` entry is an id.
5. `tags.yaml` reads as flat YAML, each key is a well-formed tag, and each value is a list of ids.

Consistency, in `check.rs`, for each ADR:

6. The file is named `NNNN-slug.md`, with the number in `id`. This is one check on `name`, which
   reports a malformed name and a mismatched number with the same message.
7. `title` and `summary` aren't empty, and `summary` is at most 300 characters.
8. `date` is a real date written as `YYYY-MM-DD`, which also rules out an empty one.

And across ADRs:

9. Numbers are unique across both directories.
10. Each `supersedes` entry names an existing ADR with a lower number, which rules out cycles.
11. No ADR appears in more than one `supersedes` list.
12. An ADR is in `superseded/` if and only if some ADR lists it in `supersedes`.
13. Each id in `tags.yaml` names an existing ADR.

This takes `ProblemKind` from 27 variants to about 20, plus five in `yaml::Error`, so the check list
itself shrinks only modestly. The savings come from having one path through it. What goes is the
requirement that every ADR has a tags entry, the rule that `title` and `summary` are quoted, and the
separate handling of malformed lines in `tags.yaml`. Everything about the supersede graph stays,
since it's the metadata that's hardest to keep right by eye, and now that nothing moves files for
the author it matters more.

### The template

`docs/templates/adr.md` carries the workflow, and can't pass `check-docs` until it's filled in:

```markdown
---
id: ADR-NNNN
title:
summary:
date: YYYY-MM-DD
supersedes:
---

# ADR-NNNN: Decision title

<!--
To write an ADR, work through these steps, and delete this comment last. The architecture and
design guide explains the reasoning.

1. Copy this file to docs/architecture/decisions/active/NNNN-short-title.md, and set `id` to match.
   NNNN is one more than the highest number in `cargo xtask adr list --all`, and the title part is
   lowercase letters, digits, and hyphens.
2. Fill in the rest of the front matter: the title, a summary of a sentence or two stating the
   decision in at most 300 characters, and today's date. If this ADR replaces earlier ones, list
   their ids in `supersedes`, separated by spaces, and otherwise delete that line. A value can
   continue on indented lines. Put it in double quotes if it contains `:` or `#`, or starts with
   anything but a letter or digit. No value can contain `"` or `\`.
3. Fill in the heading, with the same number and title.
4. Write the sections. A superseding ADR restates whatever still holds from the ADRs it replaces,
   so the current decision can be read from one document.
5. Move each ADR this one replaces to superseded/ with `git mv`, without changing its contents.
6. Add this ADR's id to its tags in tags.yaml. `cargo xtask adr tags` lists the existing tags, and
   it's best to reuse one where it fits.
7. Run `cargo xtask check-docs`, which reports anything missed, and open a PR. Merging it accepts
   the ADR.
-->

## Context

The problem, the forces at play, and what's known. Link the issue or design doc where the
question came up.
```

The remaining sections are unchanged. `ADR-NNNN` isn't an id, so a copy has a structural problem
until `id` is set, which is part of the first step. From then on, `adr list` keeps working while the
ADR is being written, and `check-docs` reports the empty `title` and `summary` and the placeholder
`date`. So `check-docs` keeps the reminder that `adr new` gave through its empty summary, now for
every field.

The comment sits below the heading, as it does in the design doc template, since at the top of the
file it would be a structural problem until it's deleted. Nothing checks that it was deleted, since
the tool doesn't parse the body, but it doesn't render and review will catch it. The heading isn't
checked either, so a stale one goes unnoticed.

The common edits then look like this:

| Task | Edits |
|---|---|
| New ADR | Copy the template to `active/NNNN-slug.md` and fill it in, then add the id to `tags.yaml` |
| Supersede an ADR | The above, with `supersedes` filled in, and `git mv` the old file to `superseded/` |
| Retag | Edit `tags.yaml` |
| Renumber after a collision | Rename the file, and update `id`, the heading, and `tags.yaml` |

### The `adr` command

| Command | Prints |
|---|---|
| `adr list [--all] [--verbose] [--tags[=<tags>]]` | The ADRs in force by number, with their tags |
| `adr tags` | Every tag in `tags.yaml`, one per line, sorted |

`--all` adds the superseded ADRs, each noting its successors. `--tags` groups the ADRs under every
tag of the ADRs shown, alphabetically, so a tag used only by superseded ADRs appears only with
`--all`. Any ADRs shown with no tags come last under a `(no tags)` heading, so the grouping leaves
no ADR out, and the heading can't clash with a tag, which can't contain parentheses.
`--tags=graph,wires` groups them under just those tags, in the order given, without that heading. In
both, an ADR appears under each of its tags, as it does today. `--verbose` adds each ADR's path,
which replaces the lookup `adr print` provided, and its summary, wrapped at 80 columns with a
two-space indent as it is today:

```markdown
## graph

- ADR-0003: Wire types: checked on connect (graph/wires)
  docs/architecture/decisions/active/0003-wire-types-checked-on-connect.md
  Every wire has a static type, and connecting two ports checks compatibility,
  so a mistyped connection is rejected before anything is evaluated.
```

For an ADR whose summary is still empty, which only `check-docs` rejects, `--verbose` prints just
the path. With no ADRs, `adr list` and `adr tags` print "None yet.", and a list with only superseded
ADRs to leave out prints "None in force.", also as they do today.

A value for `--tags` is always attached, since `--tags` can stand alone. `adr list` takes no other
values, so any value it's given gets an error suggesting the attached form. An unknown tag's error
points to `adr tags` rather than printing the existing tags:

```text
error: `graph` looks like a value for `--tags`; attach it as `--tags=graph`
error: there's no tag `graphs`; `cargo xtask adr tags` lists the tags
```

A few things go with the removed commands and options. With `new` gone, `adr list` is the only
command that takes `--tags`, so the first error moves there, and it needs neither `detached_value`
nor the state that tracked a bare `--tags`. The `UnknownTag` type goes too. Without `print` and
`new --supersedes`, nothing parses ADR numbers from arguments. The call into git for the local date
goes with `adr new`, since the author now writes the date.

`adr` also gains a recipe, `just adr <args>`. It had none because `just` splits a quoted title, and
no argument contains spaces now. `cargo xtask adr` still works, and the docs that use it can stay.
The usage text points to the template, for anyone looking for `adr new`.

### Effect on #4

`cargo xtask context` plans to use `adr list --tags --verbose` for its baseline, which still works,
now with any ADRs that have no tags under `(no tags)`. For named tags, `context` can call
`records::load_records`, filter, and read each `Adr`'s file, since `Adr` no longer holds the text. A
structural problem on a branch will stop `context` as it stops `adr list`.

### Implementation notes

| File | Before (code + tests) | After the Loading PR | Role |
|---|---|---|---|
| `mod.rs` | 288 + 41 | 265 + 227 | Dispatch, usage, `Dir`, ids and names |
| `records.rs` | 312 + 102 | 299 + 354 | The load and parse steps, `load_records`, `load_tags` |
| `check.rs` | 303 + 302 | 266 + 276 | `Problem`, the consistency checks |
| `list.rs` | 158 + 185 | 159 + 179 | `adr list` |
| `yaml.rs` | 131 + 69 | 131 + 69 | Flat YAML entries and their errors |
| `fixtures.rs` | 161 | 111 | Sample ADRs, one scratch directory helper |
| `new.rs` | 368 + 221 | Removed | |
| `print.rs` | 53 + 39 | Removed | |
| `tags.rs` | 33 + 24 | 26 + 21 | `adr tags` |

That's from 2,790 lines to 2,383 after the Loading PR. Production code goes from 1,646 lines to
1,146, almost all of that from removing `adr new` and `adr print`, and tests go from 1,144 to 1,237.
The Loading PR's gain is in structure rather than size: it leaves the production code about the
same, and adds tests for the split between the two stages. The Format PR mostly rewrites `yaml.rs`
and the `tags.yaml` parser, so it shouldn't change these totals much.

`AdrFile` lasts only from reading a file to parsing it, since nothing keeps its text after that.
`docs::front_matter` stays as it is, since design docs have a single `status` field.

The work lands in three PRs, each with its own test review, with sizes measured for the PRs that
have been built:

| PR | Changes | Changed lines |
|---|---|---|
| 1. Commands | Removes `new.rs` and `print.rs`, simplifies `adr list` and `adr tags`, adds the recipe, and drops the requirement that every ADR has tags. Rewrites the template's instructions for the current format. This PR numbers the design doc | 1,292 besides this doc, almost all deletions |
| 2. Loading | `load_records` and `load_tags`, `check` over `Records`, and the split between structural and consistency checks. `adr list` and `adr tags` fail on structural problems. Passes and fails `check-docs` on the same files as before | 1,695 besides this doc, 901 of them in tests |
| 3. Format | The new `yaml.rs` rules, the fields that depend on them, and `tags.yaml` keyed by tag, with the template and guide updated. Closes #16 | About 500, estimated |

Loading goes before the format so the format change touches one reading path rather than both.
`yaml.rs` and the `tags.yaml` parser are leaves of the pipeline, so replacing them doesn't disturb
its structure. The second PR does port a few things the third replaces, such as `NotQuoted` and
`DuplicateTagsEntry`, for about 30 lines of churn.

### Docs to update

- `docs/templates/adr.md`: as above.
- `docs/architecture/decisions/tags.yaml`: gains its header comment.
- `docs/guides/architecture-and-design.md`: the ADR section's front matter, tags, workflow, and
  views. The steps live in the template, and the guide links to them and keeps the reasoning.
- `docs/guides/repository.md`: the `adr` row, the description of `check-docs`, and the exception
  for `adr` under "Commands", which goes.
- `docs/architecture/overview.md`: `adr` no longer prints or creates ADRs.
- `docs/glossary.md`: not every ADR has tags.
- `xtask/src/main.rs`: the crate docs and the `adr` summary.

The first PR makes the changes about commands, and the third those about the format. The ADR
directories design is frozen and isn't edited.

## Alternatives considered

**The title and summary in the Markdown body.** An earlier draft took the title from the heading and
the summary from the opening paragraph, leaving only dates and ids in front matter, so the reader
would need no quoting rules at all. But that makes the metadata implicit in prose and spreads it
across the file, and the rules for a flat subset of YAML are small and well defined.

**Keeping the last value of a repeated key, as #16 proposes.** Parsers such as PyYAML and Psych do
this, and it would be slightly simpler, but YAML 1.2 forbids repeated keys, and it would silently
drop an ADR from a tag in the merge case under "`tags.yaml`".

**Dropping `id`, and taking the number from the file name.** This removes one edit when renumbering,
but the front matter would no longer be the ADR's complete record.

**Making `id`'s format a consistency check,** so an untouched copy of the template loads. An earlier
draft did this, but `Adr` would then hold an unparsed `id`, when its number is what identifies the
ADR. Setting `id` is a small step, and the string fields can wait.

**Having `adr list` warn and show the ADRs that parsed.** This was an earlier draft's approach to
structural problems. It keeps the command working on a branch with a broken file, but shows a list
that's silently incomplete. Treating empty fields as consistency problems covers the common case, a
new ADR that isn't finished yet, and a structural problem is worth fixing first.

**Running the consistency checks over whichever ADRs parsed, as #16 proposes.** It reports more in
one run, but reports false problems whenever a file fails to parse, as described under "Loading".
Suppressing them is the tracking this design removes.

**Making bare `--tags` an error.** An earlier version of this design did this, leaving the list of
tags to `adr tags`, but grouping every ADR by tag is a useful overview, #4 plans to use it, and it
costs only a few lines.

**Leaving ADRs with no tags out of `--tags`.** This is slightly simpler, and the guide asks for at
least one tag by convention, but nothing enforces that, so the grouped view could be silently
incomplete, which is what `adr list` otherwise refuses to show.

**Forwarding bare `--tags` to `adr tags`.** It's friendlier for someone who wants the list of tags,
but gives one output two spellings, and loses the overview.

**Accepting `--tags a,b` as well as `--tags=a,b`.** Since `--tags` can stand alone, the parser can't
tell whether a following value belongs to it, so a value has to be attached.

**A flat list for `--tags=`, rather than grouping under each tag.** An earlier draft did this, so an
ADR with two tags would appear once. The grouped output is already there and does the job.

**Keeping a reduced `adr new`** that copies the template with the next number filled in. It would be
about 30 lines, but the number is one command away and the check catches mistakes. It's easy to add
if writing ADRs by hand proves annoying.

**Plain strings for problems instead of an enum.** This would save about 50 lines, but it departs
from the pattern `docs.rs` uses, and ties tests to wording.

## Acceptance criteria

Commands, in the first PR:

- `adr list` prints the ADRs in force by number, each with its tags;
- `--all` adds the superseded ADRs, each with its successors;
- `--tags` groups the ADRs under every tag of the ADRs shown, alphabetically, with an ADR appearing
  under each of its tags, and combines with `--all` and `--verbose`;
- `--tags` lists any ADRs shown with no tags last, under a `(no tags)` heading, which `--tags=a,b`
  doesn't add;
- `--tags=a,b` groups the ADRs under those tags, in the order given, with an ADR appearing under
  each of its tags, and combines with `--all` and `--verbose`;
- `--tags a` with a space fails with a message giving the attached form;
- `--tags=` naming a tag not in `tags.yaml` fails with a message pointing to `adr tags`, without
  listing the tags;
- `--verbose` adds each ADR's path, and its summary wrapped at 80 columns, both with a two-space
  indent;
- `adr tags` prints every tag in `tags.yaml`, one per line, sorted, without counts;
- with no ADRs, `adr list` and `adr tags` print "None yet.", and a list with only superseded ADRs to
  leave out prints "None in force.";
- `just adr list --tags=a,b` runs the command;
- `adr new` and `adr print` are unknown subcommands;
- `check-docs` passes for an ADR with no tags.

Loading, in the second PR:

- `check-docs` passes for empty or missing ADR directories, and a missing or empty `tags.yaml`;
- it passes for a consistent set of ADRs that includes a chain of supersessions;
- one run reports every structural problem, across all the ADRs and `tags.yaml`, naming each file:
  - no front matter block, any problem from the reader, or an unknown key;
  - a missing `id`, `title`, `summary`, or `date`;
  - a malformed `id`, or a `supersedes` entry that isn't an id;
  - in `tags.yaml`, any problem from the reader, a malformed tag, or an entry that isn't an id;
- when any of those is present, it reports no consistency problems;
- otherwise it reports each of the following, naming the file:
  - a file in `active/` or `superseded/` not named `NNNN-slug.md`, or with a different number from
    its `id`;
  - an empty `title` or `summary`, or a `summary` over 300 characters;
  - a `date` that isn't a real date written as `YYYY-MM-DD`, including an empty one;
  - a number used twice across both directories;
  - a `supersedes` entry naming an ADR that doesn't exist or doesn't have a lower number;
  - an ADR listed in more than one `supersedes`;
  - an ADR in `superseded/` that no ADR supersedes, and an ADR in `active/` that one does;
  - a tag listing an ADR that doesn't exist;
- with a structural problem in any ADR or in `tags.yaml`, `adr list` prints the problems, lists no
  ADRs, and fails;
- with only consistency problems, such as an empty summary, a mismatched file name, or an ADR in
  the wrong directory, `adr list` lists the ADRs and succeeds;
- `adr tags` succeeds when an ADR has a structural problem, and fails when `tags.yaml` has one.

Format, in the third PR:

- the reader folds lines indented with spaces into a value with single spaces, quoted or not,
  including a value that starts on the line after its key;
- a blank line or `#` comment line ends a value and is otherwise skipped, so an indented line after
  one is malformed;
- a line indented with a tab is malformed;
- it treats `x` and `"x"` as the same value, and accepts an empty value;
- it reports a line that isn't an entry, a repeated key, a quoted value with no closing quote or
  with text after it, a value containing `"` or `\`, and a non-empty unquoted value that contains
  `:` or `#` or doesn't start with a letter or digit, each as a structural problem;
- an empty `supersedes` lists no ADRs, without a problem;
- `tags.yaml` is read as tags keyed to ADRs, so `graph: ADR-0001 ADR-0003` tags both ADRs.

The docs listed above describe the new format and commands.

## Test plan

Unit tests in each module of `xtask/src/adr/` cover the reader (each rule, with the values in the
table above among the cases), the structural checks, `tags.yaml`, each consistency check (comparing
the full list of problems, as `docs.rs` does), `load_records` returning problems rather than
records when any structural check fails, and the views against expected Markdown for a small set of
ADRs.

These tests work on in-memory text, except those of `load_records` and the commands, which take a
repository root and use scratch directories.

A manual run in a scratch copy of the repository covers the workflow end to end, following the
template's instructions: write an ADR by hand, supersede it, print each view, and run `just ci`.
