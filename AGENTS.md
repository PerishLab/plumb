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

- `crates/lib/src/forgejo` — Plumb orchestration and Git adaptors over Runseal's
  structured Forgejo operations. It owns no HTTP sender and reads no `tea.yml`.
- `crates/cli/src/command/ship/site` and `retire` — Cloudflare interpretation
  and orchestration over Runseal's structured Cloudflare operations. Ship owns
  immutable Worker Versions; marker-bound depot projection owns Deployments.
  Plumb owns no authenticated Cloudflare HTTP sender and no raw route dialect.
- `crates/cli/{rules,assets,cookbook,help}` — Plumb's governed resources. Their
  exact source-to-seat projections are declared on `[layout]` seats and form
  the configuration derivative; no second inventory owns them. Each is one
  file per addressable thing, and the address is the path: a rule set is its
  name, a cookbook entry is the finding that sends you there, and long help is
  the command path, so `plumb version` reads `help/version.txt`.
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

- `version` projects repository identity and governs the stable line. `release`
  only defines and verifies its immutable marker. `ship` consumes that marker
  and publishes every immutable medium through one request graph. `depot`
  independently consumes the same marker and moves its mutable projections;
  neither command is a phase inside the other.
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
- This repository carries the one canonical `ship.yml` atom. Its only execution
  classes are reusable workload and marker-bound publication requests. Product
  repositories dispatch its exact Plumb-owned revision and carry neither a
  workflow copy nor a rendered derivative.

The verbs are `plumb version --help`, `plumb release --help`, `plumb ship --help`,
and `plumb depot --help`. The laws are
`plumb rule list --namespace release`; they state the product surface, marker
and ship boundary, the seats a stable label may take, and the isolation every
non-stable release owes. Why the contract has this shape, and
every decision that put it there, is `perish.code/plumb-release-contract`.

## Authority convergence

Retirement is the mirror of release, not a foreign errand, and it lives here
because everything it destroys is something Plumb declared, published, or
protected.

Plumb may cold-start and converge an external authority only through a closed,
named profile whose model, order, verification, and retirement semantics are a
mature convention. The profile derives every conventional name and exposes only
irreducible operator choices. It reinspects before each mutation and reports the
same ordered plan whether observing or applying.

Runseal owns each authenticated provider atom. Plumb composes those atoms but
owns no raw provider route, credential store, or private instance state. Generic
provisioning remains refused: a new external resource kind first needs a closed
profile with an explicit lifecycle, not another downstream schema or an open
bag of provider arguments.

The closed workflow profile owns the shared inventory bucket, public domain,
bucket-scoped writer escrow, and organization Actions-secret binding. The
`workflow record` transaction owns immutable workload upload and conditional
inventory merge. Reusable workflows transport that transaction; ordinary
repositories own neither R2 configuration nor inventory JSON.
