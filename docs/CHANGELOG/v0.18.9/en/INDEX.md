# Plumb v0.18.9

## Exact change-boundary proof

`plumb precommit` and the matching library API now prove that the committed
delta between two exact commit OIDs stays within explicit repository-relative
write prefixes. The proof requires a clean current head, treats rename and copy
as both old and new paths, and returns normalized changed and outside sets in
the versioned JSON report.

The evaluator is coordinator-neutral. It does not read task state, create Git
hooks, or assign claims.

## Transitional domain vocabulary

Doctor now reports a release-locked `p64-v1` retired domain dictionary and
checks tracked path and worktree bytes outside `docs/CHANGELOG/**`. Hits are out
of true and unread evidence is blind. The JSON report binds results to the
dictionary digest and scan coverage.

This release intentionally carries an empty retired set. It establishes the
mechanism without beginning a vocabulary transition.

## Forgejo authority discovery

Stable operators now consume the final Tea login entry as well as entries that
precede another login. A normal single-login `tea.yml` therefore supplies its
existing Forgejo token without requiring duplicate configuration.
