# Plumb v0.18.0

## Dynamic stable assembly

Exact publication keeps its source freedom: the workflow dispatch ref is the
only selected source, and the called workflow's direct event ref plus SHA bind
every shared job to one immutable commit. Exact channels do not consult
release-branch state.

Stable publication alone is bound to `release/vX.Y.Z`. A local operator prepares
the line, appends selected changes through linear `cherry-pick -x`, and freezes
it before dispatch. Plumb validates the stable ref, version, commit, and
promotion proof before publication.

After activation, packport uses a topology-preserving merge into `main`. The
published stable commit must become an ancestor of `origin/main`; this debt
blocks only the next stable activation, never ordinary `main` work or exact
publication. A settled release branch may be deleted.

## Release identity

Immutable exact seals and the stable pointer remain the release authority. New
releases no longer create Git tags, and the stable product workflow now needs
only read access to repository contents. Historical tags remain untouched.

## Operator support

Plumb now admits the Sealkit `^0.3.1` line that provides `prepare`, `pick`,
`freeze`, and `packport`. The previous `^0.2.1` line remains accepted during
migration.
