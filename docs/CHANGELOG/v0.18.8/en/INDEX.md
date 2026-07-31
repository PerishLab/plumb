# Plumb v0.18.8

## Release operation belongs to Plumb

`plumb release dispatch` now owns generic Forgejo workflow dispatch. Exact
releases keep operator-selected channels and branches; stable alone derives
`release/vX.Y.Z`, requires an exact promotion, and accepts the stricter branch
lifecycle.

`plumb stable prepare`, `pick`, `freeze`, and `packport` own that lifecycle.
Protection is read back field for field. Packport preserves merge topology,
proves the published commit is an ancestor of `main`, and permanently retains
the frozen release branch. Release identity no longer requires a new Git tag.

## Site operation belongs to Plumb

`plumb site plan`, `inspect`, and `deploy` derive the application, package,
assets directory, and worker from the repository's one
`apps/*/wrangler.jsonc`. Deploy reports upload, platform binding, and public
reachability independently, and readback must contain the fingerprint from the
built index.

Cloudflare authority enters through typed `PLUMB_SITE_*` configuration. The
token is passed through environment or curl configuration input, never command
arguments.

## Product repositories lose generic control planes

Doctor no longer treats release or site wrappers as capability evidence. The
Plumb repository itself now uses an env-only Runseal profile and carries no
release, ship, or cold-start wrapper.

Release resource authority is provisioned input. Repository creation and
resource, domain, token, escrow, and secret synchronization remain with their
owning control planes instead of being inferred by Plumb or copied into a
product repository.
