# Plumb v0.28.0

A guard runs the same checks whatever moved. In this repository, ninety-six of
every hundred commits pay for a package install, a formatter, a type check, a
test run and a site build that read nothing that changed. The lane cannot know
that. Only a declaration can say which paths decide a step's outcome.

## A repository declares what each step reads

`[workflow.hash]` names, per key, the paths whose recorded bytes decide that
step. A key names its lane and its own namespace on either side of one slash,
because lane names are Plumb's closed enumeration and everything after the slash
is the product's to divide: the nesting under `[workflow.hash.ship.cargo]` reads
back as `ship/cargo.rehearse`.

The input covers the declared roots themselves as well as the bytes beneath
them, so widening a declaration invalidates its own key rather than keeping a
stale answer, and it measures what Git recorded rather than what lies in the
working seat.

Two prefixes keep a declaration from collapsing into a wildcard. `key://`
composes one key's inputs into another's, so a step reading what two others read
says that instead of `*`. `suite://` names a classic toolchain inventory living
in Plumb's rules beside every other number a mechanism must honour, so a product
declares intent rather than copying a list, and adding a path to a suite is a
change to Plumb. Cycles and unknown lanes, keys or suites refuse.

## A lock says whether the guard already saw that input

The record cannot live in the repository: only a run that passed may write one,
and a run may not write to the tree it is judging. One seat holds every
repository's locks, and the repository names its own room inside it, because two
products sharing a key name would otherwise push each other out and neither
would ever hold.

The room comes from the run, so a lock cannot be read or written where no run
named a seat. What would otherwise be a rule about who may record one is instead
a thing only a run can do: what passed on somebody's machine can no longer be
offered to the guard as something the guard already saw.

## A rendered guard asks once, and each proof is its own step

One step asks for every declared key at once and each key renders as its own
named step holding one condition. A key spanning several steps holds them under
that one answer and records its lock in the last of them, because skipping a
package install while running the type check needing its modules fails for a
reason no declaration described.

Whether a declared key's input moved joins `runner.os` and the resolved channel
as a condition a rendered step may hold. A lane is written once and read by every
commit after it, so the hash it turns on is one render time cannot know, which is
why that list was closed rather than empty.

The condition is written so that absence runs the step. An answer that never
arrived, a tool too old to know the question, a lock nobody could read: each
leaves the output empty, and an empty output is not `false`. Written the other
way round, any outage would have become a guard that passed what it never
checked.

## `plumb workflow status`

`status` prints each key's digest with what every declared entry contributed, so
a short declaration loses no information, and `--since` replays the declaration
across history and reports how often each key would have held. That is how the
shape gets tuned rather than guessed: the first declaration written here was
wrong, and the replay is what showed it.

## Smaller truths

`cargo test` now runs after the release check, so the three proofs reading only
what Cargo reads sit together. That line moves in every governed repository
whether or not it declares anything.

A repository whose product is the tool installs the released tool beside its own
build, because asking whether a step may be skipped must not first compile the
thing that would have run it.

## Stable promotion takes the latest exact seal at the commit

An exact release that published and then failed downstream can never be run
again. A run advances its channel pointer, and the seal records the generator
that pointer named, so the second attempt computes a seal the first already made
immutable. Burning the number is the only way forward.

Promotion therefore takes the newest published exact seal standing at the frozen
commit instead of refusing when more than one does. A commit carrying none still
refuses, because a stable release must come from an exact one that ran.
