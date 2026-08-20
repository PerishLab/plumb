# Migrating to Plumb v0.29.0

## Nothing is required of a governed repository

The compiled configuration remains the permanent floor. A repository that never
syncs a depot seat behaves exactly as it did under v0.28.0, and neither depot
law reports anything against it. Upgrade at your convenience.

## What syncing buys, and what it costs

`plumb depot sync` writes the current channel version into `~/.plumb/depot/`,
and from then on this repository's lane templates and Plumb's rules come from
that seat rather than from the binary. The gain is that a change to either
reaches you without waiting for a Plumb release. The cost is that you now hold
a version, and two laws watch it.

`depot.roots-published` only ever speaks in the repository that records the
configuration roots, which today is Plumb alone. `depot.schema-supported`
speaks anywhere: it refuses when the held version declares a floor above the
running binary, which is the depot's way of saying that this configuration was
written for a Plumb you have not installed yet. The repair is to upgrade Plumb,
never to edit the seat.

Reading is local and never fetches, so a seat that has fallen behind is silent
rather than noisy. A seat that exists and cannot be read is blind, because a
floor may not silently paper over a seat someone meant to use.

`PLUMB_DEPOT_SEAT` points the loader somewhere else, which is for debugging a
candidate configuration and not for holding one.

## `plumb changelog` now reads the depot

It reports whether a version's note seat is occupied and lists what it holds.
An empty seat exits nonzero. It no longer reads `docs/CHANGELOG`, and a
governed repository needs no changelog directory to satisfy any law.

## Release notes are written after the release, not before

Compiling a release no longer proves a note, so `release compile` will not stop
for a missing or oversized one. Write the note once the version is a
distributed fact, into `.tmp/<product>/changelog/v<version>/{en,zh}/`, then run
`plumb depot changelog --version v<version>`. Publishing proves the budget,
uploads, and clears the staging seat; `--keep` retains it, and a source given
with `--from` is never removed.

A note may be rewritten later, which is the point: a hazard found after a
version shipped belongs in that version's note.

## Seals written before this version still carry a changelog proof

They are read as they always were. New seals do not carry one. Nothing in a
downstream repository needs to change for this.

## The plumb dependency may lag stable latest

`deps.first-party-stable-latest` no longer refuses a repository for depending
on a Plumb older than the live stable. A dependency Plumb cannot read at all
still refuses. If you were pinning to silence that rule, you no longer need to.
