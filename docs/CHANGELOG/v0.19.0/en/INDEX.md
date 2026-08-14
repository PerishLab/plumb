# Plumb v0.19.0

## The ref carries the release

A release lane no longer takes a channel and a version as inputs. It reads
`github.ref`: `refs/heads/release/vX.Y.Z` carries a stable version and
`refs/tags/vX.Y.Z-<channel>.N` an exact one, and anything else refuses. The
channel then comes from `plumb release channel`, which owns that rule alone.

`plumb ship binary dispatch` takes only `--version`. It derives the channel,
derives the ref that carries it, and sends no identity input at all. `--channel`
and `--ref` are gone with the freedom the latter granted: an exact release
binding an arbitrary branch is exactly what the tag anchor removed.

Both callers stay operator-dispatched. The identity moves from a typed string to
a selected ref, and no release follows from a push, so nothing publishes by
accident.

This also closes a second source inside the lane itself. Resolve froze the
version and the commit while build and coordinate re-read the channel input
directly; every job now reads the frozen resolve outputs.

## An absent branch is created rather than refused

`plumb stable prepare` died on a new release line with "The target couldn't be
found." The existence probe classified a missing branch by matching the error
prose of the retired vendored client, and Runseal carries none of those words.
Branch and protection now both choose on the read outcome: readable is edited,
unreadable is created, and a genuine failure surfaces from the operation that
needs it. No text crosses the product boundary.

## A watched run waits for every task

`plumb ship binary dispatch --watch` called a run successful while it was still
executing, on a release that went on to fail. Forgejo computes a run status
from the tasks that have concluded, so a run whose early tasks had finished
read as successful mid-flight. The blocked arm already walked the task list;
the success arm short-circuited without looking. It walks the same list now: a
run is successful when its status says so and no task remains in flight.

## What `ship` does not settle yet

`ship` carries `binary` and nothing else here. The adaptor set is not
enumerated in this version, and `site` still answers as a command of its own;
both arrive after it. Read the nine moved verbs as the whole of the change and
not as the shape of a finished object.

`release` still keeps `activate`, `inspect` and `registry`. They span both
objects or stand apart, they are transitional rather than settled, and each
moves when the adaptor that claims it is absorbed. Migrate what this release
moved and wire nothing around what it did not.
