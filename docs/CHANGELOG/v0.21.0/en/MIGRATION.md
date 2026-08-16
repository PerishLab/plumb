# Migrating to Plumb v0.21.0

## Older Plumb cannot read a manifest declaring an account

`[release.oci]` and `[release.chart]` gain a required `account`. The release
declaration refuses unknown fields, so a Plumb older than this one cannot read a
manifest carrying it, and every release deed reads the manifest first.

```toml
[release.oci]
registry = "git.perish.top"
image = "perishlab/example"
account = "PerishFire"
```

Declare it only once the Plumb building that repository is at least this
version. Plumb's own manifest declares neither attachment here, for that reason.

## `PLUMB_RELEASE_REGISTRY_ACCOUNT` is removed

The account is a public name and now comes from the declaration. A lane that
exported the variable can stop. `PLUMB_RELEASE_REGISTRY_TOKEN` is unchanged.

## The module adaptor requires pnpm

`ship npm pack` and `ship npm publish` invoke `pnpm`. A module whose build
output is published should declare `prepack`, which both package managers run.

## Doctor may report a declaration it accepted before

Two rules become mechanized, so a repository can go out of true unchanged.
`release.spec-declared` refuses a `[release]` table the strict spec cannot read;
an attachment table preceding a later top-level key is the common shape, because
TOML reparents that key onto the attachment.
`release.attachment-deliverable` refuses a declared attachment no shared lane
carries, or one whose lanes this repository never calls.

`publishes` now lists declared attachments, so a repository carrying
`packages/*` without a module attachment no longer reports `npm`. The catalog
holds 114 rules: 74 mechanized, 1 observed, 39 prose-only.

## A partly failed release resumes

Re-running no longer refuses on a medium already reached, and no longer replaces
one silently: content differing from what the registry holds refuses and names
both sides. `plumb land` also works twice on one topic branch.
