# Migration

Repositories already migrated under v0.18.19 require no file change.

Before running v0.18.20 Doctor, any repository that skipped the compatibility
release must first follow the v0.18.19 migration document and platform script,
review every proposed source and target, record the resulting `plumb document`
seals, and reach a clean v0.18.19 Doctor result. v0.18.20 does not accept or
rewrite the retired declarations and does not provide a replacement command
for generic locks.

Doctor JSON consumers must stop reading `shape.strategy` and `shape.skills`.
Read `shape.documents`; named `brief` entries carry the governed skill target,
source count, text, budget, and leaf count.

Rule-catalog consumers must remove the retired `lock.*` and skill shape-rule
identifiers. Document schema, admission, evidence, and magnitude now own the
complete source-skill contract.
