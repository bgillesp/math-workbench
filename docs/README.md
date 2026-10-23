# Documentation

Each kind of fact has one home. This page says where each one lives, how each kind of document
changes, and what to read before different kinds of work.

## What lives where

| Kind | Location | Changes | Sign-off |
|---|---|---|---|
| Vision | `docs/vision.md` | Rarely | Human developer |
| Roadmap, priorities | GitHub Projects and Issues | Continuously | Human developer |
| Decision records (ADRs) | [`architecture/decisions/`](architecture/decisions/README.md) | Never edited once accepted; replaced by a new ADR | Human developer; agents draft only on request |
| Design docs | [`design/`](design/) | Revised until the PR that implements them merges, then frozen | Human developer, before implementation |
| Architecture overview | [`architecture/overview.md`](architecture/overview.md) | Updated in the PR that changes behavior | PR review |
| Glossary | [`glossary.md`](glossary.md) | Living | PR review |
| Guides | [`guides/`](guides/) | Living | PR review |
| API docs | `//!` and `///` in source | Living; `missing_docs` enforces coverage | PR review |
| Templates | [`templates/`](templates/) | Living | PR review |
| Branch notes | `wip/` | Branch only; removed before merge | None |

When code disagrees with an ADR, the architecture overview, or an accepted design doc, stop and
report the mismatch rather than deciding which one is right.

## Statuses

ADRs and design docs keep their status in front matter at the top of the file, and
`cargo xtask check-docs` validates it.

An ADR is `proposed`, `accepted`, `rejected`, `superseded by ADR-NNNN`, or `deprecated` (no longer
applies and has no replacement). An ADR that replaces another also says `supersedes: ADR-NNNN`.
Apart from typo and link fixes, the only edit an accepted ADR ever gets is the status change when
it's superseded.

A design doc is `draft`, `accepted` (approved for implementation), or `implemented` (describes
what was built, set before merge). A design that isn't built never merges, since its closed PR is
the record, so every design doc on `main` is `implemented`. `cargo xtask check-ready` checks this in
`just ci`.

## Read this before…

| When you're… | Read |
|---|---|
| Getting oriented | The [architecture overview](architecture/overview.md), then the [glossary](glossary.md) |
| Starting any change | The [development guide](guides/development.md) |
| Changing the checks, CI, or agent tooling | The [repository guide](guides/repository.md) |
| Writing or changing tests | The [development guide](guides/development.md#testing) |
| Changing a crate | Its `lib.rs` docs, and the [ADRs in force](architecture/decisions/README.md) for its area |
| Planning a major change | The [design doc template](templates/design-doc.md) and the ADRs in force |
| Proposing an architecture decision | The [ADR index](architecture/decisions/README.md) and [template](templates/adr.md) |
| Continuing work on a branch | Its branch notes in `wip/` |
