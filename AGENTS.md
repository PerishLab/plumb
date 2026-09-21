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

Guard is a staged-tree proof, not a repository workflow. Plumb owns the
pre-commit and commit-message hooks through its configuration depot;
`configuration install` projects them and Doctor requires their presence.
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

- `crates/lib/src/forge/forgejo` — the archived Forgejo adaptors over Runseal's
  structured Forgejo operations, still read by authority and retire. Nothing
  lands through it; it owns no HTTP sender and reads no `tea.yml`.
- `crates/lib/src/forge/{land,github.rs}` — land projects a guarded candidate
  onto GitHub through the `gh` CLI the caller's Runseal profile authorizes.
  Plumb holds no GitHub credential. The required `guard` status is Plumb's
  verified Guard proof, and the merge must match that exact candidate.
- `crates/cli/src/command/retire` — Cloudflare interpretation and orchestration
  over Runseal's structured Cloudflare operations. Worker versions and their
  deployments are wharf's to publish. Plumb owns no authenticated Cloudflare
  HTTP sender and no raw route dialect.
- `crates/cli/{rules,profiles,assets,cookbook,help}` and `crates/lib/rules` —
  Plumb's governed resources. Their exact source-to-seat projections are
  declared on `[layout]` seats; the configuration generation on Depot overrides
  the entries they name. The repository copies of rules and product profiles
  are their source and must equal the standing Depot generation; they are
  being absorbed into the binary, after which Depot no longer carries them.
  Each is one file per addressable thing, and the address is the path: a rule
  set is its name, a cookbook entry is the finding that sends you there, and
  long help is the command path, so `plumb release` reads `help/release.txt`.
  Prose an agent reads as contract is carried, never inlined in a literal.
  Rules exist only in a verified synced seat and never fall back to compiled
  bytes. Assets read seat-first; cookbook and help retain compiled copies, and
  help falls back without a word when the seat cannot be read: help is not a
  verdict, and a tool whose `--help` fails when a store is unreachable fails
  exactly where it is needed most.
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
  `ship` hands that marker to wharf, which publishes every medium. `depot`
  only reports the configuration seat this binary reads; wharf moves Depot
  generations on the marker's own path. Neither is a phase inside the other.
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
- A stable marker comes home by a real merge into main before the next stable
  marker. Doctor notes a stable marker main does not yet hold; `release stamp`
  refuses the next stable marker until the remote main holds the last.
- A marker names identity, not capability. Reading a historical marker does not
  authorize producing its targets today; wharf refuses what the product's own
  manifests no longer support.
- Depot generations remain immutable and addressable. Configuration, changelog,
  skill, channel, manager, and provider bindings may move their latest pointer,
  but every movement names the release marker and uses conditional readback.
  A marker locks its Product Profile, which alone declares the product's depot
  derivatives and their authority; product repositories repeat neither.
  A marker-exact configuration generation validates against the marker's
  published binary. It moves no channel pointer; depot consensus advances only
  after the immutable publication it consumes reads back.
  Configuration, changelog, and skill source trees are caller-owned temporary
  media passed explicitly, never repository or `PLUMB_HOME` seats. Products
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

## Retirement

Retirement is the mirror of release, not a foreign errand, and it lives here
because everything it destroys is something Plumb declared, published, or
protected.

Plumb converges no external authority. Buckets, domains, writers and secrets
that distribution needs are wharf's to provision and hold. Runseal owns each
authenticated provider atom; retire composes those atoms but owns no raw
provider route, credential store, or private instance state.
