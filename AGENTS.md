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
- `crates/cli/src/dispatch/site` and `retire` — Cloudflare interpretation and
  orchestration over Runseal's structured Cloudflare operations. Plumb owns no
  authenticated Cloudflare HTTP sender and no raw route dialect.
- `crates/cli/{rules,assets,cookbook,help}` — the resources this binary carries.
  Rules and assets are depot roots; cookbook and help are not yet. Each is one
  file per addressable thing, and the address is the path: a rule set is its
  name, a cookbook entry is the finding that sends you there, and long help is
  the command path, so `plumb ship site deploy` reads `help/ship/site/deploy.txt`.
  Prose an agent reads as contract is carried, never inlined in a literal.
  All four publish to the depot and read seat-first, and help alone falls back to
  its compiled bytes without a word when the seat cannot be read: help is not a
  verdict, and a tool whose `--help` fails when a store is unreachable fails
  exactly where it is needed most. A law read from a stale seat would be a lie,
  so rules refuse instead.
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
- NOTHING INVENTS A LOCATION. `plumb::seat` anchors at run time and
  `plumb::seat::resource!` at compile time, so a caller states what it wants
  rather than where it sits.
- plumb MUST PASS ITSELF. Running the CLI here has to come back clean, or the
  relation above does not hold for the one repo that declares it.

## Documents

- Everything a repository knows belongs where it can be judged or where it can be
  found: in the code, in a rule set, in a note beside the key it is about, or in
  the Concord task that owns the work. A document nobody must read and no
  mechanism keeps true is the worst of the four.

## Release

- `release` holds the truth cycle and `ship` holds every projection of it. No
  verb and no rule states that division for you, which is why it is here.
- This repository renders and carries its own release lanes and reaches no
  shared caller, because the tool that renders a lane cannot depend on a copy of
  itself to render its own.

The verbs are `plumb release --help` and `plumb ship --help`. The laws are
`plumb rule list --namespace release`, twenty four of them, and they state the
product surface, the rendered lane, the seats a stable label may take, and the
isolation every non-stable release owes. Why the contract has this shape, and
every decision that put it there, is `perish.code/plumb-release-contract`.

## Cold-start

Retirement is the mirror of release, not a foreign errand, and it lives here
because everything it destroys is something Plumb declared, published, or
protected.

Plumb does not cold-start. It creates no repository, no bucket, and no domain
that does not already exist, and it never infers or repairs missing external
state. A missing authority blocks at its owning control plane. This is an
implementation constraint, not a permission one: no provisioning call exists in
this codebase, and adding one is the change that must be refused, because the
authority Plumb already holds is sufficient to provision if such a call were
ever written. An absence is the one thing the source cannot show you, which is
why it is stated here and nowhere else.
