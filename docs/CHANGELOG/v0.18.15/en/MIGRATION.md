# Migrating to Plumb v0.18.15

Every repository carrying a `skills/` source seat must add this declaration to
its root `plumb.toml`:

```toml
[skill]
strategy = "brief"
```

The `brief` strategy requires each `skills/*` seat to contain exactly
`SKILL.md`, `PATHS.md`, and `SCENARIOS.md`. It applies one aggregate budget of
`clamp(2 * ceil(sqrt(source lines)), 120, 400)` Markdown lines. Remove other
root entries and bring the aggregate text within that derived budget before
upgrading.

Do not declare file names, a numeric limit, or formula parameters. Those are
owned by the selected strategy and direct fields are rejected.
