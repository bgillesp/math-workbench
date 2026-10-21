# Architecture overview

The system as it currently stands: a codemap with one section per crate, and the invariants that
span crates. It's the first stop for getting oriented. Update it in the same PR as any change it
describes. The reasoning behind the structure lives in the [ADRs](decisions/README.md).

## Codemap

Crates live in `crates/`, one directory each, and are listed here by level, lowest first.

### `mw-math` (level 0)

A placeholder for the math library, created during the repository bootstrap so the checks have a
crate to run on. Its scope is set by the crate-structure ADR.

### `xtask`

Repository checks, run through `just`: dependency levels, docs consistency, and branch notes.
It's tooling rather than part of the application, so it has no level. Its dependencies stay
minimal, with `lexopt` rather than `clap` for arguments, so it builds quickly however large the
application's dependency tree grows.

## Invariants

**Dependency levels.** Each crate in `crates/` declares `[package.metadata.mw] level = N` and
depends only on crates at strictly lower levels, counting dev- and build-dependencies.
Workspace crates outside `crates/`, such as `xtask`, have no level, so nothing in `crates/` may
depend on them. `cargo xtask check-levels` enforces both rules.

**Unsafe code.** `unsafe` is denied workspace-wide. A module that needs it, such as FFI or GPU
code, opts out locally with a justification, and every `unsafe` block documents why it's sound.
