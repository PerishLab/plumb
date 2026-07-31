---
name: plumb
description: Use when building or operating a repository in this workshop — the laws it must hold, which of them a checker enforces today, and how to run plumb.
metadata:
  short-description: Laws, standing, and commands for workshop repositories
---

# plumb

`plumb` is the substrate a managed repository builds on and the doctor that
travels to it. This brief carries what the binary cannot enforce: the
principles the workshop is built on, the laws that follow from them, and — for
each law — whether a machine catches a violation or only you will.

## Jurisdiction

These laws bind only the spaces this tool manages. Its owner defines what
managed means, states that test here, and keeps it answerable by looking
rather than by running.

plumb manages a repository carrying `plumb.toml` at its root.

Settle jurisdiction before you act. Inside, read these laws first and hold
them. Outside, they are silent, and their silence is not worth remarking on.
Only an unsettled answer asks.

## Upstream

Repository: https://git.perish.top/PerishLab/plumb

Report defects, missing shapes, and unclear guidance there as issues. Use
`plumb skill status` to check stable without changing an installation. When a
newer stable release is available, run `plumb skill upgrade` and validate it.

Read `Standing` before trusting any clause to be caught for you. Run
`plumb doctor` in a repository before changing its shape.

## Principles

Five hold, and most specific questions fall out of them.

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

**Repetition is an ownership signal.** When several repositories repeat a
Plumb-shaped mechanism nearly verbatim, surface the repeated shape to the
caller and consider absorbing it at the substrate. Do not force an abstraction
whose closure is still unclear; a strong signal starts an ownership decision,
not an automatic rewrite.

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

**Audit.** Plumb's Locus observation surface inherits one downstream-owned,
typed environment section. Its master gate defaults muted and returns before
Locus bootstrap, collection, generation, or reporting. The skill names the
contract but does not choose whether, where, or what an operator observes. See
`references/audit.md`.

**Templates.** Values reach config text through one grammar: `{name}` against
the caller's own variable table, `{{` and `}}` for the literals, and refusal for
anything unknown, unclosed, bare, or empty. The grammar is the substrate's; the
variables are the tool's.

**Home and state.** A tool that keeps user state resolves one data home as an
ordinary cascade field, keeps machine-written records under `state/`, versions
their schema, and writes them whole. State is not a config surface. A record
alone does not prove ownership: acting on a path requires the registry entry
**and** a marker inside the path, both naming the tool.

**Release lanes.** A binary product declares only its identity, binaries,
targets, and genuine product assets in the root `plumb.toml`. Plumb owns build
discovery, archives, attachments, managers, capsules, storage, verification,
activation, smoke, source binding, and packport topology checks. Actions owns
the shared matrix, credentials, and sequencing; product workflows are thin
callers. Stable activation remains a separate capability and operation. Exact
may bind any selected branch; stable binds only `release/vX.Y.Z`, and its
commit must be packported into `main` before the next stable activation. The
release branch remains as a permanent frozen source and audit boundary. See
`references/lane.md`.

**Repository protection.** Every managed repository carries the same
byte-identical `main` and `release/**` baseline rules. Active release lines add
only the exact PREPARING or FROZEN rule derived from their state; settled lines
stay frozen and are never deleted. This is a prose-only operator obligation:
`plumb doctor` does not read Forgejo. Apply and read it back with a local
credential as specified in `references/protection.md`.

**Stable consensus.** Stable from the canonical release authority alone may
occupy default install seats and root manager entrypoints. Every other channel
is an exact immutable validation candidate with explicit task-isolated install
and bin paths. Stable default mutation is single-writer, ownership-proven,
staged, atomic, and monotonic unless an exact rollback is explicit. Promotion
embeds the exact candidate seal and requires the same source commit. See
`references/lane.md`.

**Repository shape.** Generic wrappers and repository-owned Git hooks are not
repository facts. Guard keeps one canonical workflow lane and runs Plumb plus
Ectropy directly; generic init, land, and release behavior belongs to the
substrate entrypoint that owns it. Transitional or product-specific wrappers
remain observed and must have a known role, but their absence is valid. Lanes
and layout follow the skeleton the workshop already demonstrates rather than
each repository's invention. Most of this clause is enforced; see below.

