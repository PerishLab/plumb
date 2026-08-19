# Migrating to Plumb v0.27.0

## Render the lanes again, before the next stable release

A lane rendered by v0.26.0 shifts the managers and never advances the consensus
pointer, so a stable release through it publishes everything, reports
activation, and leaves the channel naming the release before it. The smokes fail
because they install what is still canonical, and the release cannot be
completed from an operator seat — the deed needs credentials that exist only
inside a lane (issue #346).

Run `plumb lane --write` with this version and land the result before the next
stable release. An exact release is unaffected. A release already split this way
is repaired by rendering again and dispatching the same stable version: publish
is create-only and self-verifying, so re-running advances the pointer and
nothing else.

## `plumb stable packport` is now `plumb stable rejoin`

The command surface changed. `packport` is in the retired dictionary, so any
tracked byte still carrying the word is out of true once this Plumb is
installed — in this repository and in every repository the release reaches.
Rename the call, and the prose around it.

The shared release lane probes for both spellings, so a repository that has not
migrated still runs. That probe comes out once the estate has.

## A dry run needs the network and credentials

`--dry-run` on the operator verbs — `stable prepare|pick|freeze|rejoin|retract`
and `ship binary dispatch` — now performs every read and skips only the writes.
It resolves the same conditions the real run resolves, so what it prints is what
would happen against the remote as it stands, rather than a guess written beside
the code.

The consequence is that it needs the same credentials as the real run, and that
it refuses instead of printing a plan it could not verify. A preview against a
line that does not exist now says so; it used to print a plan to freeze it.

`plumb retire --dry-run` is unchanged and still reads no credentials.

## The agent and design document budgets now scale

They were a flat 320 Markdown lines. They now measure the source they document,
inside a corridor of 240 to 800. A large repository gains room; a small one
drops to 240 and may be out of true on an `AGENTS.md` that passed before. The
report names the budget it applied, so the number is not a guess.

## An npm republish compares the tarball

Publishing a version that already stands used to be skipped on the strength of
the name. It now compares `dist.integrity` against the archive just built and
refuses with `published module drift` when they disagree. A re-run of an
unchanged release passes as before; a re-run whose inputs moved refuses instead
of reporting a success it did not verify.

If a lane republishes deliberately with different content under one version,
that lane was already wrong and is now told so.

## A medium publishes only after its release seal reads back

`plumb ship cargo|npm|oci|chart publish` for a product that carries a binary now
reads the release seal from the public authority before projecting. Inside the
lane nothing changes, because the projecting jobs already run after the sealing
job. Running one by hand before the binary release is published now refuses,
where it used to proceed on the strength of a local capsule file.

## Seal inputs gained the `oci` object

Baselines recorded by an earlier Plumb do not carry it, so the first release
after this one re-times every object once. Nothing else follows from it.

## `plumb stable` is now `plumb release`

Every verb moved and none changed its arguments:

```
plumb stable prepare → plumb release prepare
plumb stable pick    → plumb release pick
plumb stable freeze  → plumb release freeze
plumb stable rejoin  → plumb release rejoin
```

`stable` remains an ordinary word — the channel, the latest, the pointer — so it
is not retired and nothing goes out of true for using it. Only the command
moved. No lane calls these verbs, so no lane changes.

## `plumb stable retract` is now `plumb release retract`

A point belongs to the release segment, so the verb that removes one moved with
it. It now accepts an exact version as well as a stable one.

## A stable release is declared before it runs

`plumb stable freeze` no longer stamps the stable point. Stamp it with `plumb
release stamp --version <stable>`; dispatching stable refuses until it stands at
the line head. An exact release takes the same verb instead of `git tag`.

## Attachment widths

The per-attachment table is replaced by one `ceiling` of ten, with the table
carrying only departures. Nothing that was permitted becomes refused.
