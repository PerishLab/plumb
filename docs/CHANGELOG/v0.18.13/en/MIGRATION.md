# Migrating to Plumb v0.18.13

This release requires no repository migration.

Operator scripts that parse the exact failure suffix from
`plumb release dispatch --watch` should replace `failed jobs:` with
`failed tasks:`. The named entries now come from Forgejo's action-task surface
and may include failed, cancelled, or blocked tasks.
