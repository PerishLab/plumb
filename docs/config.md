# Config

Runtime policy enters a process through one cascade. Four layers in fixed
precedence: default, file, environment, arguments. A later layer overrides
an earlier one, every layer above default is optional, and the default
layer is total — a process with nothing else still boots with its whole
policy held.

## Cascade

`#[derive(Cascade)]` on a config struct generates the machinery
(`crates/macro/src/parse.rs`); the contract lives in
`crates/lib/src/config.rs`. `resolve(file)` folds the layers in order:
start from `Default`, merge the parsed file when one is given, merge the
environment, and `resolve_with(file, over)` merges an arguments partial
last. The environment prefix is the crate name, uppercased, `-` become
`_` — one namespace per binary, no invented names.

## Doors

Two doors, one per external layer. What arrives through a door is named,
typed, and refused loudly when malformed. There is no third door.

Files enter by parsing. `load` reads the path and parses toml; an
unreadable file is `Error::Read`, a malformed file is `Error::Parse`. The
process refuses — it does not guess, and it does not silently fall back
to defaults, because a policy half-read is a policy misread. Whether a
file is required at all is the caller's choice: `resolve` takes an
`Option`, and `discover` walks ancestors to find the repo-rooted name.

The environment enters through `Cascade::env`. Keys are derived from the
prefix and the field path, values are typed by `Env::read` — a value that
does not parse is `Error::Env`, named by its key. An empty or blank value
reads as absent, not as an override.

## Environment

`environment` is the controlled class: the surface a process inherits
without declaring it — `std::env` in rust, `Deno.env` and `process.env`
elsewhere. A direct read of that surface bypasses the cascade: the key it
names appears in no config file, obeys no precedence, and cannot be
discovered by reading policy. So the class is reserved. Product code does
not touch the surface; every crossing goes through the env door, where the
key is named by the prefix and the value is typed. The same holds for the
rest of the surface: `args` reach the cascade as the last layer through
the caller's parser, not by reading `std::env::args` mid-program.

Tests are the granted territory — a test builds its terrain and may seed
the surface it reads back through the door.

The mechanized wall is ectropy's sealed `environment` syntax class. Direct
access outside granted territory is a fault whose refusal routes here.
Plumb owns the repository-shaped grants in `ectropy.toml`; ectropy executes
them without repository knowledge.

## Vocabulary

The crate exports mechanism only: the derive, the two doors, `discover`,
`rebase`. It exports no config vocabulary — no listen struct, no store
struct, no backend enum. A shape frozen here is a closed set at the one
layer that cannot open it: which backends exist, which sections a binary
carries, what a listen block holds — every one of those is the binary's
own property, expanding under pressure this crate never feels. The 0.4
`Listen` and `Store` types were that mistake, and 0.5 removes them.

Shape alignment across the ecosystem is a norm, not a type. The norm
lives in prose here and travels with the CLI as a check, the same
relation ectropy holds for syntax: a repo-rooted `<name>.toml` per
binary, section names in the file mirrored by `#[cascade(section)]`
structs the binary declares itself, and the conventional shapes —
`[listen]` as `host`/`port`/`prefix` defaulting to `127.0.0.1:3000`,
`[store]` naming the binary's own backend set — kept alike by
convention and, when the shape check lands, by the traveling CLI.
Until it lands, this section is the wall.
