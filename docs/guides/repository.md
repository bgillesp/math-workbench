# Repository Guide

How the repository maintains code quality and consistency: its layout, the Cargo workspace and its
conventions, the checks behind `just ci`, CI and the GitHub settings around it, dependency policy,
and agent tooling. Read it before changing any of these, or when a check fails and the reason
isn't clear. The [conventions design doc](../design/0002-repository-conventions.md) records the
reasoning as first planned. This guide describes the setup as it stands, and takes precedence
where the two differ.

Two principles run through the setup. Anything that must hold is checked by `just ci`, and CI runs
the same command, so a branch that passes locally passes in CI. Hooks and skills are conveniences
on top, never the only enforcement. And development works on Linux, macOS, and Windows, so recipes
stay short calls into `cargo` or `xtask`, and anything with real logic is written in Rust in
`xtask` rather than as a shell script.

## Layout

```
.
├── AGENTS.md                 # agent entry point
├── CLAUDE.md                 # imports AGENTS.md for Claude Code
├── README.md                 # human entry point
├── LICENSE                   # Apache-2.0
├── Cargo.toml                # virtual workspace: shared settings, dependency versions, lints
├── Cargo.lock                # committed
├── justfile                  # every command; `just ci` is exactly what CI runs
├── rust-toolchain.toml       # pinned toolchain and components
├── rustfmt.toml
├── deny.toml                 # cargo-deny: advisories, licenses, bans, sources
├── .editorconfig             # UTF-8, LF, final newline, 4-space indent (2 for YAML and JSON)
├── .gitattributes            # LF line endings in the repository
├── .gitignore                # build output, local agent settings, pending snapshots
├── .cargo/config.toml        # the `cargo xtask` alias
├── .agents/skills/           # skills, one directory each
├── .claude/
│   ├── settings.json         # permissions, the format hook, and attribution
│   └── skills -> ../.agents/skills
├── .codex/hooks.json         # the format hook
├── .github/
│   ├── workflows/            # ci.yml, audit.yml, extended.yml
│   ├── dependabot.yml
│   ├── pull_request_template.md
│   └── ISSUE_TEMPLATE/       # bug and feature forms
├── crates/mw-<name>/         # the application's crates
├── xtask/                    # repository checks
└── docs/                     # see docs/README.md
```

## Commands

Every command is a recipe in the [justfile](../../justfile).

| Recipe | Runs |
|---|---|
| `just setup` | Installs `cargo-nextest`, `cargo-deny`, and `cargo-insta`, then `cargo xtask doctor` |
| `just fmt` | `cargo fmt` |
| `just fmt-check` | `cargo fmt --check` |
| `just lint` | Clippy on all targets, with `-D warnings` |
| `just test` | `cargo nextest run`, then doctests with `cargo test --doc` |
| `just doc` | `cargo doc --no-deps`, with `RUSTDOCFLAGS="-D warnings"` |
| `just deny` | `cargo deny check bans licenses sources` |
| `just audit` | `cargo deny check advisories` |
| `just check-repo` | `cargo xtask check-docs` |
| `just check` | `fmt-check`, `lint`, `test`, `doc`, `deny`, and `check-repo` |
| `just ci` | `check`, then `cargo xtask check-ready` |

