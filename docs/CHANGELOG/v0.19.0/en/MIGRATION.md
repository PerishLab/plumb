# Migrating to Plumb v0.19.0

## Stop passing channel and version to a release lane

A caller forwards only promotion selection and guard evidence:

```yaml
jobs:
  release:
    uses: PerishLab/actions/.forgejo/workflows/release-binary.yml@main
    with:
      guard_contexts: '["guard / guard (push)"]'
```

Both inputs are still declared and unread, so an unchanged caller keeps working
while it migrates. They are removed once no caller passes them.

Dispatch on the ref that carries the identity: an exact tag for
`release-exact.yml`, the frozen release line for `release-stable.yml`. Push the
tag before dispatching, because the source binding accepts exactly
`refs/tags/<exact-version>` and nothing else.

```bash
plumb ship binary dispatch --version v0.19.0-beta.1
```

## Carried over from v0.18.28

That release moved nine verbs and rebound the exact source without a migration
note, and a stable release cannot be rewritten afterwards. The guidance belongs
here instead.

```text
plumb release build     -> plumb ship binary build
plumb release assemble  -> plumb ship binary assemble
plumb release matrix    -> plumb ship binary matrix
plumb release managers  -> plumb ship binary managers
plumb release publish   -> plumb ship binary publish
plumb release smoke     -> plumb ship binary smoke
plumb release verify    -> plumb ship binary verify
plumb release dispatch  -> plumb ship binary dispatch
plumb release recovery  -> plumb ship binary recovery
```

There is no alias. An old call fails at argument parsing, before any network
write, so a stale caller refuses instead of publishing wrongly. `release` keeps
`source`, `compile`, `packport`, `authority`, `activate`, `inspect`, `promote`
and `registry`.

An exact release may no longer bind an arbitrary branch. It binds the tag that
names its version, and a stable release binds its release line.
