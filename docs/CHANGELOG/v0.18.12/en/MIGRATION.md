# Migrating to Plumb v0.18.12

This release requires no repository migration.

`plumb stable prepare` now refuses an existing release line that stands at a
different commit, where it previously reported success without moving it. A
caller that relied on the earlier reply to mean the line matched the base
should read the new message: `held` means the line already stands at the base,
and a refusal names both commits.
