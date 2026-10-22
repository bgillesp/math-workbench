# Recipes stay short calls into cargo or xtask so they run in any shell.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

# List recipes
default:
    @just --list

# Install the cargo tools the checks use, then check the local setup
setup:
    cargo install --locked cargo-nextest cargo-deny cargo-insta
    cargo xtask doctor

# Format all code
fmt:
    cargo fmt --all

# Check formatting without changing files
fmt-check:
    cargo fmt --all --check

# Run clippy on all targets, with warnings as errors
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Run tests and doctests
test:
    cargo nextest run --workspace --all-features
    cargo test --workspace --all-features --doc

# Build API docs, with warnings as errors
doc $RUSTDOCFLAGS="-D warnings":
    cargo doc --workspace --all-features --no-deps

# Check dependency licenses, bans, and sources
deny:
    cargo deny check bans licenses sources

# Check dependencies against the RustSec advisory database
audit:
    cargo deny check advisories

# Check docs consistency
check-repo:
    cargo xtask check-docs

# Run every CI check except the branch-notes check; use while working
check: fmt-check lint test doc deny check-repo

# Run exactly what CI runs; must pass before a PR is ready for review
ci: check
    cargo xtask check-wip
