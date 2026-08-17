# Migrating to Plumb v0.24.0

## Render your lanes before your next release

`plumb ship binary dispatch` now names the rendered `exact.release.yml` and
`stable.release.yml`. A repository still carrying the thin callers into the
shared workflows refuses before it reaches the forge and names the command to
run:

```bash
plumb lane --write
```

Land that, then dispatch. Nothing about the change is global, so a repository
migrates when its operator wants a release, not on a schedule someone else sets.
Delete `release-exact.yml`, `release-stable.yml`, and any `deploy.yml` once the
rendered lanes are in place; a site becomes `[release.cfworker]` instead.

## Rendered lanes from v0.23.0 must be rendered again

A lane written by v0.23.0 carries at least one shape this forge cannot run. Any
repository that already ran `plumb lane --write` with v0.23.0 should run it again
with this release and land the result. Doctor reports the difference as drift,
and a release refuses on a lane an older Plumb rendered.

## A worker medium needs its credential forwarded

`[release.cfworker]` projects through the ship lane, which now takes a site token
beside the registry token. Provision `PLUMB_SITE_TOKEN` as a repository secret;
the rendered release lanes forward it.
