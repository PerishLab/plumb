# Restrained scenarios

## An operation is blind

Stop the stateful path and restore the named evidence. A network lookup,
manifest, lock, source tree, or policy that could not be read has not proved a
negative fact. Re-run the same operation after evidence is readable; do not
substitute a cached assumption.

## A release is being prepared

Use `plumb release <command> --help` for the exact command contract. One run
binds its source revision and release identity, builds only declared artifacts,
publishes immutable objects and an exact seal, then reads the public bytes back.
Non-stable releases remain exact and have no moving pointer or activation.
An exact release binds `refs/tags/<exact-version>`, so push that tag before
dispatching one; its channel is read from the version, never named beside it.

Stable starts from an explicit release line:

```bash
plumb stable prepare
plumb stable pick
plumb stable freeze
plumb ship binary dispatch
plumb stable packport
```

Inspect each subcommand before use. `prepare` also records the line's datum,
so the frozen candidate is judged against the registry answers that stood when
the line was cut; a line carrying no datum is out of true. Stable publication
requires the frozen line and release-local changelog. `freeze` also derives the promotion source
and refuses unless exactly one published exact seal stands at the frozen
commit, so recover a failed exact release by rerunning it rather than by
tagging the next candidate. Packport settles ancestry after publication;
it does not rewrite a successful release result, and `prepare` and `freeze`
refuse while the last stable point still sits outside `origin/main`, so a late
packport is caught before the next line exists. Never delete the frozen line.

Packport merges, and a merge can conflict: a line that absorbed fixes while
`main` absorbed the same fixes carries them under different commits, and Git
reads both sides as having changed the same lines. Settle it by hand, and settle
it one way only — take `main`'s tree entire and record the line as a second
parent, which is `git merge -s ours` from `main`. Do not resolve the conflicts
file by file. Nothing on the line is missing from `main`: every commit there was
picked from `main`, and the datum is the line's own state. The merge carries no
content, only reachability, so its diff against `main` must be empty and the
stable commit must be reachable from the result. Verify both before pushing.

## A skill candidate needs validation

Do not replace a managed stable seat. Stage the exact candidate under a new
isolated root, run the intended session against that path, and remove the
surrounding isolated root only after the caller has preserved any evidence it
needs. Promotion later uses the same source revision as the validated candidate.

## A site reports partial success

Keep `deployed`, `bound`, and `reachable` separate. Deployment proves upload;
binding proves the declared route state; reachability proves the public edge
serves the built fingerprint. An authority unable to inspect binding yields
`unknown`, not `no`, and a status code without the fingerprint is not proof.

## Observation is configured

Observation defaults muted. Do not enable it, choose a report path, or select
facts on an operator's behalf. When explicitly enabled, invalid observation
configuration refuses the audit record without replacing the command's own
result. Treat trace keys as opaque correlation values, never as authority or
lifecycle state.
