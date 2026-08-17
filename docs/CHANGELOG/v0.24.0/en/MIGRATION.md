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

## A rendered lane serves what canonical stable holds

A rendered lane installs canonical stable Plumb, so every deed it spends must
exist in the release that is already published. The projected matrix carries the
prepare deed from this release onward; a repository rendering these lanes
releases through the Plumb that reads them, which is this one or later.

Two consequences to know before you render. A binary-backed image cannot project
through the ship lane yet: the projection job holds no archives, and an image that
wraps one takes its payload from them. And this product releases v0.24.0 through
the shared workflows one last time, because the lane it now carries asks for a
plan only this release produces.
