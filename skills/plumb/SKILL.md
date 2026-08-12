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
- A **document** is one closed target strategy, its exact source bindings, and
  the seals recorded at the last human reading.
- A **release** is an immutable product identity and declared artifact set.
- A **stable line** is the explicit prepare, pick, freeze, publish, and packport
  lifecycle for one permanent version.
- A **site** is a declared application whose deploy, binding, and reachability
  are independently evidenced.
- A **retirement** destroys one declared delivery chain in a fixed order; it is
  refused unless the product declares `[release.retire]` and every confirmation
  equals its target verbatim.
- A **skill seat** is one source brief or one ownership-proven installed brief.
- A **document strategy** owns its target files, evidence mass, invalidation
  lifecycle, and text magnitude; none of those are downstream parameters.

## Actions

```bash
plumb doctor [ROOT]
plumb rule list
plumb rule show RULE_ID
plumb precommit [ROOT] --base BASE --head HEAD --write PATH
plumb land [ROOT] [--base BASE] [--dry-run]
plumb document [ROOT]
plumb radius --root [LABEL=]PATH --product PRODUCT --candidate VERSION
plumb policy [ROOT] --write
plumb changelog [ROOT]
plumb release --help
plumb stable --help
plumb site --help
plumb retire [--root ROOT] [--execute --confirm-repo R --confirm-bucket B --confirm-domain D]
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
- Record document seals only after reading every named source and target.
  `plumb document` prints proposals and never rewrites `plumb.toml`.
- Stable default skill seats accept canonical stable only. Validate every
  other candidate in an exact, isolated stage path.
- A governed repository declares `[[document]]` strategies. Unknown
  strategies, retired document declarations, arbitrary targets, and numeric
  limits refuse.

Use [PATHS.md](PATHS.md) for routine flows and
[SCENARIOS.md](SCENARIOS.md) only when one of its bounded cases applies.
