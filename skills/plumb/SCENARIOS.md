# Restrained scenarios

## An operation is blind

Stop the stateful path and restore the named evidence. A network lookup,
manifest, lock, source tree, or policy that could not be read has not proved a
negative fact. Re-run the same operation after evidence is readable; do not
substitute a cached assumption.

## A release is being prepared

Use `plumb release --help` and `plumb ship --help` for the exact contracts.
Plumb owns release identity; wharf owns distribution. A marker is an annotated
tag on the head of its release line, and sources keep version `0.0.0`:

```bash
plumb release stamp --version VERSION-rc.N
plumb ship dispatch --marker VERSION-rc.N --watch
plumb release stamp --version VERSION
plumb ship dispatch --marker VERSION --watch
```

A stable marker promotes an rc or beta that already shipped from the same
commit, and refuses while the last stable marker is not yet an ancestor of the
remote main. Doctor notes that debt on every run. Settle it by merging the
release line into main with a merge commit, never by squashing it away, and
integrate real fixes on main first. Recover a failed ship by dispatching the
same marker again; never move or delete a marker.

## A skill candidate needs validation

Do not replace a managed stable seat. Stage the exact candidate under a new
isolated root, run the intended session against that path, and remove the
surrounding isolated root only after the caller has preserved any evidence it
needs. Promotion later uses the same source revision as the validated candidate.

## A worker reaches its stable domain

Worker versions and their deployments are wharf's to publish. Never recover by
rebuilding or by running a direct deploy from source; dispatch the same marker.

## Observation is configured

Observation defaults muted. Do not enable it, choose a report path, or select
facts on an operator's behalf. When explicitly enabled, invalid observation
configuration refuses the audit record without replacing the command's own
result. Treat trace keys as opaque correlation values, never as authority or
lifecycle state.
