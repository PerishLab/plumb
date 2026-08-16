# Migrating to Plumb v0.21.0

## Older Plumb cannot read a manifest declaring an account

`[release.oci]` and `[release.chart]` gain a required `account`. The release
declaration refuses unknown fields, so a Plumb older than this one cannot read a
manifest carrying it, and every `plumb release` and `plumb ship` deed reads the
manifest first.

Declare it only once the Plumb that builds that repository is at least this
version. A repository that already declares either attachment must add it:

```toml
[release.oci]
registry = "git.perish.top"
image = "perishlab/example"
account = "PerishFire"
```

Plumb's own manifest declares neither attachment in this release, for the same
reason.

## `PLUMB_RELEASE_REGISTRY_ACCOUNT` is removed

The account is a public name and now comes from the declaration. A lane that
exported the variable can stop; one that never did needs no change.
`PLUMB_RELEASE_REGISTRY_TOKEN` is unchanged and still carries the credential.

## The module adaptor requires pnpm

`ship npm pack` and `ship npm publish` invoke `pnpm`, not `npm`. A lane
publishing a module must have pnpm available; the shared binary lane already
enables corepack where a lockfile exists.

A module whose build output is published should declare `prepack`, which both
package managers run. Nothing else needs to change.

## Doctor may report a declaration it accepted before

Two rules become mechanized, so a repository can go out of true without changing:

- `release.spec-declared` refuses a `[release]` table the strict spec cannot
  read. A manifest whose attachment table precedes a later top-level key is the
  common shape: TOML reparents that key onto the attachment.
- `release.attachment-deliverable` refuses a declared attachment when no shared
  lane carries it, or when this repository calls none of the lanes that do.

`publishes` in the Doctor report now lists declared attachments. A repository
carrying `packages/*` without a module attachment no longer reports `npm`.

The rule catalog holds 114 rules: 74 mechanized, 1 observed, 39 prose-only. A
reader asserting those counts must update them.

## A release that partly failed now resumes

Re-running a release no longer refuses on a medium it already reached, and no
longer replaces one silently. A chart or image whose content differs from what
the registry holds refuses and names both sides rather than overwriting.

## `plumb land` works twice on one branch

A second landing from one topic branch previously waited out a guard that would
never run, because the landing branch is retained and its merged pull still
matched. Nothing changes for a first landing.
