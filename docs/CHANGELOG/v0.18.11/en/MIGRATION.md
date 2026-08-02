# Migrating to Plumb v0.18.11

This release requires no repository migration.

A repository still carrying a generic `land` wrapper may drop it and call
`plumb land` directly. The wrapper is not removed for you, and Doctor does not
yet report its presence.

Coordinators may call `plumb::land::run` with a repository path. It resolves the
Forgejo remote from `origin`, reads its token through the existing cascade and
Tea login seats, and reports the pull request, the projection, and the synced
base seat.
