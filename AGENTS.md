Math Workbench is a Rust desktop application for working with mathematical objects in a dynamic,
graphical environment: tiles that compute, display, and respond to interaction, connected by typed
wires. Think: "Factorio for mathematics". It aims to provide an ergonomic interface, optimized core
computations, interoperability with external libraries, publication-quality visual output, and
export functionality to still and animated graphics formats. A streamlined user experience is of key
importance.

## Commands
```bash
just setup
just fmt    # Run before commits
just test
just check  # Codebase checks for use while working
just ci     # Checks used by CI; must pass before a PR is marked ready
```

## Conventions

**Code.** Code is clean, straightforward, and idiomatic. Design performance in rather than tuning it
later: estimate a hot path's cost before writing it, allocate up front and reuse buffers, and prefer
static to dynamic dispatch. Process data in batches, with functions that take a slice rather than
one item, so per-call costs are paid once per batch. Fix warnings rather than `#[allow]`ing them,
and never loosen a lint, check, or workflow to get an unrelated change through.

**Control flow.** Keep control flow simple, explicit, and centralized. A parent function owns the
branching and the state. Its helpers are short, mostly branch-free, and return what should change
rather than changing it: push `if`s up and `for`s down. Functions should rarely exceed 70 lines.
Application logic uses explicit loops instead of recursion, though math library code may recurse
where that's the natural form.

**Comments.** Comments are compressed and concise, summarizing functional intent. Don't restate what
is obvious from the code. Instead give the big picture and anything non-obvious. Document the
purpose of data structure fields and enum variants, except for types for which the purpose is
obvious without explanation. Justify the safety of any necessary `unsafe` Rust code.

**Tests.** Prefer fewer tests pinning behavior whose failure would be a real regression. Avoid
pinning exact wording, incidental output, or choices a later change could reasonably make. We use
`cargo nextest`, which runs each test in its own process so tests cannot share in-process state.

**Workspace.** The monorepo uses a Cargo workspace. All dependencies are declared in the top-level
`Cargo.toml` and inherited by individual crates. Application crates are in `crates/`, and the
`xtask` crate implements development tooling. Temporary working documents can go in `docs/wip/`,
which is checked to be empty by CI before merge to main.

**Commit messages.** Write descriptive commit messages that inform and delight the reader, because
your commit messages are being read. Use Conventional Commits, with plain text wrapped at 72
columns, no headings or links. Start with one or a few sentences that succinctly describe the
purpose of a commit in a self-contained way. A reader should get a "good enough" picture by reading
the first paragraph. Then summarize the main changes in general terms. Err on the side of concision.
When merging a PR, the squash commit's subject is the PR title plus the PR number, as in
`test(xtask): focus tests on behavior that can regress (#28)`. 

## GitHub

**Usage.** The project is hosted on GitHub, and uses Issues, PRs, and Releases. Issues are labeled
by kind with one of `kind/bug`, `kind/feature`, `kind/chore`, or `kind/question`, and with
`needs-triage` until a human developer has reviewed and prioritized them. PRs are squash-merged to
main. Releases are tags on `main` (`vX.Y.Z`) with GitHub Releases, and release notes are generated
from the commit history.

**Descriptions.** Issue and PR descriptions lead with a compressed, self-contained summary, which
should aim to provide conceptual simplicity potentially at the cost of precision or completeness.
Additional detail can come after the initial summary, but err on the side of including those details
that are interesting or useful, and describe them in summary form. For Issues, don't spend much time
on potential solutions. For PRs, not every change from the changeset needs to be represented, but do
mention new dependencies or unsafe code. PR titles follow Conventional Commits subject guidelines.

## Docs

**Architecture.** High-level details of system architecture are in `docs/architecture/overview.md`.
The document should be updated for large changes in the PR that implements them.

**ADRs.** Records of hard-to-reverse architecture decisions are in `docs/decisions/`. ADRs currently
in force are found in `active/`, and those that have been retired are in `inactive/`. See
`docs/decisions/README.md` for more detail when asked to write or change ADRs.

**Design docs.** When a change is large enough to need a dedicated design document, the doc is
committed to the repository as a historical record, saved as `docs/design/YYYY-MM-DD-<topic>.md` and
dated the day it was first written. Design documents may change freely during development. Before
the PR merges, it should be updated to reflect the decision made. If work on a design doc extends to
multiple PRs, the doc should open with a short note on what's happening, to be removed once work is
finished. Once a design doc is implemented, its contents are frozen and any later change in the same
area gets a new doc.
