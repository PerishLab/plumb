# Hot paths

## Enter and change a repository

1. Resolve the repository root by its `plumb.toml`.
2. Read the repository's own instructions.
3. Run `plumb doctor .` before changing shape. If Doctor reports absent guard
   hooks, run `plumb configuration install` and repeat Doctor.
4. Inspect the relevant files and make the smallest coherent change.
5. Run Doctor again; the pre-commit hook Plumb projects proves the exact staged tree.

Doctor's human report is for immediate work. Use JSON when another command
needs stable fields:

```bash
plumb doctor . --json
```

## Resolve a finding

Start from its rule instead of guessing from the evidence string:

```bash
plumb rule show RULE_ID
plumb rule list --namespace NAMESPACE
plumb rule list --standing mechanized
```

Repair `out of true`. Restore readable evidence for `blind`. For `unknown
shape`, decide whether the repository is wrong or the skeleton lacks a real
shape; do not suppress the observation. A clean Doctor report covers only
mechanized rules that actually evaluated.

## Prove a task boundary

Commit the member, keep its worktree clean, and supply exact revisions plus
each declared write prefix:

```bash
plumb guard . --base BASE --head HEAD --write PATH
```

Repeat `--write` for disjoint prefixes. Both sides of a rename or copy must be
covered. A symbolic revision, moving head, dirty tree, non-ancestor base, or
path outside the boundary refuses.

## Prove a staged tree

Project the Git hooks this Plumb carries, then commit normally:

```bash
plumb configuration install
git commit
```

The pre-commit hook checks out the index as an isolated exact tree and runs only
actions whose input and tool world have no held proof. The commit-msg hook
carries the resulting proof into the commit. Plumb owns and replaces both
guard hooks. Run `plumb guard .` directly to inspect or refresh the staged proof
before committing.

## Land a completed branch

Prove and commit the staged tree first, then preview and land from the clean
topic worktree:

```bash
plumb land . --base main --dry-run
plumb land . --base main --title "TITLE" --body "BODY"
```

Forgejo is archived; origin must be the GitHub repository. Land drives `gh`,
so run it under the Runseal profile that authorizes it, such as
`runseal :liberte plumb land .`. Plumb holds no GitHub credential.

Keep the source branch. Plumb creates the one-commit projection, requires its
tree to equal the carried proof, merges with a merge commit only if the pull
still stands at that candidate and the base has not moved, and syncs the separate base worktree.

## Inspect and affirm a document

When Doctor reports drift, read every named source and its target. If the
projection remains true, print fresh source and target seals and deliberately
record them in `plumb.toml`:

```bash
plumb document .
```

Run Doctor after the edit. Never script affirmation into guard or migration.

## Measure release radius

Supply the candidate and every consumer root explicitly:

```bash
plumb radius --product PRODUCT --candidate VERSION \
  --root api=/path/to/api --root web=/path/to/web
```

The result reads locks only. An unread or non-version resolution is blind and
is never counted as current.

## Operate a skill seat

A repository shipping a source skill declares one closed document binding:

```toml
[[document]]
strategy = "brief"
name = "product"
source = [{ path = "src", seal = "<human affirmation>" }]
target-seal = "<human affirmation>"
```

The strategy derives `skills/product/{SKILL.md,PATHS.md,SCENARIOS.md}` and its
aggregate text budget. Do not declare either separately. A repository that
predates document bindings follows the versioned release CHANGELOG before
running the current binary.

```bash
plumb skill status
plumb skill upgrade --dry-run
plumb skill upgrade
plumb skill stage --channel CHANNEL --version VERSION --path /isolated/plumb
plumb skill list
plumb skill uninstall
```

Managed operations accept stable. Stage accepts one exact non-stable version,
requires a new path ending in the skill name, and never enters the managed
ledger.
