# Migrating to Plumb v0.30.0

## Re-render the Ectropy policy

Run `plumb depot sync`, upgrade Plumb, then run `plumb policy --write` and
`ectropy .`. The default path limit changes from 4 to 3. Do not raise it back
globally: flatten an unearned level or add a narrow boundary whose note names
the task that will retire it.

## Validate release declarations locally

Run `plumb doctor . --json` before opening a release line. Doctor now refuses a
release table that the release commands cannot parse and names attachments no
called lane can deliver. TOML keys intended for `[release]` must appear before
an attachment table; the parser, rather than convention, is the authority.

## Expect network reads from stable line gates

`plumb release prepare` and `plumb release freeze` now fetch origin before
checking stable ancestry, including under `--dry-run`. The commands still make
no remote writes in dry-run mode, but they require the remote to be readable.

If the prior stable point is absent from current main, run the explicit
`plumb release rejoin --version <version>` flow and inspect its topology before
opening or freezing the next line.

## No command or library migration is required

The 17 top-level command contracts, 121 rule identifiers, public library API,
and rendered policy shape are the v0.30 compatibility boundary. This release
does not require downstream source changes beyond accepting the path budget.
