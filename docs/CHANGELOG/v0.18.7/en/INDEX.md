# Plumb v0.18.7

## Guard follows repository authority

Doctor now recognizes the canonical guard seat of either supported repository
authority: `.forgejo/workflows/guard.yml` for Forgejo or
`.github/workflows/quality.yml` for GitHub. One repository carries exactly one
of those seats; carrying both refuses as an ambiguous automation authority.

The selected workflow supplies the same guard evidence on either authority:
canonical concurrency, Plumb and Ectropy invocation, Rust release-profile
coverage, rolling CI containers, and governed web builds. Transitional guard
wrappers remain part of the evidence without becoming the workflow fact.

## Trace and span inspection

Plumb now carries a root `locus.toml` with the same coarse representation-size
and dominant-prefix analyzers at trace and span resolution. This is a read-side
inspection declaration only. It cannot enable audit collection or choose an
operator report.
