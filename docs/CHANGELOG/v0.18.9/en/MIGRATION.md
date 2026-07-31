# Migrating to Plumb v0.18.9

This release requires no repository migration.

Coordinators may adopt `plumb precommit` or the `plumb::boundary` API when they
are ready to supply exact base/head OIDs and explicit write prefixes. The
bundled retired dictionary is empty, so Doctor introduces no vocabulary cleanup
in this release.
