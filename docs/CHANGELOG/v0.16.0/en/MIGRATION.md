# Migration to Plumb v0.16.0

Binary products must move their release declaration to `[release]` in the root
`plumb.toml`. Delete product release scripts, `.forgejo/release.toml`,
`release-verify`, and locally implemented manager, packaging, verification, or
registry-publish logic.

Keep only `release-exact.yml` and `release-stable.yml` as thin callers of
`PerishLab/actions/.forgejo/workflows/release-binary.yml@main`. Repository
credentials use the generic `RELEASE_PUBLISH_S3_*`,
`RELEASE_ACTIVATE_S3_*`, and optional `RELEASE_REGISTRY_TOKEN` names.

Setup callers use `PerishLab/actions/setup-binary@main` with
`PERISH_SETUP_PRODUCT`. Stable may use the default seat; every non-stable
channel must also provide an exact version and is isolated automatically.
