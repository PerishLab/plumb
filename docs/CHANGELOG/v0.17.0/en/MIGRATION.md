# Migrating to v0.17.0

## If your repository consumes Sealkit

Existing unversioned imports with a frozen stable `0.1` resolution require no
immediate edit. Doctor keeps them green during the transition and now prints
the held requirement and resolution.

When deliberately migrating a repository, use the supported line:

```json
{
  "imports": {
    "@perish/sealkit": "jsr:@perish/sealkit@^0.2.1"
  }
}
```

Then let Deno replace the matching frozen resolution:

```sh
deno install \
  --config .runseal/deno.json \
  --lock .runseal/deno.lock \
  --frozen=false \
  --minimum-dependency-age=0
```

The age override is for the deliberate refresh immediately after a new
release. Ordinary guards continue to use the frozen lock and do not resolve the
registry. Plumb neither edits the lock nor asks the network.

A missing or malformed matching lock is reported as blind without creating the
workshop-wide red interval this transition avoids. An unsupported requirement,
a supported requirement resolving below `0.2.1`, or a resolution outside
stable `0.1` and `^0.2.1` is out of true.

## If your repository publishes crates

Declare the surface so Plumb can see it, and name the operator entry `release`:

```toml
[release.cargo]
registry = "perish"
packages = ["your-macro", "your-lib"]
```

`ship` names a site deployment and `release` names a registry publish; both
halves are mechanized, so a wrapper carrying the wrong word is reported once
the surface is visible.

## If your repository declares a release table

Nothing to do. Every repository declaring `[release]` in this workshop names
`binaries`, so the sharper test changes no verdict. A repository that declared
a release table without binaries was counted as a binary publisher before and
is not any more, which is the point.

## If you upgraded a repository that publishes crates before upgrading Plumb

Declare the cargo surface only after this version is installed. An older Plumb
reads any release key as a binary release and will ask for `release-exact` and
`release-stable` lanes that a crate family does not use.

## If your guard runs `plumb doctor` without a `plumb.toml`

Decide which you meant. A repository that gates its build on a doctor finding
is governed by these laws and should declare so; one that only wants the report
is not, and the laws do not bind it either way.

Declaring costs one file. An empty `plumb.toml` is a complete declaration when
there is no release surface, site, or lock to name; the tables come later if
they ever apply. Nothing changes for a repository that already carries it.

## If a stable manager reports an old installation as unowned

Run the current canonical stable manager again. It now adopts the old marker
format when the root marker, version marker, and installed entrypoint all prove
one stable seat. No manual uninstall is needed. A custom authority, prerelease
channel, ambiguous seat, or changed entrypoint is intentionally not adopted;
inspect those installations rather than weakening the ownership proof.
