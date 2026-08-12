# Hot paths

## Enter and change a repository

1. Resolve the repository root by its `plumb.toml`.
2. Read the repository's own instructions.
3. Run `plumb doctor .` before changing shape.
4. Inspect the relevant files and make the smallest coherent change.
5. Run Doctor again, then the repository's complete guard.

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
plumb precommit . --base BASE --head HEAD --write PATH
```

Repeat `--write` for disjoint prefixes. Both sides of a rename or copy must be
covered. A symbolic revision, moving head, dirty tree, non-ancestor base, or
path outside the boundary refuses.

## Land a completed branch

Run the repository guard first, then preview and land from the clean topic
worktree:

```bash
plumb land . --base main --dry-run
plumb land . --base main --title "TITLE" --body "BODY"
```

Keep the source branch. Plumb creates the one-commit projection, waits for its
guard, advances the base only if the observed revisions still match, and syncs
the separate base worktree.

## Inspect and affirm a lock

When Doctor reports drift, read every path named by the lock and compare the
brief with the behavior it describes. If the binding is still true, print the
fresh values and deliberately apply them to `plumb.toml`:

```bash
plumb lock .
```

Run Doctor after the edit. Never script affirmation into guard.

## Measure release radius

Supply the candidate and every consumer root explicitly:

```bash
plumb radius --product PRODUCT --candidate VERSION \
  --root api=/path/to/api --root web=/path/to/web
```

The result reads locks only. An unread or non-version resolution is blind and
is never counted as current.

## Operate a site

```bash
plumb site plan .
plumb site inspect .
plumb site deploy .
```

Plan is credential-free. Inspect reads current state. Deploy builds, uploads,
reads binding, and proves the public fingerprint.

## Operate a skill seat

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
