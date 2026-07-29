---
name: plumb
description: Use when building or operating a repository in this workshop — the laws it must hold, which of them a checker enforces today, and how to run plumb.
metadata:
  short-description: Laws, standing, and commands for workshop repositories
---

# plumb

`plumb` is the substrate every repository here depends on and the doctor that
travels to them. This brief carries what the binary cannot enforce: the
principles the workshop is built on, the laws that follow from them, and — for
each law — whether a machine catches a violation or only you will.

## Upstream

Repository: https://git.perish.top/PerishLab/plumb

Report defects, missing shapes, and unclear guidance there as issues. Use
`plumb skill status` to check stable without changing an installation. When a
newer stable release is available, run `plumb skill upgrade` and validate it
before preserving compatibility with an older managed installation.

Read `Standing` before trusting any clause to be caught for you. Run
`plumb doctor` in a repository before changing its shape.

## Principles

Four hold, and most specific questions fall out of them.

**Mechanism at the substrate, vocabulary with the product.** plumb exports the
derive, the doors, the template grammar, the install machinery. It exports no
config sections, no backend names, no manifest keys. A shape frozen here is a
closed set at the one layer that cannot open it — which backends exist and what
a section holds are properties of the binary that owns them, expanding under
pressure the substrate never feels.

**The right to amend travels with the layer.** A law about repository shape
lives with plumb because plumb can be amended when the shape must change. A law
about a product's own sections lives with the product. Putting either in the
other's house means the party who needs the change cannot make it.

**Refusal over repair.** A policy half-read is a policy misread. Malformed
config refuses to boot; an unknown template variable refuses to resolve; a state
record with an unknown schema refuses to load; a path that is not provably ours
refuses to be replaced. None of these guess, and none silently repair.

**The wall stands until the check does.** A law is written before it is
mechanized. A page that says so — and says it plainly — is the enforcement
until the check lands. Claiming otherwise is worse than silence.

## Laws

Detail lives in the reference files; these are the clauses themselves.

**Config.** Runtime policy enters through one cascade: default, file,
environment, arguments, in that fixed order, with the default layer total. Two
doors and no third — a parsed file and the typed environment. A direct read of
the environment bypasses both; so does a key the cascade cannot derive. Tests
are the granted territory. See `references/laws.md`.

**Templates.** Values reach config text through one grammar: `{name}` against
the caller's own variable table, `{{` and `}}` for the literals, and refusal for
anything unknown, unclosed, bare, or empty. The grammar is the substrate's; the
variables are the tool's.

**Home and state.** A tool that keeps user state resolves one data home as an
ordinary cascade field, keeps machine-written records under `state/`, versions
their schema, and writes them whole. State is not a config surface. A record
alone does not prove ownership: acting on a path requires the registry entry
**and** a marker inside the path, both naming the tool.

**Release lanes.** A lane has an anatomy, and every clause in it was bought
with an incident: resolve metadata that refuses regressions and same-version
reruns, guard fresh rather than incremental, stamp the manifest the publisher
actually reads, assert the dry run names the version **before** the
irreversible step, publish idempotently, verify by reading back from the
registry, then seal. Rehearse the lane when nothing has exercised it — a lane
that has not run since a rename is already broken. See `references/lane.md`.

**Repository shape.** Wrappers, hooks, lanes, and layout follow the skeleton
the workshop already demonstrates rather than each repository's invention. Most
of this clause is enforced; see below.

**Sites.** An app declaring a site ships through one dispatch-only lane calling
the same wrapper an operator runs, with a purpose-scoped key the lane holds and
the operator does not. The build stamps its commit into the health document and
declares its routes in the artifact — a sitemap, or the prerendered documents
themselves; the shipper reads those and never application source. A deploy
reports three separate states — deployed, bound, reachable — each proven on its
own. Binding admits `unknown`, because a credential that cannot ask has not
learned `no`, and an unprovable deploy says so rather than implying success
(`docs/site.md`).

**Changelogs.** A stable release carries
`docs/CHANGELOG/v<version>/<lang>/{INDEX.md, MIGRATION.md}` before it goes out,
because afterwards the release is immutable and there is nowhere to put it. `en`
and `zh` are the floor and not the set. A release requiring nothing of anyone
still writes MIGRATION.md saying so — that sentence is a conclusion somebody
reached, and a file nobody wrote is not the same artifact. Prereleases are
exempt, and the rule does not reach back past the version that introduced it
(`docs/changelog.md`).

