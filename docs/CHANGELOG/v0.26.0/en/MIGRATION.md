# Migrating to Plumb v0.26.0

## Render the lanes again

The guard lane changed twice: it declares the release line it stands on, so the
datum is read in CI as well as under an operator's hand, and it seats the package
store outside the source, which no guard container did before. Run
`plumb lane --write` and land the result. Until then a rendered lane draws its
usual drift note, and a release refuses to start on it.

## Open release lines from this version

A line prepared by an earlier Plumb carries no datum, so `doctor` reports it out
of true on that line. Cut new lines with this version; an open line either takes
`plumb stable prepare` again for the same version, which records the datum and
moves nothing else, or finishes under the Plumb that opened it.

`prepare` and `freeze` now refuse while the previous stable point is not an
ancestor of `origin/main`. If a release was published and never packported, run
`plumb stable packport --version <previous>` first. That is not new work; it is
the step that was always last and is now required to be.

## Track before you seal

`plumb document` proposes nothing for a source seat holding an untracked leaf.
The habit this asks for — `git add`, then `plumb document` — was already the
correct order; it is now the enforced one.

## Attachment width

An attachment declares at most the number of packages Plumb permits: two for
`cargo`, one for `npm`. A repository declaring more is out of true, and widening
the permission is a change to Plumb rather than a declaration a product makes.
Declaring more than Plumb has itself released is noted, not refused.

## The `plumb 0.25.0` hole in the registry

v0.25.0 published `plumb-macro` and skipped `plumb`, because the skip this
release removes decided the library had not changed. That version does not exist
in the registry and cannot be added: a published identity is immutable, and the
gap is permanent. Anyone who looked for `plumb 0.25.0` and found nothing was
looking at a real absence. Depend on `0.26.0`, which publishes both.

## Seals recorded before this release

A seal's `inputs` map is keyed by object, and objects are now attachments rather
than packages: `cargo/<crate>` and `npm/<package>` become `cargo` and `npm`. An
earlier baseline therefore matches nothing, and the first release after this one
records every object as having changed at that version. Nothing acts on the
difference, so nothing is lost but the age of the evidence.

`[release.depends]` keys use the same names. No repository in this domain
declares that table today.
