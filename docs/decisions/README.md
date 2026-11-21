# Architecture Decision Records

These documents record high-level decisions with a significant impact on downstream work. Such
decisions call for careful consideration, and the thought process and outcome of such decisions is
recorded in an Architecture Decision Record, or ADR. At any time, the project will have some set of
ADRs which are active over the system design, and a decision is changed by a new ADR, or retired by
moving its ADR to `inactive/`. A human developer approves an ADR in its PR. A rejected proposal
isn't merged, since its closed PR is the record. An approved ADR's decision isn't rewritten, though
typos and links can be fixed.

Examples of such decisions are the wire type system and serialization versioning. A question that's
still open stays in an issue until it's decided, and a choice that's local or easy to reverse
belongs in a design doc.

ADRs live in `docs/decisions/`: the ones in force in `active/`, and retired ones in `inactive/`,
whether or not a later ADR replaced them. Each is a file named like `0003-short-title.md`, with the
id `ADR-0003`, numbered in sequence across both directories. Two branches that each add an ADR can
take the same number, and the second to merge renumbers its ADR by hand.

An ADR that replaces earlier ones moves them to `inactive/` with `git mv`, names them in its
`Supersedes:` line, and restates whatever still holds from them, so the current decision can be read
from one document. To write an ADR, copy the [template](TEMPLATE.md).

ADRs are cited by id in prose, as in `ADR-0003`, so citations survive a move to `inactive/`.
Architecture docs and design docs cite the ADRs they rest on. No tooling checks ADRs, so these
conventions are kept by hand.
