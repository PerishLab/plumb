# Plumb v0.18.1

## Durable release sources

Settled `release/vX.Y.Z` branches are now permanent frozen source and audit
boundaries. Packport changes only `main` ancestry and never authorizes removing
the branch or its protection.

## Protection wall

The Plumb skill now owns one canonical, byte-exact branch-protection shape for
every managed repository. `main` and `release/**` baseline rules have no
repository-specific options, while exact release rules derive only from
PREPARING or FROZEN state.

This law is deliberately prose-only. `plumb doctor` does not claim access to
Forgejo control-plane truth; a local operator applies each rule and verifies
the projected API response byte-for-byte.
