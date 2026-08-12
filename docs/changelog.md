# Changelog

A stable release is immutable. Whatever it changed, and whatever anyone must do
about it, has to be written before the release goes out, because afterwards
there is nowhere to put it.

## The shape

```
docs/CHANGELOG/v<version>/<lang>/{INDEX.md, MIGRATION.md}
docs/CHANGELOG/v<version>/artifacts/*
```

`en` and `zh` are the floor, not the set. A repository may carry more languages;
it may not publish a stable release carrying fewer. Naming the floor rather than
the membership keeps the language list out of the substrate — adding a third
language is a repository's decision and requires no change here.

INDEX.md says what changed. MIGRATION.md says what a consumer must do about it.

`artifacts/` is optional. Its direct regular files join the immutable release
artifact set under their filenames. Plumb preserves their bytes but assigns no
meaning to their names or contents; the product owns those contracts. Links,
directories, non-UTF-8 names, and collisions with derived release artifacts
refuse assembly. There is no file-count limit.

Prerelease identity resolves this directory through its stable base. Thus
`v1.2.0-beta.3` and `v1.2.0` both read
`docs/CHANGELOG/v1.2.0/artifacts/`, allowing the existing promotion proof to
bind the same version-owned payload.

## Write the empty migration

Most releases require nothing of anyone. Write MIGRATION.md anyway, saying so.
"This release requires no migration" is a conclusion someone reached by reading
the diff; a file nobody wrote is not the same artifact, and the two are
indistinguishable once the release is out. The point of the requirement is to
make someone look, which is also why the check refuses a file that exists but
holds nothing.

## The gate is on the lane, not the doctor

`plumb doctor` does not check changelogs, deliberately. A changelog is a
property of a **release**, not of a working tree; gating the working tree would
turn a repository red the moment someone bumped a version number, for a document
that is not owed until the release actually happens.

`plumb release compile` checks the stable version before it creates a capsule —
the same position as the dry-run assertion, and for the same reason: it is the
last moment at which the omission is still free. `plumb changelog` remains the
direct operator probe for that contract.

Prereleases are exempt. A beta is a disposable validation cut that carries no
tag; demanding migration notes for something nobody is asked to migrate to would
be ceremony.

## Not retroactive

The rule takes effect from the version that introduces it. Reconstructing the
changelog of an already-published version means presenting a guess as a record,
which is worse than the gap it fills.

## The install root keeps one version

Related, because both are about a release being a thing that happened rather
than a thing you can revisit: `manage.sh` keeps exactly one version on disk.

The versioned install root read as a rollback affordance, but nothing ever used
it — `install --version <older>` deletes that directory and downloads the
version again. What accumulated was not a safety net, only unread bytes: eleven
versions in a single day of ordinary work.

Rollback is a download, because published artifacts are immutable and
permanently retrievable. The cost is offline rollback on a workstation that
cannot reach the release host, which was accepted knowingly.