**Locks.** A pair bound across a boundary no compiler crosses — a brief and the
binary it describes, a chart and an environment key — is declared in
`plumb.toml` with the version and hash of its last reading. Drift on either
side refuses. Affirmation is a human act and never automatic
(`references/laws.md`).

## Standing

What `plumb doctor` catches today:

- missing guard, init, land, or release wrappers, and missing git hooks
- a guard wrapper that does not run `plumb doctor`; an init that does not
  require plumb
- a missing or unreadable `ectropy.toml`
- a guard lane without its concurrency block; a CI container pinned to a tag
- an edition other than 2024
- a publishable package at the root instead of under `packages/`, or one whose
  directory does not match its name
- a rust binary shipping without clap or without plumb
- a `Cascade` derive outside the anchor crate; a repository with no crate named
  after itself
- a release wrapper without its lane, or a lane without its wrapper
- a dependency renamed upstream; a self-built dependency pinned to a version
- a boundary naming a path that does not exist; a `.runseal` test that belongs
  in sealkit
- `Cargo.toml` without `target/` in `.gitignore`
- a declared lock whose covered paths changed, or whose affirmed version has
  been left behind by the repository
- an unknown workflow: the skeleton now knows the deploy lane by name
- an app declaring a site without a ship wrapper or without a deploy lane

What the **release lane** catches, which is not the same as the doctor catching
it — the working tree stays green and the refusal arrives only when you try to
publish:

- a stable release whose `docs/CHANGELOG/v<version>/` is missing any of
  `{en,zh}/{INDEX.md,MIGRATION.md}`, or holds one that is empty. Today only
  plumb's own lane carries this step; a repository adopts it by adding
  `plumb changelog . --version "$RELEASE_VERSION"` before its first irreversible
  step, and nothing forces that adoption.

What is **prose only** — no machine will stop you:

- the config vocabulary rule: importing section shapes from the substrate
  instead of declaring your own
- template variables and their spelling in manifests
- the home and state clauses, including the two-sided ownership proof
- every clause of the release lane anatomy, and every clause of the site lane
  below the pairing above: nothing checks that the route is bound in the
  repository, that the shipper reads the artifact instead of source, or that a
  readback happens at all
- the accuracy of this section in any skill, including this one — a lock makes
  someone re-read it when the version moves, which is attention, not proof

When a walled clause becomes mechanized, its line moves up and its prose
shrinks to the check's name. Which of the three lists it moves into matters:
saying the doctor catches something the release lane catches is still a false
claim, because it tells a reader their working tree will warn them and it will
not. A clause claimed as enforced that is not is the one defect this document
cannot afford.

## Invocation

```bash
plumb doctor [ROOT]           # shape report; nonzero when something is out of true
plumb lock [ROOT]             # print what a fresh affirmation would record
plumb changelog [ROOT]        # refuse a version whose en+zh changelog is absent or empty
plumb policy [ROOT] --write   # reconcile ectropy.toml against the repository
plumb skill install           # install this brief into detected agent directories
plumb skill status            # compare managed installs with stable, read-only
plumb skill upgrade --dry-run # print the exact stable movement without applying it
plumb skill upgrade           # move managed installs to the selected version
plumb skill list              # show managed installs
plumb skill uninstall         # remove only what is recorded as managed
```

Install and upgrade take `--channel` (default stable) and `--version` to pin a
published version; `--path` installs to one explicit directory, which must end
with `plumb`. `--force` replaces an install that is already managed, and never
touches a path that is not; upgrade already owns what it replaces, so it needs
no flag and still refuses a path that is not provably managed. Status and
upgrade dry-run fetch metadata but never the skill artifact and never write
state. Upgrade leaves current seats untouched, refuses an implicit rollback or
same-version artifact drift, and treats an explicit older `--version` as a
deliberate rollback. Stable may resolve through latest metadata; every
non-stable channel requires an exact `--version`.

The binary is the truth about its own flags: prefer `plumb <command> --help`
over assuming.
