# Migrating to Plumb v0.27.0

## Render the lanes again, before the next release

A lane rendered by v0.26.0 shifts the managers and never advances the consensus
pointer, so a stable release through it publishes everything, reports activation,
and leaves the channel naming the release before it. The smokes fail because they
install what is still canonical, and the release cannot be completed from an
operator seat — the deed needs credentials that exist only inside a lane (#346).

A lane rendered by this version also points its channel at each exact release, so
a line generates itself thereafter. Run `plumb lane --write` and land it before
the next release. A release already split by the pointer defect is repaired by
rendering again and dispatching the same stable version: publish is create-only
and self-verifying, so re-running advances the pointer and nothing else.

## `plumb stable` is now `plumb release`

Every verb moved and none changed its arguments: `prepare`, `pick`, `freeze`,
`rejoin` and `retract` are now `plumb release` verbs, and `plumb release stamp`
makes a release point. `stable` remains an ordinary word — the channel, the
latest, the pointer — so it is not retired and nothing goes out of true for using
it. Only the command moved, and no lane calls these verbs.

## `plumb stable packport` is now `plumb release rejoin`

`packport` is in the retired dictionary, so any tracked byte still carrying the
word is out of true once this Plumb is installed. Rename the call and the prose
around it. The shared release lane probes for both spellings, so an unmigrated
repository still runs.

## A stable release is declared before it runs

`freeze` no longer stamps the stable point. Stamp it with `plumb release stamp
--version <stable>`; dispatching stable refuses until it stands at the line head,
and an exact release takes the same verb instead of `git tag`.

## A dry run needs the network and credentials

`--dry-run` on the operator verbs now performs every read and skips only the
writes, so it prints what would happen against the remote as it stands. It needs
the same credentials as the real run and refuses rather than print a plan it could
not verify. `plumb retire --dry-run` is unchanged.

## The agent and design document budgets now scale

They were a flat 320 Markdown lines and now measure the source they document,
inside a corridor of 240 to 800. A small repository drops to 240 and may be out of
true on an `AGENTS.md` that passed before.

## An npm republish compares the tarball

Publishing a version that already stands used to be skipped on the strength of the
name. It now compares `dist.integrity` against the archive just built and refuses
with `published module drift` when they disagree. An unchanged re-run passes.

## A medium publishes only after its release seal reads back

`plumb ship cargo|npm|oci|chart publish` for a product carrying a binary reads the
release seal from the public authority first. Inside the lane nothing changes;
running one by hand before the binary release is published now refuses.

## Two that ask nothing of you

Seal inputs gained the `oci` object, so the first release after this one re-times
every object once. The per-attachment width table is replaced by one `ceiling`
of ten; nothing permitted becomes refused.
