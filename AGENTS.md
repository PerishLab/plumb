# Agents

This repository is the workshop's living skeleton. `ectropy .` must print
`clean` before anything lands, and CI runs the complete repository guard in
explicit fail-fast order. Ectropy has one severity: every finding is an error.

Four surfaces answer, and knowing which one to ask is most of knowing this
repository. **The source** says what a thing does and why it exists; it is the
only one that cannot be wrong. **`--help`** says how to use it, and covers every
command. **`plumb cookbook`** says what to do when something fired, and covers
only the findings whose remedy the finding itself does not give. **This file**
says what none of the other three can, which is why it is short and why anything
belonging to them is removed from it on sight. Ask the surface that owns the
question rather than trusting the nearest prose.

## The relation

plumb holds to the constitution ectropy enforces, and inherits it for shapes.
If a repository in this ecosystem has no shadow here, plumb owes the shape:
divergence is an error in the skeleton, never an exception in the repository.

Guard is a staged-tree proof, not a repository workflow. Plumb carries the
pre-commit and commit-message hooks in its binary; `configuration install`
projects them and Doctor requires their presence.
Guard stages proof state below `PLUMB_HOME` and carries the compact proof in Git history. Land and
release marker creation refuse a tree without that exact proof; an unchanged
action world is reused and never starts a process.

Downstream repositories do not get scanned by plumb, and plumb does not know
they exist. The CLI travels to them: install it, run it in a repository, read
what it reports. Feedback comes home as issues on this repo. That is the hot
link, and it is the only one.

## Layout

These are the nodes `[layout]` has not converged on. Each leaves this list when a
seat can state it; nothing belongs here that a seat, a rule, or a verb already
answers for.

- `crates/lib/src/forge/{land,github.rs}` — land projects a guarded candidate
  onto GitHub through the `gh` CLI the caller's Runseal profile authorizes.
  Plumb holds no GitHub credential. The required `guard` status is Plumb's
  verified Guard proof, and the merge must match that exact candidate.
- `crates/cli/{rules,assets,cookbook,help}` — Plumb's governed resources,
  carried inside the binary. A release is its own rule set: nothing is fetched,
  installed or overlaid at run time, and changing a rule is a change to Plumb.
  `rules/` holds four things and the compiler holds it to them:
  `catalog.toml` says what each rule requires, `taxonomy.toml` the words it is
  filed under, `atoms/` each judgement a mechanism makes, and `suites/` which
  atoms a position selects. An atom is self-contained and names the mechanized
  rule it judges; a suite only selects, by `rule://<atom>/<member>`, and
  carries no assertion of its own. Anything that judges nothing and selects
  nothing is the binary's own and lives in source. `plumb::seat::catalogue!`
  reads that directory where it is carried, refuses every departure from it,
  and emits what `catalog/carried.rs` used to list by hand. A rule that stops
  being judged leaves the catalogue; the commit that removes it says why. Each
  is one file per addressable thing, and the address is the
  path: a rule set is its name, a cookbook entry is the finding that sends you
  there, and long help is the command path, so `plumb release` reads
  `help/release.txt`. Prose an agent reads as contract is carried, never
  inlined in a literal. A debug build alone reads a rule overlay below an
  explicit `PLUMB_HOME`, so tests can exercise a mechanism with rules of their
  own; the overlay moves the rule digest, so a proof made under it never
  verifies against a released binary.
- `apps/web` — the Svelte specimen the web shape checks. Its seat declares no
  anchor, so nothing yet states what an app must carry to earn one.

## Boundaries

- LIVING SPECIMENS ONLY. An archetype held here must be really built and really
  published, or it is not exercised and will rot exactly like boilerplate. What
  is described but not run must say so.
- CHECKING BEFORE SCAFFOLDING. `check` ships before `new`. A generator encodes
  guesses; a diff harvests facts, and the ecosystem already holds the facts.
- EVERY ELEMENT STAYS REMOVABLE. Encoding combinations is the point — many
  choices here are only defensible together, not alone — but no element may
  become unremovable, or its justification decays from finding to story.
- NOTHING INVENTS A LOCATION. `plumb::seat` anchors at run time;
  non-rule fallback resources may use `plumb::seat::resource!` at compile time,
  so a caller states what it wants rather than where it sits.
- plumb MUST PASS ITSELF. Running the CLI here has to come back clean, or the
  relation above does not hold for the one repo that declares it.

## Documents

- Everything a repository knows belongs where it can be judged or where it can be
  found: in the code, in a rule set, in a note beside the key it is about, or in
  the Concord task that owns the work. A document nobody must read and no
  mechanism keeps true is the worst of the four.

## Distribution

- `release` only stamps its immutable marker on the head of a release line.
  `ship` hands that marker to wharf, which publishes every medium. `depot
  consign` places a marker's changelog or skill in wharf's yard, dispatches
  wharf to lodge it, and reads Depot back; wharf is Depot's only writer.
  Neither is a phase inside the other.
