# Law detail

The pages this file summarizes live in the plumb repository under `docs/`. When
they disagree with this file, they win.

## Config cascade

Four layers in fixed precedence: default, file, environment, arguments. Later
overrides earlier, every layer above default is optional, and the default layer
is **total** — a process with nothing else still boots holding its whole policy.

`#[derive(Cascade)]` generates the machinery. `resolve(file)` folds the layers;
`resolve_with(file, over)` merges an arguments partial last. The environment
prefix is the crate name, uppercased, with `-` becoming `_`: one namespace per
binary, no invented names.

Two doors, one per external layer:

- **Files** enter by parsing. Unreadable is an error, malformed is an error,
  and neither falls back to defaults. Whether a file is required at all is the
  caller's choice — `resolve` takes an option, and `discover` walks ancestors
  to find the repo-rooted name.
- **The environment** enters through the derived key. A value that does not
  parse is an error named by its key. An empty or blank value reads as absent,
  not as an override.

The `environment` class is reserved: product code does not touch `std::env`,
`Deno.env`, or `process.env`. A direct read bypasses the cascade — the key it
names appears in no config file, obeys no precedence, and cannot be discovered
by reading policy. Tests are the granted territory.

**Vocabulary stays with the binary.** The substrate exports no listen struct,
no store struct, no backend enum. Each binary declares its own sections, and
alignment across the workshop is a norm carried in prose and, eventually, in a
shape check — not a shared type. The 0.4 substrate exported `Listen` and
`Store`; 0.5 removed them, because which backends exist is a property of the
product and a closed enum at the substrate can only be opened by the layer that
does not feel the pressure.

## Templates

`{name}` resolves against the table the caller supplies. `{{` and `}}` are the
literal braces and nothing else escapes. Unknown, unclosed, bare, and empty
forms are refusals — an unresolved template in delivered config is a misread,
and silent pass-through is what hid typos in manifests before the grammar was
one.

The substrate owns the grammar and the resolver and knows no variable. What
`{port}` or `{namespace}` mean belongs to the tool that supplies them.

## First-party dependency currency

Published dependencies owned by perish.code move under a stable-latest norm.
The compiled policy names authorities—the `@perish` JSR scope and `perish`
Cargo registry—but deliberately contains no package-version inventory.
Registry metadata remains the live authority.

A Deno import requirement and its frozen lock answer different questions.
First-party imports carry no version requirement, while the lock records the
exact code a guard reproduces. Cargo keeps native requirement spelling because
its manifest constraint and exact lock resolution are distinct concerns. In
both ecosystems, each direct published edge resolves exactly to the latest
non-prerelease, non-yanked version. Same-workspace path edges belong to the
release train and are excluded.

Plumb discovers shape offline outside doctor. Doctor reads registry metadata
with bounded retries and caches each package once, then compares the direct
lock resolution. It changes no manifest or lock. Missing or malformed
manifest, lock, or registry evidence is blind and blocks. A stale resolution
or a versioned first-party Deno declaration refuses. Transitive staleness is
routed to the direct upstream owner instead of being guessed from a consumer.

## Home and state

The data home is an ordinary cascade field: platform default, then a repo-rooted
file, then `<TOOL>_HOME`, then an explicit flag. A tool that reads `$HOME`
itself has bypassed the cascade exactly as an invented key does.

Under the home, `state/` holds machine-written records. Records are
schema-versioned, written whole by writing beside and renaming, and are not a
config surface — policy belongs in the cascade where a human authors it.

Ownership of a path a tool created requires two-sided evidence: the registry
entry and a marker inside the path, both naming the tool. One side alone is
refused and reported, never repaired. A tool that deletes what it did not
create has done worse than one that stops.

## Skills

A skill is the tool's operating brief for an agent: principles, laws, standing,
invocation — in that order, with standing carrying whether each law is
mechanized or walled. A clause enters when it governs how a repository is built
or operated, the binary cannot enforce it, and a real cost was paid for its
absence. When a clause becomes mechanized its prose demotes to the check's name.

A skill declares its own tool's law and its own binary's checks, and references
another tool's clauses rather than restating them: a restated clause in a second
repository has nobody to keep it true.

Managed skill seats are stable consensus. Install, status, and upgrade admit
stable releases only. A non-stable brief enters through `skill stage`, which
requires one exact version and a new explicit path, uses a distinct staged
marker, and never enters the global ledger. Staged ownership never enters
managed ownership.

## Stable consensus seats

Stable is a pair: the `stable` channel and the canonical release authority.
Only that pair may write a tool's default install root, default command
entrypoint, and root public manager. A custom authority is isolated even when
it calls its channel stable, and it must name an exact version.

Every non-stable channel requires an exact immutable version and explicit
install and bin paths. Effective paths are normalized through relative
segments and symlinks and must be disjoint from stable defaults and from each
other. The refusal happens before network access or filesystem mutation.

A default stable seat is a singleton. Its manager locks mutation, proves
authority-and-channel-bound root, version, and entrypoint ownership, verifies a
staged binary, switches the entrypoint atomically, and only then sweeps old
owned versions. An implicit stable-pointer selection never moves backwards; an
exact stable version is a deliberate rollback. Unowned state refuses rather
than being adopted.

## Locks

A lock declares a pair bound across a boundary no compiler crosses, stamped
with the version and hash of its last reading (`plumb.toml`). The doctor
refuses when the covered paths change or the repository cuts a version past
the affirmation.

It buys attention, not accuracy: a machine cannot check that a brief still
describes a binary, only that somebody was made to look when the question
became askable. So affirmation is never automatic — not in CI, not behind a
write flag — and the guarantee stops at version boundaries, leaving mid-version
drift uncaught until the next cut.

A pair earns a lock when both sides are genuinely bound, no checker can verify
the binding, and drift has already cost something. When a binding becomes
checkable, its lock retires.
