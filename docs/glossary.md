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
alternatives, and consequences. Cited as `ADR-0003`. The ADRs in force are in
`architecture/decisions/active/`, and retired ones in `inactive/`.

**Branch notes.** Working notes in `docs/wip/`, kept only on a branch and removed before merge.

**Design doc.** The design for a change that calls for one, revised during development to reflect
its decisions by merge and then left as a record. It's named by the date it was first written, as
in `docs/design/2026-11-19-lighter-agent-guidelines.md`.
