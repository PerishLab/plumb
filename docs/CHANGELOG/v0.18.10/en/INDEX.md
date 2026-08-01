# Plumb v0.18.10

## Topic landing

`plumb land` settles the current clean topic branch onto its base. The source
branch is pushed without rebasing, a base-relative one-commit projection is
derived at `land/<branch>`, and the base fast-forwards onto that projection once
its guard reports success. Afterwards the separate worktree holding the base is
brought up to the landed state.

The merge request pins the exact projection head and never asks Forgejo to
delete a branch. Retention lives in the mechanism, so no caller flag can request
the opposite. `--dry-run` prints the planned actions without touching Git or the
remote, and `--no-watch` stops once the pull request exists.

Every refusal carries a kind: a detached head, the base branch itself, a release
line, a dirty tree, a missing upstream, an empty contribution, a conflicting
merge, a source or base that moved while the projection was guarded, and a guard
that failed or stayed pending.

## Forgejo and Git substrate

The Forgejo client, the Git remote helpers, the token seats, branch protection,
and the Actions workflow surface moved from the stable operator command into the
`plumb::forge` library. `plumb::land` builds on them and is reachable from a
coordinator without a subprocess. The stable operator command surface is
unchanged.

`Client::settle` takes an explicit merge strategy. The packport path keeps a
merge commit; landing uses fast-forward only. Neither may delete a branch.
