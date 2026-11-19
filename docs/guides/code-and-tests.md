# Code and Test Style

> Seed: carried over from earlier guides and review feedback, for a human developer to rewrite.

## Code

Doc comments say what an item is for, not just its name restated. A non-trivial struct or enum
gets a short doc comment on every field and variant, so coverage is complete rather than patchy.
An obvious type, such as `Point3 { x, y, z }`, needs none, and the fields of a struct-like variant
can be covered by the variant's own comment.

Check the [glossary](../glossary.md) before naming something new. Lints, formatting, and `unsafe`
code are covered in the [repository guide](repository.md#lints-formatting-and-unsafe).

## Tests

Prefer fewer tests, each pinning behavior whose failure would be a real regression. Avoid pinning
exact wording, incidental output, or choices a later change could reasonably make.

Apply the following testing techniques as code that needs them is added to the repository.

- **Property tests** (`proptest`) for the math library: algebraic laws, round-trips, and invariants.
  A property is a compact, reviewable spec that's hard to satisfy by special-casing.
- **Snapshot tests** (`insta`) compare structured output, such as evaluated graphs, serialized
  projects, and exports, against approved copies stored in the repository. Updating a snapshot
  changes what the test expects, so review each update with `cargo insta review`.
- **Format compatibility corpus.** Fixture files for every released serialization version, with
  tests proving they still load. Add fixtures when a format version ships, since they can't be
  recreated later.
- **Golden images** compare rendered output against approved reference images, within a tolerance.
  Start using them once rendering stabilizes.
- **Fuzzing** (`cargo-fuzz`) for deserializers and parsers.
- **Benchmarks** for core computations.
- **Mutation testing** (`cargo-mutants`), run periodically, checks that the tests catch broken code.
