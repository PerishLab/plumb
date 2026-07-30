# Plumb v0.18.5

## A Rust guard must exercise the release profile

`structure.guard-checks-release-profile` is mechanized. A repository
carrying a `Cargo.toml` and a guard wrapper must invoke cargo with
`--release` somewhere in that guard.

The gap it closes is a profile nobody compiles. A guard typically runs
`cargo fmt`, `cargo clippy --all-targets`, and `cargo test` — all of them
debug. Code that only type-checks under `debug_assertions` therefore ships
green and fails at the first release build, which for a library is a
downstream consumer's packaging step rather than anything in its own
repository. A library's release lane does not close this either: it
publishes crates, and `cargo publish` verifies in debug too.

Keel v0.10.0 and v0.10.1 were unbuildable in release for exactly this
reason — a `debug_assert!` whose argument called a `#[cfg(debug_assertions)]`
method, which type-checks in every profile and exists in only one. Two
releases went out before a downstream Dockerfile found it.

The rule asks for the profile, not for a full build. A `cargo check
--release` satisfies it, which is what the defect class needs: these are
type errors under a different `cfg`, and a check costs seconds where a
build costs minutes.

## What this asks of a repository

A Rust repository whose guard has no `--release` invocation becomes out of
true on upgrade. Adding one step to the guard settles it. A repository with
no `Cargo.toml` is not asked anything.
