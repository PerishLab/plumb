# Migrating to Plumb v0.20.0

## Two verb families moved

```text
plumb release registry publish   -> plumb ship cargo publish
plumb release registry rehearse  -> plumb ship cargo rehearse
plumb site plan                  -> plumb ship site plan
plumb site inspect               -> plumb ship site inspect
plumb site deploy                -> plumb ship site deploy
```

There is no alias. An old call fails at argument parsing, before any network
write, so a stale caller refuses instead of publishing wrongly. `release` keeps
`source`, `compile`, `packport`, `authority`, `channel`, `promote`, and the
unsettled `activate` and `inspect`.

## A lane asks the binary, not the version

A lane installs canonical stable Plumb and runs whatever verbs that version
holds, so a lane pinned to one spelling breaks the moment stable crosses this
release. Ask the binary instead:

```sh
if plumb ship cargo publish --help >/dev/null 2>&1; then
  plumb ship cargo publish
else
  plumb release registry publish
fi
```

Probe the deed, not the adaptor. `plumb ship cargo --help` succeeds on a
version where the adaptor is declared and still refuses, and the probe would
then choose a verb that cannot run.

## An older Plumb refuses a newer manifest

`plumb.toml` gains optional `[release.oci]`, `[release.chart]`, and
`[release.npm]`. The release declaration rejects unknown fields, so a Plumb
older than this release cannot read a manifest that carries them, and every
`plumb release` and `plumb ship` deed reads the manifest first.

Declare an attachment only after the Plumb that builds the repository is at
least this version. A repository that declares none is unaffected.

## JSR is gone

First-party JSR resolution is removed with the registry, its protocol seat, and
the radius dimension that read `deno.lock` and `.runseal/deno.lock`. A product
resolving a first-party package from JSR has no path forward here; Cargo is the
only lock radius reads. `plumb radius` reports one fewer dimension.

## A stable line stamps a point

`plumb stable freeze` now creates and pushes `vX.Y.Z` at the frozen head. It is
idempotent when the point already stands at that commit and refuses when it
stands elsewhere.

```bash
plumb stable retract --version vX.Y.Z
```

Retraction removes that point while nothing is published, and refuses once the
release authority serves a seal for the version. It never touches the line.
