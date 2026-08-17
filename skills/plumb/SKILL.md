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
- A **projection** renders one release onto one medium. The adaptors are a
  closed set, and all six are absorbed: `binary`, `site`, `cargo`, `oci`,
  `chart`, and `npm`. Publishing one carries the compiled capsule wherever the
  release carries a seal, so a sealed version with no published seal has
  projected nothing. A release declaring only attachments compiles no capsule
  and is bound instead by the registries that receive it.
- Release activation advances the stable consensus pointer, while binary
  activation shifts generated managers. Their inspect deeds likewise prove
  release identity and the binary projection separately.
  `release` keeps the truth cycle and never projects.
- A **stable line** is the explicit prepare, pick, freeze, publish, and packport
  lifecycle for one permanent version. Freeze stamps its point and `retract`
  removes that point while nothing is published; a served seal refuses it.
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
plumb lane [ROOT] [--write]
plumb changelog [ROOT]
plumb release --help
plumb ship --help
plumb stable prepare|pick|freeze|packport|retract --help
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
- A skip is an optimisation, never a gate. An unreadable baseline projects
  everything and says so; it does not refuse the release. A settled crate keeps
  the release it last changed in, and every requirement on it names that one.
- A ship object carries the name of its medium, plus `/<package>` where one
  medium holds many: `cargo/<crate>`, `npm/<package>`, `chart`, `cfworker`.
  `[release.depends]` keys and a seal's recorded inputs use those names.
- A worker is a release medium, not a side channel. `[release.cfworker]` names
  the account and the stable domain; exact channels stage a version behind its
  own preview URL and only stable reaches the declared domain.
- Never hand-edit a governed workflow. `plumb lane --write` renders it from the
  declaration; doctor notes drift instead of refusing it, so a stale lane stays
  visible without reddening the repository. A rendered lane carries no condition:
  the job set follows the declared surface, and matrices only size a job kind.
  Dispatch refuses a lane this Plumb rendered and someone then edited, and the
  absence of the one lane it must dispatch; every other unrendered lane only
  draws the note, so adoption stays incremental.
- A rendered lane is refused at render when it carries a shape the forge cannot
  run: secrets under `workflow_call`, an empty matrix, an installed tool absent
  from the job path, or a release lane that follows a push. Only the forge parses
  a lane body, so a local check must stand where the text is written.
- Stable default skill seats accept canonical stable only. Validate every
  other candidate in an exact, isolated stage path.
- A governed repository declares `[[document]]` strategies. Unknown
  strategies, retired document declarations, arbitrary targets, and numeric
  limits refuse.

Use [PATHS.md](PATHS.md) for routine flows and
[SCENARIOS.md](SCENARIOS.md) only when one of its bounded cases applies.
