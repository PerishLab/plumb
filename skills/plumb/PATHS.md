# Hot paths

## Enter and change a repository

1. Resolve the repository root by its `plumb.toml`.
2. Read the repository's own instructions.
3. Run `plumb doctor .` before changing shape. If Plumb reports an absent or
   unreadable rule seat, run `plumb depot sync` and repeat Doctor.
4. Inspect the relevant files and make the smallest coherent change.
5. Run Doctor again, then prove the exact staged tree with `plumb precommit .`.

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

## Prove a staged tree

Install Plumb's untracked Git hooks once, then commit normally:

```bash
plumb precommit . --install
git commit
```

The pre-commit hook checks out the index as an isolated exact tree and runs only
actions whose input and tool world have no held proof. The commit-msg hook
carries the resulting proof into the commit. Hooks refuse an occupied seat; they
do not replace another tool's hook. Run `plumb precommit .` directly to inspect
or refresh the staged proof before committing.

## Land a completed branch

Prove and commit the staged tree first, then preview and land from the clean
topic worktree:

```bash
plumb land . --base main --dry-run
plumb land . --base main --title "TITLE" --body "BODY"
```

Forgejo operations use the published Runseal library dialect in-process.
Provide `FORGEJO_URL` and either `FORGEJO_TOKEN_FILE` or `FORGEJO_TOKEN`.
Plumb does not discover or parse `tea.yml`.

Keep the source branch. Plumb creates the one-commit projection, requires its
tree to equal the carried proof, advances the base only if the observed
revisions still match, and syncs the separate base worktree.

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

## Operate a site

```bash
plumb ship site plan .
plumb ship site inspect .
plumb ship site deploy .
```

Plan is credential-free. Inspect reads current state. Deploy builds, uploads,
reads binding, and proves the public fingerprint. Cloudflare reads use
Runseal's structured in-process dialect; Plumb retains deployment and outcome
interpretation.

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