**Dependencies.** Direct first-party dependencies across the `@perish` JSR
scope and `perish` Cargo registry track live stable latest. Deno declarations
carry no version requirement; Cargo retains native requirement syntax. In both
ecosystems the exact lock resolution equals the latest non-prerelease,
non-yanked registry version. The compiled policy names registry authorities,
not package versions. Doctor reads registry, manifest, and lock evidence but
mutates none of them. Unread evidence is blind and blocks; stale or pinned
shape refuses. Same-workspace path edges remain a release-train concern rather
than a published dependency edge.

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

The catalog compiled into the matching binary is the complete source for
standing. Run `plumb rule list` to inspect it, or select the part relevant to
the current work:

```bash
plumb rule list --standing mechanized
plumb rule list --standing prose-only
plumb rule list --namespace site --tag site
plumb rule show structure.guard-lane-present
```

`mechanized` means the doctor owns an evaluator and may emit a verdict.
`prose-only` means the law is indexed but no finding or exit status follows
from it. `observed` is reserved for evidence gathered without a verdict.
`blind` is not a fourth standing: it means a mechanized evaluator ran but could
not read enough evidence. A prose-only rule is never blind merely because no
evaluator exists.

The catalog also names the evidence and owning layer. Namespace, tag, standing,
and owner selectors quick-fail when their vocabulary is unknown. Do not copy a
standing list into prose or infer completeness from a clean doctor report:
`clean` means this repository emitted no finding, while
`plumb doctor --json` reports whole-catalog coverage separately.

The release changelog gate remains release-local rather than a doctor rule.
`plumb release compile` enforces it for stable before capsule creation.

## Invocation

```bash
plumb doctor [ROOT]           # shape report; nonzero when out of true or blind
plumb rule list               # complete catalog; compose typed selectors
plumb rule show RULE_ID       # law, standing, evidence, owner, and tags
plumb rule namespaces         # registered namespace vocabulary
plumb rule tags               # registered tag vocabulary
plumb rule owners             # registered owner vocabulary
plumb lock [ROOT]             # print what a fresh affirmation would record
plumb changelog [ROOT]        # refuse a version whose en+zh changelog is absent or empty
plumb policy [ROOT] --write   # reconcile ectropy.toml against the repository
plumb release authority       # print the product's canonical public release authority
plumb release matrix          # derive the shared target matrix from plumb.toml
plumb release build           # build and archive one declared target
plumb release assemble        # gather targets and build declared attachments
plumb release source          # bind the frozen event branch, commit, channel, and version
plumb release compile         # seal one exact declared product artifact set
plumb release publish         # publish immutable objects and the exact seal
plumb release activate        # move stable consensus with its separate authority
plumb release inspect         # verify an exact seal or stable public surface
plumb release smoke           # exercise a generated manager on this platform
plumb release packport        # prove a stable commit is now an ancestor of main
plumb skill install           # install this brief into detected agent directories
plumb skill stage             # unpack one exact candidate into a new explicit path
plumb skill status            # compare managed installs with stable, read-only
plumb skill upgrade --dry-run # print the exact stable movement without applying it
plumb skill upgrade           # move managed installs to the selected version
plumb skill list              # show managed installs
plumb skill uninstall         # remove only what is recorded as managed
```

Install, status, and upgrade admit stable only; `--version` pins an exact
stable release, while no version resolves the stable pointer. `--path` installs
to one explicit managed directory ending with `plumb`. `--force` replaces only
a seat proved by the ledger and marker. Upgrade leaves current seats untouched,
refuses an implicit rollback or same-version artifact drift, and treats an
explicit older stable version as deliberate rollback.

`stage` requires `--channel`, `--version`, and `--path`. Its channel must be
non-stable, its version exact, and its target absent and ending with `plumb`.
It writes a staged marker but never the managed ledger. The caller owns cleanup
of the surrounding isolated root.

The binary is the truth about its own flags: prefer `plumb <command> --help`
over assuming.
