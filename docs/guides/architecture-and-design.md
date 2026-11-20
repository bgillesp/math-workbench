# Architecture and Design Guide

The `docs/` subdirectory of the repository holds natural language descriptions of various aspects of
the planning and design of the repository.  These documents are held in a structured way within the
repository so that elements of architecture and "institutional knowledge" about the repository that
are often held in the heads of a core development team or in email threads or scattered documents,
are made fully available as context for agents engaging with the codebase.

Knowledge is represented in a few main ways:
- High-level overview representations: `docs/architecture/overview.md` for a living architecture
  summary, `docs/glossary.md` for terminology and naming conventions, and
  `docs/architecture/decisions/active/` for the architecture decisions in force.
- Architecture decision records (ADRs): documents in `docs/architecture/decisions/` representing
  significant decisions about the system design which have broad downstream consequences for
  development.
- Design documents: feature-level design documents created during development of individual
  features, recorded in `docs/design/`.
- User guides: documents like this one which summarize some aspect of interacting with the codebase,
  found in `docs/guides/`.
- Branch notes: context recorded in `docs/wip/` during development, used especially to pass context
  between agent sessions and to record observations or temporary artifacts from the work. These
  files are transient, and are never merged into main.

For full details on the structure and conventions of the `docs/` directory, see `docs/README.md`.

## Architecture Decision Records

These documents record high-level decisions with a significant impact on downstream work.  Such
decisions call for careful consideration, and the thought process and outcome of such decisions is
recorded in an Architecture Decision Record, or ADR.  At any time, the project will have some set of
ADRs which are active over the system design, and a decision is changed by a new ADR, or retired by
moving its ADR to `inactive/`.  A human developer approves an ADR in its PR. A rejected proposal
isn't merged, since its closed PR is the record. An approved ADR's decision isn't rewritten, though
typos and links can be fixed.

Examples of such decisions are the wire type system and serialization versioning. A question that's
still open stays in an issue until it's decided, and a choice that's local or easy to reverse
belongs in a design doc.

ADRs live in `docs/architecture/decisions/`: the ones in force in `active/`, and retired ones in
`inactive/`, whether or not a later ADR replaced them. Each is a file named like
`0003-short-title.md`, with the id `ADR-0003`, numbered in sequence across both directories. Two
branches that each add an ADR can take the same number, and the second to merge renumbers its ADR
by hand.

An ADR that replaces earlier ones moves them to `inactive/` with `git mv`, names them in its
`Supersedes:` line, and restates whatever still holds from them, so the current decision can be
read from one document. To write an ADR, copy the [template](../templates/adr.md).

ADRs are cited by id in prose, as in `ADR-0003`, so citations survive a move to `inactive/`.
Architecture docs and design docs cite the ADRs they rest on. No tooling checks ADRs, so these
conventions are kept by hand.

## Design Documents

Development work which implements a major feature or codebase change may call for a design
document which elaborates on the details and decisions of the design to be implemented.  These
design documents provide significant context outside of the code changes themselves which may be
useful to know for future development in the same region of the codebase, and so are preserved for
reference.

Whether a change calls for one is up to whoever makes the change. A design doc is named
`docs/design/YYYY-MM-DD-<topic>.md`, dated the day it was first written. There's no template,
front matter, or status, so a doc takes whatever shape suits the task.

A design doc changes freely during development, and before its PR merges it reflects the decisions
made. A design whose work isn't finished, such as one that lands across several PRs, opens with a
short note on what's happening, and the note is removed once the work is done. After that the doc
is left as a record, though typos and links can still be fixed, and a later change to the same
area gets a new dated doc, which can link the old one.
