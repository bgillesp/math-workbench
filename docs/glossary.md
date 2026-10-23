# Glossary

Terms with a specific meaning in this project, especially ones that are easy to confuse. Add a
term when it first appears in code or docs, and update its entry in the same PR when its meaning
changes.

## Domain

**Tile.** A unit of the computation network. A tile computes, renders, or responds to
interaction, or some combination of these, and connects to other tiles through wires.

**Wire.** A strongly typed connection that carries values from one tile's output to another
tile's input. Wire types come from the math library's type system.

## Repository

**ADR.** An architecture decision record: one hard-to-reverse decision, with its context,
alternatives, and consequences. Cited as `ADR-0003`. See the
[index](architecture/decisions/README.md).

**Branch notes.** Working notes in `docs/wip/`, kept only on a branch and moved to permanent homes
before merge.

**Change tier.** How much process a change needs: minor, standard, or major. See the
[development guide](guides/development.md).

**Design doc.** The design for one major change, accepted before implementation, revised until
its PR merges, and frozen after. Named by PR number, as in
`docs/design/0042-typed-wire-validation.md`.
