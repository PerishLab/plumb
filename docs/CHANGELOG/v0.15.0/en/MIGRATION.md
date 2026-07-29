# Migrating to v0.15.0

## For anyone who installs plumb

Nothing to do. The next `manage.sh update` sweeps the older version directories
it finds and prints what it removed.

What you lose: **offline rollback**. Previously a stale directory could be
reached by repointing the symlink by hand when the network was down. Now
rollback is `manage.sh install --version <older>`, which needs to reach the
release host. Released artifacts are immutable and permanently retrievable, so
this costs a download, not a version — but on a workstation with no route to
`releases.plumb.perish.uk` it is a real loss, and it was a deliberate trade.

CI is unaffected: those runners start from a clean container and never had a
cached version to lose.

## For a repository that publishes a stable release

Two things are now required before `release-stable` will publish.

**1. Write the changelog.** For the version you are about to release:

```
docs/CHANGELOG/v<version>/
├── en/
│   ├── INDEX.md
│   └── MIGRATION.md
└── zh/
    ├── INDEX.md
    └── MIGRATION.md
```

All four files must exist and be non-empty. `en` and `zh` are the minimum; more
languages are allowed.

Most releases change nothing that anyone must act on. Write MIGRATION.md
anyway, saying so plainly — "this release requires no migration" is a
conclusion someone reached by reading the diff, and it is not the same artifact
as a file nobody wrote.

**2. Add the gate to the lane.** In `release-stable.yml`, before the first
irreversible step:

```yaml
      - name: Changelog
        env:
          RELEASE_VERSION: ${{ needs.metadata.outputs.release_version }}
        run: plumb changelog . --version "$RELEASE_VERSION"
```

plumb itself runs `cargo run --quiet -p plumb-cli -- changelog` instead, because
a tool cannot gate its own release on a copy of itself that does not exist yet.

Nothing forces this on you until you add it: `plumb doctor` stays green either
way, and only this repository carries the gate today. The other six repositories
with a `manage.sh` are untouched and adopt it when they choose.

## Not retroactive

Versions before v0.15.0 have no changelog and none will be written. Their
diffs are in the history; a changelog reconstructed after the fact would be a
guess presented as a record, which is worse than the gap.
