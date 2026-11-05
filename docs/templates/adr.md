---
id: ADR-NNNN
title: ""
summary: ""
date: YYYY-MM-DD
supersedes:
---

# ADR-NNNN: Decision title

<!--
To write an ADR, work through these steps, and delete this comment last. The architecture and
design guide explains the reasoning.

1. Copy this file to docs/architecture/decisions/active/NNNN-short-title.md, and set `id` to match.
   NNNN is one more than the highest number in `cargo xtask adr list --all`, and the title part is
   lowercase letters, digits, and hyphens.
2. Fill in the rest of the front matter: the title, a summary of a sentence or two stating the
   decision in at most 300 characters, and today's date. The title and summary stay in double
   quotes, can't contain `"` or `\`, and can continue on indented lines. If this ADR replaces
   earlier ones, list their ids in `supersedes`, separated by commas, and otherwise delete that
   line.
3. Fill in the heading, with the same number and title.
4. Write the sections. A superseding ADR restates whatever still holds from the ADRs it replaces,
   so the current decision can be read from one document.
5. Move each ADR this one replaces to superseded/ with `git mv`, without changing its contents.
6. Add an entry for this ADR to tags.yaml, as in `ADR-0003: graph, wires`, keeping the entries
   sorted by id. `cargo xtask adr tags` lists the existing tags, and it's best to reuse one where
   it fits.
7. Run `cargo xtask check-docs`, which reports anything missed, and open a PR. Merging it accepts
   the ADR.
-->

## Context

The problem, the forces at play, and what's known. Link the issue or design doc where the
question came up.

## Decision

What was decided, stated so it can be followed. If this supersedes an earlier ADR, restate what
still holds so this document stands on its own.

## Alternatives considered

Each serious alternative, and why it wasn't chosen.

## Consequences

What becomes easier or harder, known risks, and any conditions under which the decision should
be revisited.
