# Documentation

Each kind of fact has one home. This page says where each one lives, how each kind of document
changes, and what to read before different kinds of work.

## What lives where

| Kind | Location | Changes | Sign-off |
|---|---|---|---|
| Vision | `docs/vision.md` | Rarely | Human developer |
| Decision records (ADRs) | [`architecture/decisions/`](architecture/decisions/) | Never edited once accepted; a new ADR supersedes it, and it moves to `superseded/`. Their tags are living | Human developer; agents draft only on request |
| Design docs | [`design/`](design/) | Revised during development to reflect its decisions by merge, then left as a record | PR review |
| Architecture overview | [`architecture/overview.md`](architecture/overview.md) | Updated in the PR that changes behavior | PR review |
| Glossary | [`glossary.md`](glossary.md) | Living | PR review |
| Guides | [`guides/`](guides/) | Living | PR review |
| API docs | `//!` and `///` in source | Living; `missing_docs` enforces coverage | PR review |
| Templates | [`templates/`](templates/) | Living | PR review |
| Branch notes | `wip/` | Branch only; removed before merge | None |

## Read this before…

| When you're… | Read |
|---|---|
| Getting oriented | The [architecture overview](architecture/overview.md), then the [glossary](glossary.md) |
| Starting any change | The [development guide](guides/development.md) |
| Changing the checks or CI | The [repository guide](guides/repository.md) |
| Writing or changing tests | The [code and test style guide](guides/code-and-tests.md), and the [development guide](guides/development.md#testing) |
| Writing an issue or PR | The [issues and PRs guide](guides/issues-and-prs.md) |
| Changing a crate | Its `lib.rs` docs, and the ADRs in force for its area, from `cargo xtask adr list --tags` |
| Writing a design doc | The [architecture and design guide](guides/architecture-and-design.md#design-documents), and the ADRs in force |
| Proposing an architecture decision | The [ADR workflow](guides/architecture-and-design.md#architecture-decision-records), and the ADRs in force from `cargo xtask adr list` |
| Continuing work on a branch | Its branch notes in `wip/` |