- Artifact reuse and distribution identity are separate domain-wide concerns,
  across binaries and registry media. Plan must key reusable content by its
  actual build inputs and tool world, and key publication by that content plus
  the validated marker and binding implementation. Identity-only changes must
  not repeat unaffected builds or proofs; binding and installation checks still
  run when their inputs change. Ignoring a version field is not proof that it
  cannot affect compiled behavior.
- Distribution is wharf's. It binds reused content to the explicit marker
  without rewriting its build provenance, mutating cached originals, or
  replacing published objects, and it owns packaging, signing, readback and the
  channel pointers. Reuse does not imply that a versioned package is
  byte-identical or that a mutable registry tag proves immutable identity.
- Plumb and plumb-lib must absorb the shared identity protocol and each medium's
  binding mechanics. Applications retain their own commands, but version output,
  Doctor, dispatch, and Depot consumers must agree on one bound identity. A
  missing or invalid binding must never masquerade as stable. Downstream repos
  must not grow identity files, patch scripts, workflow copies, or format knobs.
  For Rust binaries, the selected direction is a reserved embedded identity
  region bound after compilation and before final signing. This is a convergence
  contract, not a claim of completed support: platform retention, optimized
  runtime reads, signing, and consumer acceptance must be proved before release.
- Doctor never calls a repository true where it read nothing. A repository
  without plumb.toml declares nothing, so Doctor says that and stands blind
  rather than clean, which is what Guard already says when it refuses to prove
  a staged tree there. An undeclared ectropy policy reads as undeclared too,
  not as limits of zero.
- A release line is opened by `release open` and closed by `release close`;
  a stable marker leaves obligations that are paid before the next marker:
  its distribution completed (`ship dispatch`, read from wharf's
  `distribution.json`, the one authority on how far a marker is distributed,
  which is also what a stable stamp reads for the prerelease it promotes),
  its merge into main (`release rejoin`), its changelog on Depot and its skill
  where one is declared (`depot consign`), and its line closed. `command/operator/owed` is their
  table; each row names its rule, the command that settles it and the detector
  that reads it, `release owed` reports it, and `release open`, `release
  stamp` and `ship dispatch` refuse any later version while one is owed. A settlement never ships without its detector, nor
  a detector without its settlement.
- Wherever a marker has a distribution record, it alone says how far the marker
  is distributed: `ship status` reads it, `ship dispatch --watch` judges its run
  by it, and `release retract` refuses a marker it shows anything public for. A
  marker without one — older than the record, or one whose record was lost — is
  judged by its seal, conservatively.
- A marker names identity, not capability. Reading a historical marker does not
  authorize producing its targets today; wharf refuses what the product's own
  manifests no longer support.
- Depot carries changelog and skill generations and nothing else;
  configuration travels inside the binary. Generations remain immutable and
  addressable. A latest pointer may move, but every movement names the release
  marker and uses conditional readback. A product's own `plumb.toml` declares
  whether it owes a skill (`skill = true`) and, where it differs from
  `https://depot.<product>.perish.uk`, its depot source; there is no central
  product catalogue. Changelog and skill source trees are caller-owned temporary
  media passed explicitly with `--dir`, never repository or `PLUMB_HOME` seats:
  `skills/*` is a retired seat, and a skill is written for its release on top
  of the previous generation. Products
  consume a skill through their own command surface while delegating its exact
  generation and digest binding to `plumb` the library.
- The distribution workflow lives in wharf, which sits above Plumb. This
  repository carries no workflow copy and no distribution secret: `plumb ship
  dispatch` hands wharf a marker, and wharf is the only writer.

- Delivery reuse includes execution preparation, not only business builds.
  A proven workload must not acquire an unrelated build prerequisite merely
  because binding uses a native runner. Check available prerequisites before
  expensive work; recovery preserves completed evidence and retries only the
  affected work. Measure the complete delivery path, including preparation,
  queueing, and readback; cache hits alone do not prove efficient delivery.

The verbs are `plumb release --help`, `plumb ship --help`, and
`plumb depot --help`. The laws are
`plumb rule list --namespace release`; they state the product surface, marker
and ship boundary, the seats a stable label may take, and the isolation every
non-stable release owes. Why the contract has this shape, and
every decision that put it there, is `perish.code/plumb-release-contract`.

## Commitments

These bind how Plumb is written rather than what a repository carries, so no
tree answers them and the catalogue does not hold them.

- A stateful tool resolves one data home as ordinary four-layer configuration.
- Machine state may be rewritten; human policy enters through the cascade.
- Machine-written records live below the tool's data-home state directory, name
  their shape version, and refuse a version they do not know.
- A state write lands beside the target and replaces it whole.
- Acting on a managed path requires both a registry record and an in-path
  marker naming the tool.
- The substrate exports config mechanism; each binary declares its own sections
  and values, and owns the variables it supplies to a template whose grammar the
  substrate owns. A manifest spells every injected environment key exactly as
  the target product derives it.
- A released skill never claims enforcement its matching binary does not
  provide. Install, status and upgrade keep the global skill ledger anchored to
  stable, and a non-stable skill takes an exact version and an explicit path,
  receives a staged marker, and never enters managed state.

