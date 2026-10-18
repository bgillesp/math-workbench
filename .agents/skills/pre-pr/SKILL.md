---
name: pre-pr
description: Walk through the definition of done before a pull request is marked ready for review. Use when work on a branch is finished, before opening a PR or moving it out of draft.
---

# Pre-PR check

Go through the [definition of done](../../../docs/guides/workflow.md#definition-of-done) in
order, fixing what you can and reporting what you can't.

1. Run `just ci` and fix what it reports. Never weaken, delete, or `#[ignore]` an approved test
   to get there. If a test looks wrong, stop and ask.
2. Check that every public item added or changed in this branch has a doc comment that explains
   what it's for, not just its name restated.
3. Compare the tests against the acceptance criteria in the issue or design doc, and list any
   criterion without a test.
4. Update the living docs the change affects: the architecture overview, glossary, crate docs,
   and guides.
5. Empty `docs/wip/`, moving each note to its home using the
   [branch notes table](../../../docs/guides/workflow.md#branch-notes).
6. For a major change, set the design doc's status to `implemented`.
7. Draft the PR title in Conventional Commits form, scoped by crate, and a description with a
   summary, `Closes #N`, review notes, and a justification for any new dependency or `unsafe`
   code. Show both to the developer.
