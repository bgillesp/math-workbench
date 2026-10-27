# Architecture and Design Guide

The `docs/` subdirectory of the repository holds natural language descriptions of various aspects of
the planning and design of the repository.  These documents are held in a structured way within the
repository so that elements of architecture and "institutional knowledge" about the repository that
are often held in the heads of a core development team or in email threads or scattered documents,
are made fully available as context for agents engaging with the codebase.

Knowledge is represented in a few main ways:
- High-level overview representations: `docs/architecture/overview.md` for a living architecture
  summary, `docs/glossary.md` for terminology and naming conventions, and
  `cargo xtask adr list` for a listing of the architecture decisions in force.
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
creating a superseding ADR.  ADRs are approved and merged in their own PR, and merging the PR
accepts the ADR. A rejected proposal isn't merged, since its closed PR is the record. Apart from
typo fixes, an accepted ADR is never edited. Agents draft a new ADR only when a human developer
asks for one.

Examples of such decisions are the wire type system and serialization versioning. A question that's
still open stays in an issue until it's decided, and a choice that's local or easy to reverse
belongs in a design doc.

ADRs live in `docs/architecture/decisions/`: the ADRs in force in `active/`, and the ones a later
ADR has replaced in `superseded/`, kept for reference. Each is a file named like
`0003-short-title.md`, numbered in sequence across both directories. Whether an ADR is in force
shows in its path, so nothing inside it changes when it's superseded. Its front matter is written
once, when it's created:

```yaml
---
id: ADR-0003
title: "Wire types: checked on connect"
summary: "Every wire has a static type, and connecting two ports checks compatibility, so a
  mistyped connection is rejected before anything is evaluated."
date: 2026-11-02
supersedes: ADR-0001
---
```

The `title` and `summary` are double-quoted, and the summary, a sentence or two of at most 300
characters, may continue on indented lines. `supersedes` is there only for an ADR that replaces
earlier ones.

Tags say broadly what each ADR affects. They live in `decisions/tags.yaml`, one entry per ADR,
sorted by id, as in `ADR-0003: graph, wires`, and a tag is lowercase letters, digits, and hyphens.
Unlike the ADRs, `tags.yaml` is a living file: tags can be added, renamed, or merged in any PR as
the architecture grows, and a superseded ADR keeps its tags. There's no list of allowed tags, so
reuse an existing tag where one fits.

To write an ADR:

1. Run `cargo xtask adr new "<title>" --tags=<tags>`, adding `--supersedes=<N>,...` to replace
   earlier ADRs. It writes `active/NNNN-slug.md` from the [template](../templates/adr.md), adds
   the entry to `tags.yaml`, and moves the ADRs it supersedes to `superseded/`. Moving them
   doesn't count as editing them. It also notes any tag that no other ADR uses, so near-duplicates
   such as `graph` and `graphs` don't creep in.
2. Write the summary and the body. A superseding ADR restates whatever still holds from the ADRs
   it replaces, so the current decision can be read from one document.
3. Open a PR.

`cargo xtask check-docs` checks the file names, the front matter, the supersede links, and
`tags.yaml`. Until the summary is written, it reports it as empty. Two branches that each add an
ADR take the same number, so the second to merge fails the check once it's up to date with `main`,
and its ADR is renumbered by hand.

`cargo xtask adr` also prints views of the ADRs:

- `adr list` lists the ADRs in force by number, with their tags. `--all` adds the superseded ones,
  `--tags` groups them by tag, `--tags=graph,wires` groups them under just those tags, and
  `--verbose` adds each summary.
- `adr tags` lists each tag, with the number of ADRs in force that use it.
- `adr print <N>...` prints ADRs in full, with whether each is in force.

ADRs and design docs cite ADRs by id in prose, as in `ADR-0003`, rather than with links, since a
link breaks when its target moves to `superseded/`, and fixing it would mean editing a frozen
document. Living docs may link to ADR files. The link check catches a link that a move breaks, and
the fix is usually to point at the successor. The template's heading, `# ADR-NNNN: Title`, keeps
ADRs consistent, though it isn't checked, and neither are citations.

## Design Documents

Development work which implements a major feature or codebase change begins with the creation and
approval of a design document which elaborates on the details and decisions of the design to be
implemented, and defines the acceptance criteria for the resulting code changes, from which concrete
tests are derived.  These design documents provide significant context outside of the code changes
themselves which may be useful to know for future development in the same region of the codebase,
and so are preserved for reference.  Design documents are numbered by the PR that implements them,
and are revised until it merges so that they describe the problem and the solution as built. They're
frozen from then on.
