# plumb v0.15.0

## One version on disk

`manage.sh install` and `update` now leave exactly one version behind. Every
other version directory under the install root is removed once the new binary is
linked and has answered `--version`.

The versioned install root looked like it existed for rollback, but nothing ever
read it: `install --version <older>` deletes that directory and downloads the
version again. Eleven versions and 46MB had accumulated in a single day, and no
code path would have used any of them.

The sweep names what it removed rather than doing it silently.

## `plumb changelog`

A new command reads `docs/CHANGELOG/v<version>/{en,zh}/{INDEX.md,MIGRATION.md}`
and refuses when any of the four is missing or empty. The version comes from
`--version`, or from the repository's own declared version when the flag is
absent.

English and Chinese are the floor, not the set. A repository may add more
languages; it may not publish a stable release with fewer.

## A changelog gate on stable releases

`release-stable.yml` runs that command before the first irreversible step. This
is where the enforcement lives — `plumb doctor` does not check changelogs, so
bumping a version number never turns a working tree red. A changelog is a
property of a release, not of a working tree, and a published stable release is
immutable: what it changed can never be written afterwards.

## Standing gained a third category

The skill's `Standing` section used to be a two-way split — what `plumb doctor`
catches, and what nothing catches. This clause belongs to neither: a machine
enforces it, but the machine is the release lane. Claiming doctor checks it
would be exactly the kind of false claim that section forbids.
