# Migrating to Plumb v0.18.17

Existing repositories require no migration.

A product that needs immutable version-owned files may add direct regular files
under `docs/CHANGELOG/v<base-version>/artifacts/`. Plumb deliberately does not
define their filenames or semantics.
