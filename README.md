# Math Workbench

Math Workbench is a desktop application for working with mathematical objects in a dynamic and
graphical environment. Objects live in a computation network of tiles connected by typed wires,
where they can be explored interactively and exported as figures and animations.

The project is in early development, and there's nothing to run yet.

## Building

You need [rustup](https://rustup.rs) and [just](https://just.systems). The Rust toolchain is
pinned in `rust-toolchain.toml` and installs automatically.

```sh
just setup   # install the cargo tools the checks use
just check   # format check, lints, tests, docs, and repository checks
```

Development works on Linux, macOS, and Windows. On Windows, enable Developer Mode and clone with
`git clone -c core.symlinks=true`, so the one symlink in the repository checks out correctly.

## Contributing

[docs/README.md](docs/README.md) explains how the repository and its docs are organized, and the
[workflow guide](docs/guides/workflow.md) covers how changes are made. Coding agents start from
[AGENTS.md](AGENTS.md).

## License

Licensed under the [Apache License, Version 2.0](LICENSE).
