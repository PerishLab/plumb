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

The mechanized wall (a negentropy built-in class denying `environment`
syntax outside granted territory, with its refusal routing here) is
declared law but not yet running; until it lands, this page is the wall.
