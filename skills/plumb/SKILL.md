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
- A **stable line** is the explicit prepare, pick, freeze, publish, and rejoin
  lifecycle for one permanent version. Freeze stamps its point and `retract`
  removes that point while nothing is published; a served seal refuses it.
- A **datum** is the stable answers a release line judges against, recorded by
  `prepare` under `.plumb` and read there by `doctor` in place of a live
  registry, so the frozen candidate keeps the verdict it was proved with.
- A **depot** is Plumb's store for the configuration it would otherwise only
  compile in: rules and lane templates as immutable objects under a timestamped
  version, plus one movable channel pointer naming the current one. A synced
  seat answers before the compiled bytes, and the compiled bytes stay as the
  permanent floor, so a repository that never syncs is not thereby blind.
- A **site** is a declared application whose deploy, binding, and reachability
  are independently evidenced.
- A **retirement** destroys one declared delivery chain in a fixed order; it is
  refused unless the product declares `[release.retire]` and every confirmation
  equals its target verbatim.
- A **skill seat** is one source brief or one ownership-proven installed brief.
- A **document strategy** owns its target files, evidence mass, invalidation
  lifecycle, and text magnitude; none of those are downstream parameters. Its
  seal projects the Git index, so an untracked leaf under a declared source
  seat is out of true and proposes nothing.

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
plumb workflow status [ROOT] [--since REV]
plumb workflow ask LANE [ROOT]
plumb workflow hash|lock KEY [ROOT]
plumb changelog [ROOT]
plumb depot publish [ROOT] [--version FLOOR] [--dry-run]
plumb depot changelog [ROOT] --version VERSION [--from DIR] [--keep] [--dry-run]
plumb depot sync
plumb depot show
plumb release --help
plumb ship --help
plumb release prepare|pick|freeze|rejoin|stamp|retract --version VERSION [--dry-run]
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
- Judge a release by its run graph, never by the dispatch that started it. A
  watch holds until every job is terminal and refuses an empty or failed graph.
- Record document seals only after reading every named source and target.
  `plumb document` prints proposals and never rewrites `plumb.toml`.
- A projection never skips. Input hashes are evidence of what moved, not a gate
  on what ships, so a release projects every declared object at its own version
  and an unreadable baseline changes nothing but the evidence.
- A ship object is one attachment: `cargo`, `npm`, `chart`, `cfworker`. Every
  package an attachment declares carries one digest, one version, and ships
  together. `[release.depends]` keys and a seal's inputs use those names.
- Re-running a publish is safe and is also a check: where the registry answers
  with a digest, the adaptor compares it against the bytes it just built and
  refuses with `published <medium> drift` rather than reporting a success it
  did not verify.
- How wide an attachment may be is Plumb's law, held in its rules directory
  beside the forge image. Declaring above the width Plumb has released is
  noted, never refused; declaring above the width Plumb permits is refused.
- A worker is a release medium, not a side channel. `[release.cfworker]` names
  the account and the stable domain; exact channels stage a version behind its
  own preview URL and only stable reaches the declared domain.
- `--dry-run` does every read and no write, so it needs credentials and the
  network. It prints the steps that would run against the remote as it stands
  now, and refuses instead of printing a plan it could not verify.
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
- A release note is a seat in the depot, one known address per released
  version, and it is mutable because ship is not: identity is paid once at the
  version and its seal, so a note about that version can only be improved.
  Publishing refuses a language pair that is empty or above its diff-derived
  budget; compiling a release checks neither, so a note can never fail a
  release. Notes stage in the repository's temporary seat and are cleared once
  the depot holds them, so they never enter the tree.
- The depot is read from the held seat and never fetched, so a stale seat is an
  observation and a seat that cannot be read is blind. Two laws stand on it: the
  roots this repository records are carried by the held version under the same
  digest, and the running binary is at or above the floor that version declares.
  Publishing refuses roots carrying uncommitted change.
- Stable default skill seats accept canonical stable only. Validate every
  other candidate in an exact, isolated stage path.
- A governed repository declares `[[document]]` strategies. Unknown
  strategies, retired document declarations, arbitrary targets, and numeric
  limits refuse.

Use [PATHS.md](PATHS.md) for routine flows and
[SCENARIOS.md](SCENARIOS.md) only when one of its bounded cases applies.
