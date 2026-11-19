Math Workbench is a Rust desktop application for working with mathematical objects in a dynamic,
graphical environment: tiles that compute, display, and respond to interaction, connected by typed
wires. It aims for efficient core computations, an ergonomic interface, and publication-quality
visual output.

You're working with a human developer, who sets direction, approves architecture decisions, and
reviews PRs before merging to main. Development uses GitHub tooling: Issues record problems to
solve later, and review takes place in a Pull Request.  Work doesn't need an Issue to start.
Planning takes place in a GitHub Project managed by a human developer.

## Repository design

The repository aims to be a clean and legible Rust project targeting efficient operation for agentic
coding, via: tool-driven validation processes, in-repository documentation for system design and
architecture, and concise style guides.  Code style is concise and polished, idiomatic, and
well-documented, including a moderate degree of doc comments for non-public definitions, and of
inline code comments.

Common commands are exposed via the `just` command-line utility.  `just check` runs primary codebase
evaluations for development, `just fmt` formats code (run it before committing), and
`just test` runs the test suite.  `just ci` runs the full workflow required for passing CI -- a
small extension of `just check`.  Use `just --list` to show all available recipes.

The most important elements of the repository are:
- Application crates with a flat directory structure in `crates/mw-*` directories.
- An additional utility crate in `xtask/` provides tools for repository maintenance, written in Rust
  for cross-platform compatibility.  A Cargo alias exposes these tools via `cargo xtask` commands.
- Various natural language documentation in `docs/`, including architecture descriptions and formal
  design choices, feature design docs, and guides for working with the codebase.

## Application overview

The application provides an interactive graphical environment for working with mathematical objects
using a polished and ergonomic interface, arranging computational and interactive tiles in multiple
2D layers, with connections by wires carrying outputs of a tile to inputs of another.  The design
concept is "Factorio for mathematics", a game engine for the mathematical sciences.  The primary
design elements are: optimized computation over an extensive and extensible mathematics library; a
smooth cross-platform interactive graphics experience; consistent and professional graphics output;
and a highly ergonomic interface.

The application is currently in early development, so there is no content yet.  Major architecture
components will include:
- An abstracted interface for graphics and UI elements, decoupling platform-specific code from the
  rest of the application.
- An optimized library of math primitives.
- Compatibility layers for some major external math libraries, and support for dynamic scripting.
- A standard API for computational and interactive elements, allowing users to extend application
  functionality without requiring too much familiarity with the codebase.
- Export functionality to still and animated graphics formats.

A more concrete summary will appear here once development begins in earnest.

## Resources

More extensive guides are available for the following categories of tasks:
- **Development.** See [Development Guide](docs/guides/development.md) for how the repository is
  used: Issues, branches, PRs, tests, and the merge checklist.
- **Style.** See [Issues and Pull Requests](docs/guides/issues-and-prs.md) for writing issues, PRs,
  and commit messages, and [Code and Test Style](docs/guides/code-and-tests.md) for code and tests.
- **Architecture and Design Work.** See [Architecture and Design
  Guide](docs/guides/architecture-and-design.md) for how the design docs are structured and
  organized. The docs provide high-level context for the present design, including Architecture
  Decision Records (ADRs) and feature design documents.
- **Repository and Infrastructure.** See [Repository Design Reference](docs/guides/repository.md)
  for full details on the design and structure of the repository and connected infrastructure,
  including notes on agent setup.
