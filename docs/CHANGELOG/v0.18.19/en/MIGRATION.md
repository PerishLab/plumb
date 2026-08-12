# Migrating to Plumb v0.18.19

Run the platform migration script from this directory. It removes legacy
`[skill]` and `[[lock]]` tables, declares an unaffirmed root `agent` binding,
and declares one unaffirmed `brief` binding for every exact three-file skill
seat. The broad root source is deliberate migration evidence, not a fabricated
claim of review.

After the script, classify or remove every Markdown file reported by Doctor.
Current operating law belongs in `AGENTS.md`; current cross-boundary topology
may earn `ARCHITECTURE.md`; product doctrine may earn `DESIGN.md`; agent
operations belong in an exact three-file skill; release history stays under
`docs/CHANGELOG`; unsettled work stays outside the repository.

Read each declared source and target, run `plumb document .`, and manually
record the proposed seals. Do not copy proposals without review and do not add
exclusions, arbitrary target paths, or local numeric limits.
