# Migrating to Plumb v0.18.3

Repositories without direct `@perish` JSR or `perish` Cargo dependencies need
no dependency migration. For governed edges, run doctor with registry access
and move each stale direct dependency when that repository is next touched.

For Deno, remove the published version from each first-party import:

```json
{
  "imports": {
    "@perish/sealkit": "jsr:@perish/sealkit"
  }
}
```

Then use Deno to refresh the frozen lock. For Cargo, retain the intended native
SemVer requirement, widen or advance it when necessary, and use Cargo to update
the exact lock resolution to stable latest. Do not edit either lock by hand.

Doctor deliberately refuses when the registry, manifest, or lock cannot prove
the current version. Restore that evidence rather than introducing a cached
version table, compatibility line, or temporary exception. Transitive
staleness belongs to the upstream repository that declares the direct edge.
