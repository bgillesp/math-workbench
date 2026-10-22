# Development Guide

This document provides a practical description of the standard development process in this
repository. Work is tracked in GitHub Issues and prioritized in a GitHub Project managed by a human
developer. Agents working on feature development can read the Project, but don't change it unless
explicitly asked.

Issues carry three types of labels:

- `kind/bug`, `kind/feature`, `kind/chore`, or `kind/question`
- `area/<crate>` for the crate a change mainly touches
- `needs-triage` until a human developer has reviewed and prioritized the issue

Agents may read issues, comment on progress, link PRs with `Closes #N`, and file `needs-triage`
issues for out-of-scope findings.

Changes are classified into one of the following complexity tiers. When the tier isn't clear, ask.

| Tier | Examples | Design doc | Tests first |
|---|---|---|---|
| Minor | typo, dependency bump, rename | No | n/a |
| Standard | bug fix, small feature within one crate | No (the issue is enough) | Yes |
| Major | new tile category, new crate, format change, public API change | Yes, accepted before implementation | Yes |

## Starting work

Each session takes one issue and does development on its own branch in its own git worktree, so
several sessions can run in parallel. Name the branch `<issue-no>-<short-slug>`:

```sh
git fetch origin
git worktree add ../math-workbench-42 -b 42-typed-wire-validation origin/main
```

Before changing a crate, read its `lib.rs` docs and the ADRs in force for its area, listed in the
[ADR index](../architecture/decisions/README.md). Check the [glossary](../glossary.md) before
naming anything new.

For work that will span sessions, keep [branch notes](#branch-notes) in the `docs/wip/` directory,
and open a draft PR early.

## Workflow

1. **Design.** For major changes, draft a design doc from the issue at
`docs/design/NNNN-short-title.md`, where NNNN is the issue number, starting from the
[template](../templates/design-doc.md). The acceptance criteria of the design must be concrete enough to
turn into tests. A human developer reviews and accepts it before implementation starts. If the work
requires changing a decision, a superseding ADR is accepted first.

2. **Tests.** For major and standard changes, write failing tests as a separate commit. Tests for
major changes are derived from the design doc's acceptance criteria, and for standard changes from
the issue notes and any session interaction. A human developer reviews the failing tests before
implementation -- it's the executable spec, and the most valuable review in the PR.

3. **Implement** until the tests pass. Never weaken, delete, or `#[ignore]` an approved test. If one
looks wrong, stop and raise it. Fix lint warnings rather than allowing them, and never loosen
lints, CI, or repository checks to get a change through (see the
[repository guide](repository.md#lints-formatting-and-unsafe)).

4. **Document.** Update the architecture overview, glossary, and crate docs. Set the design doc to
`implemented`. Empty the branch notes in `docs/wip/`.

5. **Merge.** Mark the PR ready for review. It goes through CI, an agent review pass, and human
review, and is then squash-merged.

## Testing

We make use of test-driven development for this repository, so tests come first for every change
except minor ones. Never weaken, delete, or `#[ignore]` an approved test to make it pass. If a test
looks wrong, stop and raise it.

Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the module they test. Integration
tests, which use only a crate's public API, live in the crate's `tests/` directory. Examples in doc
comments run as doctests, so they have to stay correct. Long-running checks (extended property
tests, fuzzing, benchmarks, and mutation testing) belong in the extended workflow, not on every PR.

The following recipes are used to run tests. Note that nextest runs each test in its own process, so
tests can't rely on state shared within a process.

| Command | Runs |
|---|---|
| `just test` | All tests with `cargo nextest`, then doctests with `cargo test --doc` |
| `cargo nextest run -p <crate>` | One crate's tests |
| `cargo nextest run <filter>` | Tests whose names match a filter |

### Test techniques

Apply the following testing techniques as code that needs it is added to the repository.

- **Property tests** (`proptest`) for the math library: algebraic laws, round-trips, and invariants.
  A property is a compact, reviewable spec that's hard to satisfy by special-casing.
- **Snapshot tests** (`insta`) compare structured output, such as evaluated graphs, serialized
  projects, and exports, against approved copies stored in the repository. Updating a snapshot
  changes what the test expects, so review each update with `cargo insta review` like any other test
  change. Never accept snapshots in bulk.
- **Format compatibility corpus.** Fixture files for every released serialization version, with
  tests proving they still load. Add fixtures when a format version ships, since they can't be
  recreated later.
- **Golden images** compare rendered output against approved reference images, within a tolerance.
  Start using them once rendering stabilizes.
- **Fuzzing** (`cargo-fuzz`) for deserializers and parsers.
- **Benchmarks** for core computations.
- **Mutation testing** (`cargo-mutants`), run periodically, checks that the tests catch broken code.

## When to request clarification

As a non-comprehensive baseline, stop and ask a human developer when:

- the issue or design doc is ambiguous, or the work is growing beyond its scope;
- code disagrees with an ADR, the architecture overview, or an accepted design doc;
- an approved test looks wrong; or
- the change seems to need a new architecture decision, a new dependency, or `unsafe` code.

## Branch notes

`docs/wip/` holds a branch's working notes (implementation plan, progress, findings), committed on
the branch so a task can continue across sessions. Before merge, move each kind of content to its
permanent home and empty the directory. `just ci` enforces that the branch notes directory is empty
before merging to main.

| Content | Destination |
|---|---|
| Plan and progress log | Deleted; the PR description summarizes the outcome |
| Out-of-scope bug or improvement | New `needs-triage` issue |
| Question needing a decision | The PR, or a `kind/question` issue if it outlives the PR |
| Dead end or rejected approach | Alternatives considered, in the design doc |
| Context about specific code | A comment at that code |
| Idea for later | An issue, or a draft item in the Project |
| How the system works | Architecture overview or crate docs |

If branch notes content is created without a clear home, ask the human developer what to do with it.

## Pull requests

PRs are squash-merged, and the PR title and description become the commit on `main`, so write
them for someone reading the history later.

- **Title:** [Conventional Commits](https://www.conventionalcommits.org/), scoped by crate where one
  applies, as in `feat(mw-graph): validate wire types on connect`. Common types are `feat`, `fix`,
  `docs`, `test`, `refactor`, `perf`, `ci`, and `chore`. Mark breaking changes with `!`.
- **Description:** follow the [PR template](../../.github/pull_request_template.md): what changed
  and why with `Closes #N`, how a reviewer should order their review, how the tests cover the
  acceptance criteria and which living docs changed, and any new dependency or `unsafe` code with
  its justification. Leave out the last two sections when they don't apply.
- **Draft until done.** Drafts skip CI. Mark the PR ready once the definition of done is met, and CI
  runs.

Branch protection on `main` requires CI to pass on a branch that's up to date with `main`, and keeps
history linear.

## Definition of done

- `just ci` passes.
- Public items are documented.
- Tests cover the acceptance criteria.
- Living docs are updated: the architecture overview, glossary, crate docs, and guides, as the
  change requires.
- `docs/wip/` is empty.
- The design doc status is set (major changes).
- Any new dependency or `unsafe` code is justified in the PR.

The `pre-pr` skill walks through this list before a PR is marked ready.

## Releases

Releases are tags on `main` (`vX.Y.Z`) with GitHub Releases, and release notes are generated from
the commit history. Run the extended workflow manually before tagging.
