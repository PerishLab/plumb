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
