# Plumb v0.18.14

## Coupled Cargo prereleases

Cargo release stamping now treats the packages declared by
`release.cargo.packages` as one coupled family. Selecting an exact prerelease
identity updates both package versions and exact internal path dependency
requirements before Cargo rehearsal or publication.

The rewrite covers workspace dependencies and package-level dependencies,
build dependencies, and development dependencies while remaining limited to
path dependencies whose package identity belongs to the declared release
family.
