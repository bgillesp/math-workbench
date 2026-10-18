# Testing

Tests come first for every change except minor ones. For a major change, a human developer
reviews the failing tests derived from the design doc's acceptance criteria before
implementation starts, and from then on they're the spec. Never weaken, delete, or `#[ignore]` an
approved test to make it pass. If a test looks wrong, stop and raise it.

## Running tests

| Command | Runs |
|---|---|
| `just test` | All tests with `cargo nextest`, then doctests with `cargo test --doc` |
| `cargo nextest run -p <crate>` | One crate's tests |
| `cargo nextest run <filter>` | Tests whose names match a filter |

nextest runs each test in its own process, so tests can't rely on state shared within a process.

## Where tests go

Unit tests live in a `#[cfg(test)] mod tests` at the bottom of the module they test. Integration
tests, which use only a crate's public API, live in the crate's `tests/` directory. Examples in
doc comments run as doctests, so they have to stay correct.

## Techniques

Adopt each technique when the code that needs it appears.

- **Property tests** (`proptest`) for the math library: algebraic laws, round-trips, and
  invariants. A property is a compact, reviewable spec that's hard to satisfy by special-casing.
- **Snapshot tests** (`insta`) compare structured output, such as evaluated graphs, serialized
  projects, and exports, against approved copies stored in the repository. Updating a snapshot
  changes what the test expects, so review each update with `cargo insta review` like any other
  test change. Never accept snapshots in bulk.
- **Format compatibility corpus.** Fixture files for every released serialization version, with
  tests proving they still load. Add fixtures when a format version ships, since they can't be
  recreated later.
- **Golden images** compare rendered output against approved reference images, within a
  tolerance. Start using them once rendering stabilizes.
- **Fuzzing** (`cargo-fuzz`) for deserializers and parsers.
- **Benchmarks** for core computations.
- **Mutation testing** (`cargo-mutants`), run periodically, checks that the tests catch broken
  code.

Long-running checks (extended property tests, fuzzing, benchmarks, and mutation testing) belong
in the extended workflow, not on every PR.
