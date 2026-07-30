# Migrating to Plumb v0.18.1

Upgrade the release operator to Sealkit v0.3.2 or newer and remove
`--keep-branch` from packport calls. Retention is no longer optional.

Read `skills/plumb/references/protection.md`, reconcile the canonical `main`
and `release/**` rules in every repository carrying a root `plumb.toml`, then
read each rule back and compare its canonical projection byte-for-byte.

For each stable pointer whose `release/vX.Y.Z` branch is missing, recreate the
branch at the pointer's exact commit and apply the FROZEN rule. Do not create a
new release tag.
