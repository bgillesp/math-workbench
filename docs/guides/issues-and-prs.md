# Issues and Pull Requests

> Seed: carried over from earlier guides, for a human developer to rewrite.

## Issues

An issue records a problem to handle later: something that doesn't work, or a capability that's
missing. The bug and feature forms in `.github/ISSUE_TEMPLATE/` ask for what's needed, and apply the
`needs-triage` label described in the [development guide](development.md#issues-and-the-project).

## Pull request titles

The PR title becomes the squash commit's subject, so it follows
[Conventional Commits](https://www.conventionalcommits.org/), scoped by crate where one applies, as
in `feat(mw-graph): validate wire types on connect`. Common types are `feat`, `fix`, `docs`,
`test`, `refactor`, `perf`, `ci`, and `chore`. Mark breaking changes with `!`.

## Pull request descriptions

The description is written for reviewers, following the
[PR template](../../.github/pull_request_template.md): what changed and why, what a reviewer should
look at first and anything surprising, and any new dependency, including one between workspace
crates, or `unsafe` code, each with its justification. Add `Closes #N` when the PR resolves an
issue, and leave out sections that don't apply.

## Commit messages

The squash commit gets its own message, written when the PR is merged, for someone reading
`git log` later. Its subject is the PR title followed by the PR number, as in
`feat(mw-graph): validate wire types on connect (#42)`. Its body is plain text wrapped at 72
columns: a short summary of what the change does, calling out anything unusual that a reader
scanning the log should notice, such as a new dependency, `unsafe` code, a breaking change, or a
setting changed outside the diff. Inline code is fine, but leave out headings, links, and the
review details that belong in the PR description. A human developer writes or approves the message
when merging.
