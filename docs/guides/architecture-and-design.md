# Architecture and Design Guide

The `docs/` subdirectory of the repository holds natural language descriptions of various aspects of
the planning and design of the repository.  These documents are held in a structured way within the
repository so that elements of architecture and "institutional knowledge" about the repository that
are often held in the heads of a core development team or in email threads or scattered documents,
are made fully available as context for agents engaging with the codebase.

Knowledge is represented in a few main ways:
- High-level overview representations: `docs/architecture/overview.md` for a living architecture
  summary, `docs/glossary.md` for terminology and naming conventions, and
  `docs/architecture/decisions/README.md` for a listing of active architecture decisions.
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
ADRs which are active over the system design, and the decision of an ADR is modified or nullified by
creating a superseding ADR.  ADRs are approved and merged in their own PR. Apart from typo fixes
and the status change when it's superseded, an accepted ADR is never edited. Agents draft a new ADR
only when a human developer asks for one.

## Design Documents

Development work which implements a major feature or codebase change begins with the creation and
approval of a design document which elaborates on the details and decisions of the design to be
implemented, and defines the acceptance criteria for the resulting code changes, from which concrete
tests are derived.  These design documents provide significant context outside of the code changes
themselves which may be useful to know for future development in the same region of the codebase,
and so are preserved for reference.  Design documents are numbered by the GitHub Issue the work is
based on.
