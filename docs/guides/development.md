# Development Guide

How the repository is used: Issues, branches, pull requests, tests, and what has to hold before a
change merges. How a developer and their agents plan and carry out the work is up to them, as long
as the result follows the guides and passes CI.

## Issues and the Project

Issues record problems to solve later, and work doesn't need an issue to start. They carry three
types of labels:

- `kind/bug`, `kind/feature`, `kind/chore`, or `kind/question`
- `area/<crate>` for the crate a change mainly touches
- `needs-triage` until a human developer has reviewed and prioritized the issue

Planning happens in a GitHub Project, which a human developer manages. The
[issues and PRs guide](issues-and-prs.md) covers how to write an issue.

## Branches and pull requests

Each change is made on its own branch, named with a short slug with no issue number required, and
merged through a pull request. Drafts skip CI. Once the [merge checklist](#merge-checklist) holds,
mark the PR ready for review, which runs CI. PRs are squash-merged into a single commit on `main`.
Branch protection on `main` requires CI to pass on a branch that's up to date with `main`, and keeps
history linear.

The [issues and PRs guide](issues-and-prs.md) covers PR titles, descriptions, and the squash commit
message.

## Testing

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

The [code and test style guide](code-and-tests.md) covers what makes a good test here, and the
techniques the repository uses.

## Branch notes

`docs/wip/` holds a branch's working notes, committed on the branch so work can continue across
sessions. They're removed before merge, which `just ci` enforces.

## Merge checklist

- `just ci` passes.
- Living docs are updated as the change requires: the architecture overview, glossary, crate docs,
  and guides.
- A design doc, if the change has one, reflects the decisions made during development.
- `docs/wip/` is empty.
- Any new dependency, including one between workspace crates, and any `unsafe` code is justified
  in the PR.

## Releases

Releases are tags on `main` (`vX.Y.Z`) with GitHub Releases, and release notes are generated from
the commit history. Run the extended workflow manually before tagging.