Recipes cover the whole workspace with all features enabled. `just check` leaves out
`check-ready`, since a branch in progress is expected to have branch notes and an unfinished
design doc, and neither `check` nor `ci` runs `just audit`, which has its own workflow (see
[CI](#ci)). On Windows, `just` runs recipes in PowerShell.

## Cargo workspace

The root `Cargo.toml` is a virtual workspace whose members are `crates/*` and `xtask`. It holds
everything the crates share: package settings under `[workspace.package]` (version, edition 2024,
`rust-version`, license, repository, and `publish = false`), every dependency version under
`[workspace.dependencies]`, including path entries for the workspace's own crates, and the lint
baseline. Crates inherit these with `key.workspace = true` and `[lints] workspace = true`, and
never declare a dependency version of their own.

Crates sit side by side in `crates/`, each directory named like its crate, with the common prefix
`mw-`. The dependency graph carries the hierarchy, so directories don't nest. Each crate's
`lib.rs` opens with `//!` docs on its purpose, and the
[architecture overview](../architecture/overview.md) has a section for each crate.

The toolchain is pinned in `rust-toolchain.toml`, with the `rustfmt` and `clippy` components, and
rustup installs it on first use. `rust-version` matches it. As an application, the project only
needs to build with its pinned toolchain, so there's no older minimum supported version to test.
Bump the toolchain by hand, in its own PR.

### Adding a crate

A new crate is a major change, so it follows an accepted design doc.

1. Create `crates/mw-<name>/` with a manifest modeled on `crates/mw-math/Cargo.toml`: the
   inherited package settings, a `description`, and `[lints] workspace = true`.
2. Open `src/lib.rs` with `//!` docs on the crate's purpose.
3. Add a path entry to `[workspace.dependencies]` in the root manifest, so other crates can
   depend on it with `mw-<name>.workspace = true`.
4. Add the crate to the architecture overview's codemap, after the crates it depends on.

A human developer creates the crate's `area/` label.

### Dependencies between crates

The crate-structure ADR will describe how the workspace is organized. A new dependency of one
workspace crate on another is justified in its PR, like an external one. A rule about which crates
may depend on what is enforced only once it matters: `cargo-deny` can restrict an external crate
to named dependents with `wrappers` in `deny.toml`, and a small `xtask` check can cover a rule
between workspace crates.

## Lints, formatting, and `unsafe`

The lint baseline in the root manifest sets `missing_docs` and clippy's `pedantic` group to
`warn`, and `just lint` and `just doc` turn every warning into an error. Fix warnings rather than
allowing them. A pedantic lint that proves too noisy can be allowed workspace-wide in the root
manifest, with a one-line reason beside it.

`unsafe_code` is `deny` rather than `forbid`, so a module that needs `unsafe`, such as FFI or GPU
code, can opt out locally with `#[allow(unsafe_code)]` and a comment justifying it. Clippy's
`undocumented_unsafe_blocks` requires a `// SAFETY:` comment on every `unsafe` block saying why
it's sound, and new `unsafe` code is justified in its PR.

Formatting follows `rustfmt.toml`. The agent hooks run `just fmt` after each edit, and
`just fmt-check` fails CI on unformatted code. `.editorconfig` and `.gitattributes` keep
whitespace and line endings consistent in every other file.

## Repository checks

`xtask` is a binary crate of repository checks, run as `cargo xtask <command>` through the alias
in `.cargo/config.toml`, usually from a recipe. `cargo xtask --help` lists its commands.

| Command | Checks | Run by |
|---|---|---|
| `check-docs` | ADRs, design docs, and relative links | `just check-repo` |
| `check-ready` | `docs/wip/` is empty or missing, and every design doc is `implemented` | `just ci` |
| `doctor` | Missing tools, or a broken skills symlink | `just setup` |

Each check prints an `error:` line for each problem, naming the file or crate to fix, and exits
with a failure status if it found any. `doctor` only warns, so `just setup` always finishes.

`check-docs` covers three things:

- ADRs in `docs/architecture/decisions/`: file names like `0001-short-title.md`, unique numbers,
  the `# ADR-NNNN: ` title line, statuses, and supersede links in both directions. The index in
  that directory's `README.md` must list every ADR, with exactly the accepted ones under
  "In force".
- Design docs in `docs/design/`: file names, unique numbers, and statuses.
- Every Markdown file that git tracks or would track: relative links and images must point to
  files in the repository. A path starting with `/` resolves from the repository root. Links with
  a scheme, such as `https:`, are skipped, and anchors aren't checked.

The statuses are listed in [docs/README.md](../README.md#statuses).

Each command is a module in `xtask/src/`, listed in `Command::ALL` in `main.rs`, which also
generates the usage text. A check keeps its file and process I/O in a `run` function, and its
logic in a pure `check` function that the unit tests call directly. Problems are recorded as enum
values and formatted as messages only at the end, so tests compare exact results. A new check gets
a recipe under `check-repo` if it should run while working, or joins `check-ready` if it checks
the state of a branch that's ready to merge.

`xtask` keeps its dependencies few, with `lexopt` rather than `clap` for arguments and
`pulldown-cmark` without default features, so it builds quickly however large the application's
dependency tree grows.

## CI

| Workflow | Trigger | Runs |
|---|---|---|
| `ci.yml` | Non-draft PRs to `main`, and pushes to `main` | `just ci` on Linux |
| `audit.yml` | Weekly, manually, and PRs that change `Cargo.lock` or `deny.toml` | `just audit` |
| `extended.yml` | Weekly, and manually | `just test` on macOS and Windows |

Each job installs the toolchain from `rust-toolchain.toml` and the cargo tools with
`taiki-e/install-action`, then runs a recipe, so CI never does anything a local `just` can't
reproduce. `ci.yml` and `extended.yml` cache builds with `Swatinem/rust-cache`. Workflows get
read-only repository access, and checkout doesn't persist credentials.

Draft PRs skip CI, and marking a PR ready runs it. A new push to a PR cancels its stale run. Each
commit on `main` gets a run of its own, which leaves a status on the commit and seeds the cache
that PRs restore from. The ruleset on `main` requires the job named `just ci`, so renaming that
job blocks every merge until a human developer updates the ruleset.

Advisories run in `audit.yml` rather than in `just ci`, so a newly published advisory doesn't fail
unrelated PRs. The weekly run surfaces it instead, and PRs that change dependencies are checked
before merge. `audit.yml` isn't a required check.

`extended.yml` holds slower checks that don't run on every PR. For now it builds and tests on
macOS and Windows. Run it on `main` before tagging a release, and on any branch that touches
platform-specific code, with `gh workflow run extended.yml --ref <branch>`. Extended property
tests, fuzzing, benchmarks, and mutation testing will go here once there's code for them (see the
[development guide](development.md#testing)).

## GitHub settings

These live in the repository settings rather than in files, and only a human developer changes
them.

PRs are squash-merged, and branches are deleted after merge. The default squash message is the PR
title alone, so the commit body starts empty and is written at merge time, as the
[development guide](development.md#commit-messages) describes. A ruleset on `main` requires a PR
and a passing `just ci` on a branch that's up to date with `main`, keeps history linear, and
blocks force pushes and deletion.

Workflows may use only GitHub's own actions and an allowlist, currently `taiki-e/install-action`
and `Swatinem/rust-cache`. GitHub also requires every action to be pinned to a full commit SHA,
with its version in a comment, and Dependabot keeps the pins current. Treat a new third-party
action like a new dependency: justify it in the PR, and have a human developer add it to the
allowlist under Settings → Actions → General first, or GitHub rejects the workflow run.

Dependabot alerts, secret scanning, and push protection are on. Push protection rejects any push
that contains a recognized credential. Don't bypass it: remove the secret from the branch's
commits, or ask a human developer if it looks like a false positive.

The `kind/`, `area/`, and `needs-triage` labels are described in the
[development guide](development.md). The repository is linked to the
[Math Workbench project](https://github.com/users/bgillesp/projects/1).

## Dependencies

Dependency versions live in `[workspace.dependencies]`, `Cargo.lock` is committed, and each new
dependency is justified in its PR. `cargo-deny`, configured in `deny.toml`, checks the rest:

- Licenses must be on an allowlist of permissive licenses compatible with distributing an
  Apache-2.0 application.
- Bans: two versions of one crate in the dependency graph fail the check, as do wildcard version
  requirements.
- Sources: crates come only from crates.io. Other registries and git dependencies fail the check.
- Advisories: a crate with a RustSec advisory, or a yanked version, fails `just audit`.

Each check has an exception list in `deny.toml`, such as `ignore` for advisories that don't apply
and `skip` for tolerated duplicate versions. An entry loosens the check, so it needs a reason
beside it and in the PR, and an `ignore` entry also says when to revisit it.

Dependabot opens one grouped PR a week for cargo dependencies, and one for GitHub Actions, each
labeled `kind/chore` with a `chore` title. It waits seven days after a release before proposing
it, so a malicious release is likely to be caught and yanked first. Dependabot doesn't cover the
toolchain.

## Agent tooling

The tooling is tool-neutral at its core, with a thin adapter for each harness. Claude Code is the
primary harness, and Codex is also supported.

| Concern | Source | Claude Code | Codex |
|---|---|---|---|
| Instructions | `AGENTS.md` | `CLAUDE.md` imports it with `@AGENTS.md` | Reads it directly |
| Skills | `.agents/skills/<name>/SKILL.md` | Through the `.claude/skills` symlink | Reads `.agents/skills` directly |
| Format hook | `just fmt` | `.claude/settings.json` | `.codex/hooks.json` |
| Permissions | | `.claude/settings.json` | |

Both harnesses run `just fmt` after file edits. Codex runs the hook only once it's been trusted
in Codex. When changing a hook, change both adapters. The Claude Code permissions allow `just`,
the cargo commands that build, check, and inspect, and read-only `gh` commands for issues, PRs,
and the Project, so these run without prompting. The settings also leave Claude Code's attribution
lines out of commit messages and PR descriptions. Personal settings go in
`.claude/settings.local.json`, which git ignores.

Skills live in `.agents/skills/`, one directory each. Keep each skill a thin wrapper around a
guide, so the procedure lives in docs that people read too. The `pre-pr` skill walks through the
definition of done.

The `.claude/skills` link is the only symlink in the repository. On Windows it needs Developer
Mode and `core.symlinks=true`. Without them, git checks it out as a plain file, Claude Code finds
no project skills, and `cargo xtask doctor` warns.

Add a nested `AGENTS.md` only for a crate with unusual local rules, such as unsafe GPU code.

## Changing this setup

Changes to the checks, lints, workflows, or agent tooling follow the usual workflow. Never loosen
a lint, check, or workflow to get an unrelated change through. Propose it as its own change
instead. Keep the recipes, the workflows, and the commands in `AGENTS.md` in agreement, and
update this guide in the same PR. A change that needs a repository setting, such as a newly
allowlisted action or a renamed required check, says so in its review notes, since a human
developer has to make it.
