---
name: plumb
description: Build, inspect, prove, release, and land a repository governed by a root plumb.toml.
metadata:
  short-description: Objects and actions for governed repositories
---

# Plumb

Plumb governs a repository carrying `plumb.toml` at its root. Outside that
boundary this brief is silent. Read the repository's own instructions, then
use Doctor before changing its shape.

## Objects

- A **repository** is the root selected by `plumb.toml`.
- A **shape** is the evidence Doctor can read from that repository.
- A **rule** is one catalogued law with an owner, evidence, tags, and standing.
- A **finding** is one rule verdict: `out of true`, `unknown shape`, or `blind`.
- A **standing** is `mechanized`, `observed`, or `prose-only`.
- A **boundary** proves one committed delta stays inside declared write paths.
- A **landing** projects a clean topic branch onto its base and waits for guard.
- A **lock** binds declared files to the version and hash of their last human
  reading.
- A **release** is an immutable product identity and declared artifact set.
- A **stable line** is the explicit prepare, pick, freeze, publish, and packport
  lifecycle for one permanent version.
- A **site** is a declared application whose deploy, binding, and reachability
  are independently evidenced.
- A **skill seat** is one source brief or one ownership-proven installed brief.

## Actions

```bash
plumb doctor [ROOT]
plumb rule list
plumb rule show RULE_ID
plumb precommit [ROOT] --base BASE --head HEAD --write PATH
plumb land [ROOT] [--base BASE] [--dry-run]
plumb lock [ROOT]
plumb radius --root [LABEL=]PATH --product PRODUCT --candidate VERSION
plumb policy [ROOT] --write
plumb changelog [ROOT]
plumb release --help
plumb stable --help
plumb site --help
plumb skill --help
```

The binary is the authority for flags. Use `plumb <command> --help` before an
unfamiliar or stateful action.

## Operating laws

- Refuse missing or unread evidence; never infer or silently repair it.
- Run Doctor before shape changes and again after them.
- `out of true` and `blind` make Doctor nonzero. `unknown shape` is visible but
  does not claim a known law was violated.
- `observed` gathers evidence without a verdict. `prose-only` remains a human
  obligation. Never describe either as mechanized.
- Query the catalog for the current law instead of copying a standing list.
- Keep product declarations limited to product identity and genuine inputs;
  Plumb owns the shared mechanism derived from them.
- Keep generated managers, capsules, archives, and staged installs out of
  source control.
- Use `plumb land`; do not recreate its projection and guard protocol in a
  repository wrapper or hook.
- Affirm a lock only after reading both sides of the declared binding. Lock
  output is a proposal, never an automatic rewrite.
- Stable default skill seats accept canonical stable only. Validate every
  other candidate in an exact, isolated stage path.

Use [PATHS.md](PATHS.md) for routine flows and
[SCENARIOS.md](SCENARIOS.md) only when one of its bounded cases applies.
