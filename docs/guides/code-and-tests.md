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
