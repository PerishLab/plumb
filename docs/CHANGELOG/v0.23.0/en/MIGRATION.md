# Migrating to Plumb v0.23.0

## A module attachment names packages

`[release.npm]` took `package`. It now takes `packages`, an ordered list, and a
repository publishing a single module writes a list of one.

## Dispatch takes only a version

`plumb ship binary dispatch` no longer accepts `--promotion-channel` or
`--promotion-version`, and a caller passing either now fails to parse. A stable
dispatch names its version and nothing else.

## Governed workflows are rendered

Run `plumb lane --write` and land the result. Until then `doctor` notes each
governed lane as absent, which does not change its exit code and does not block
a release. Once a lane is rendered, editing it by hand makes the next dispatch
refuse: a rendered lane that drifted is a lie about what will run, while one
never rendered is a repository that has not adopted the mechanism yet.

## A site declares itself

The account and domain `deploy.yml` passed as workflow variables belong in
`[release.cfworker]`; the token stays a secret. A repository still running
`ship site deploy` is unaffected until it declares the attachment.

## Seals gain fields without changing schema

Seal schema stays 1. `inputs` is additive and optional, so an older Plumb reads
a new seal and a newer Plumb reads an old one. `ship binary smoke` now accepts a
seal URL and resolves the manager for the platform it runs on; a manager URL
still works.

## A settled crate keeps its published version

A workspace crate whose inputs have not changed is not republished, so a crate
released now can require an older exact version of it. Nothing to do: the
requirement names a version the registry already holds.
