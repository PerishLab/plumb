---
name: plumb
description: Build, inspect, prove, release, and land a repository governed by a root plumb.toml.
metadata:
  short-description: Objects and authorities for governed repositories
---

# Plumb

Plumb governs a repository carrying `plumb.toml` at its root. Outside that
boundary this brief is silent. Read the repository's own `AGENTS.md`, then run
Doctor before changing its shape and again after.

Four surfaces answer: the source says what a thing does, `--help` says how to
use it, `plumb cookbook` says what to do when something fired, and this brief
says what none of those can. Ask the one that owns the question.

## Objects

- A **repository** is the root selected by `plumb.toml`.
- A **shape** is the evidence Doctor reads from that repository.
- A **finding** is one verdict: `out of true`, `unknown shape`, or `blind`.
  The first and the last make Doctor nonzero; the middle one is visible and
  claims no law was broken.
- A **rule** is one catalogued law. Its **standing** is `mechanized`,
  `observed`, or `prose-only`; only the first carries a verdict.
- A **seat** is where something may sit. `[layout]` declares seats, the anchors
  a member carries to earn one, and the rules its members answer to. A
  repository declaring no layout is judged by the released name sets instead.
- A **boundary** proves one committed delta stays inside declared write paths.
- A **landing** projects a clean topic branch onto its base and waits for guard.
- A **release** is an immutable product identity; a **projection** renders one
  onto one medium.
- The **depot** carries Plumb's own configuration as immutable objects. Rules
  exist only in a verified synced seat; they are never compiled into the binary
  and reading them never fetches. Installation syncs the seat, while an absent,
  drifted, or unsupported seat refuses rule-dependent commands and names
  `plumb depot sync` as the remedy.

## Authorities

Ask, never recall:

```bash
plumb <command> --help     # every flag and contract
plumb rule list|show       # the current law, its standing and evidence
plumb doctor . --json      # this repository's shape and findings
plumb layout .             # the seats this repository declares
plumb cookbook [entry]     # what to do about a finding that names one
```

`--help` answers before you act and covers every command; a cookbook entry
answers after something fired and exists only where the finding does not already
tell you the move.

Copying a standing list into prose is how a brief starts lying. This one holds
objects and points at authorities; it states no flag and no law.

## Laws it will not repeat

Refuse missing or unread evidence; never infer or repair it silently. Run the
repository's own guard before landing. Record nothing a source did not prove.

Use [PATHS.md](PATHS.md) for routine flows and [SCENARIOS.md](SCENARIOS.md)
only when one of its bounded cases applies.
