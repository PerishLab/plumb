# Agents

Read the canonical [PerishLab delivery governance](https://github.com/PerishLab/.github/blob/main/GOVERNANCE.md)
at work start and again before delivery or Issue closure. That document owns
organization-wide Issue, pull-request and acceptance policy; this file keeps
repository-specific constraints without copying that policy.

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

plumb also absorbs the practices products share. A practice each product writes
for itself is one plumb has not absorbed yet: plumb owns its single default
form, and a product declares only its identity and its values. A practice whose
shape is clear but whose owner is uncertain goes up to plumb, as an Issue here
or a question to its owner, never into a private copy in the product. A practice
whose shape is still unsettled stays in the product until it settles.

Guard is a staged-tree proof, not a repository workflow. Plumb carries the
pre-commit and commit-message hooks in its binary; `configuration install`
projects them and Doctor requires their presence.
Guard stages proof state below `PLUMB_HOME` and carries the compact proof in Git history. Land and
release marker creation refuse a tree without that exact proof; an unchanged
action world is reused and never starts a process.

A Guard authority is the producer a proof names (`vX.Y.Z@commit`) and its
Depot mark; `judge` matches both exactly. `Authority::released()` is the one
compiled into plumb-lib. `plumb release authority --json` prints the running
binary's, and wharf writes it into each stable seal as
`"guard": {"producer": "vX.Y.Z@<commit>", "depot": "<sha256>"}`, no other keys,
the producer naming that seal's release. `Authority::stable()` (feature
`skill`) reads it from `releases.plumb.perish.uk`: the stable pointer, then the
seal pinned by its sha256, then that field. It refuses a digest mismatch and a
missing or foreign field, and its unreachable error names the pointer URL. The
location is fixed; only unit tests override it. A caller accepting several
authorities picks the proof's own with `Authority::among`, then verifies.

Downstream repositories do not get scanned by plumb, and plumb does not know
they exist. The CLI travels to them: install it, run it in a repository, read
what it reports. Feedback comes home as issues on this repo. That is the hot
link, and it is the only one.

## Layout

These are the nodes `[layout]` has not converged on. Each leaves this list when a
seat can state it; nothing belongs here that a seat, a rule, or a verb already
answers for.

- `crates/lib/src/forge/delivery/kernel.rs` — the narrow `delivery` feature
  prepares and revalidates an exact candidate from caller-supplied Issue and
  pull declarations under released Guard authority, and holds the one squash
  merge contract and its readback that `land` and Concord both run. Only after
  that readback does it delete the branches a delivery pushed, by lease on the
  heads it pushed, and it keeps and names any that moved. It has no
  provider client: callers run its argument vector through their own command.
  Its distinct native-gate API accepts an explicit caller-owned verifier only
  for a source and base without Plumb governance. It binds native evidence to
  the exact source tree and shares topology, candidate construction and squash
  readback with Guarded delivery. Revalidation has two explicit forms:
  `revalidate` re-executes the verifier, and `confirm` repeats every other
  check while asking the gate only to recompute its identity evidence. Native
  evidence names inputs rather than attesting a result, so `confirm` never
  proves a pass on its own; the caller decides where an execution must
  precede a mutation, and only a gate that also implements `Identity` can be
  confirmed. The caller owns native authority admission and gate policy; no
  Plumb CLI switches to native delivery on a missing or invalid Guard proof.
  It is consumed by Concord. The registered `follow` operation observes only
  its automation-owned Auto Issue and pull; ordinary Issue delivery belongs
  to Concord. Repository-only `land` remains a separate explicit surface.
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
  the Issue that owns the work. A document nobody must read and no
  mechanism keeps true is the worst of the four.

## Brand

Crest owns brand geometry, palette and supported image variants. The Web specimen
uses the formal Design adapter and registered plumb product, including its head
media. The specimen uses Shell to declare its actual theme surface. Its root
Home is prerendered and hydrated as the same component; Views mounts unrendered
entries instead of hydrating a different component over that root. Retired unused
brand copies are removed; ordinary business graphics are
not automatically classified as branding.

An explicit [brand] table activates the current source/Guard consumption
contract. The reader discovers root/apps/* Svelte/Vite applications instead of
assuming every app is named web. Qualified configuration is a direct object or
imported defineConfig, literal plugin options, Design plus the imported Svelte
plugin, optional local absolute base and build.outDir. Other resolution,
transforms, config imports, plugin implementations and dynamic selections remain
unqualified. Unrelated each/snippet/await business templates do not invalidate
brand references outside their scope. Brand references inside those lexical
scopes, const or slot bindings in a brand-bearing scope, and snippet names that
shadow an imported brand component remain unqualified. This bounded AST projection
does not resolve arbitrary local component aliases. Module/context scripts remain
unqualified. Literal TypeScript generics are parsed as type parameters and create
no runtime brand binding; dynamic, malformed or JavaScript generics refuse.
The default HTML carrier requires registered data-crest images in actual generated
HTML. Explicit mode="views" qualifies only the standard Design Views bootstrap,
root Vite base, login:false, one root view, and one unconditional brand under
plain imported Shell/Frame layouts. Other bootstraps, aliases and route graphs
stay unqualified. That carrier binds source, origins and unchanged official
images; its output module digest is descriptive, not proof of compiled-source
equivalence or runtime rendering. Browser display still needs independent evidence.
The v2 report separates the carrier and rendering obligation and always reports
browser verification as unverified.

The Svelte compiler call must be genuine, unique and argument-free. Automatic
Svelte configs, package browser mappings and producer self-reference app names
stay unqualified. Required package, lock, app config and HTML inputs must be tracked
regular files. Headless Views consumption requires an explicit null head receipt
and no graphical head, allowing an empty data favicon only. Declared head media
still requires its formal product/origin and image bytes; unsupported alternate
image/manifest metadata refuses. Current copies require a pnpm input world; unsupported
hosts and package worlds are not silently activated or given a UI framework.

The optional origin library feature exposes the shared packages Origin capability queries the fixed official npm authority
and fetches an exact stable archive without executing repository/package code.
SHA512, package identity and bounded regular archive entries are verified. Its
serialized reference/read operation validates content only; a supplied reference
never authenticates a publisher. Guard captures its own references and complete
formal publication identities before product execution and binds them into the
web action input. Build execution remains credential-free.

After the build, the checker compares installed Crest/Design files with their
formal archives, reads the official asset catalog, and checks every receipt
selection/output byte, actual HTML head references and declared rendered image
identities. Tracked app inputs also bind the web action, including root app
source outside the usual apps directory. Tracked unchanged copies
name only an official asset ID and consumer output path. They are checked before
Guard cache reuse as well as after execution. No geometry, palette, product list
or image digest becomes a second compiled brand truth. This bounded evidence
proves neither arbitrary image semantics nor browser appearance, runtime request
coverage or domain-wide activation. Formal tool distribution, installation and
real land canaries remain separate rollout evidence.

## Lane evidence

Schema2 selection/operation evidence retains its canonical bytes. Schema3
confirmation links an immutable original unknown, a distinct new public
Authorization Reference and an exact applied mutation observation. The new
reference also differs from the original admission reference. Explicit
linked readback preserves the entire request and original assessment. Ordinary
operation access and settlement do not consume confirmation. The generic types
bind references and supplied observations; they authenticate no authorization
receipt, observer, physical writer idle or live publication/settlement. The
consumer must independently fetch and validate the new public receipt under its
prepared authority, prove current authority/quiescence and order evidence before
state writes. Authority-specific execution details remain in that consumer's
public receipt. Source support alone does not qualify an installed consumer.

## Distribution

Release semantics foundation reads current protected Wharf requirements and exact
native review declarations. Its supplied record and frozen world codecs validate
data and internal consistency; they authenticate no publisher, execution, baseline
adoption or canary. A captured world must come from its original publication;
rebuilding old source with today's controls cannot recover that historical world.
The foundation does not enable a semantics gate in open, stamp or ship. Those
admission and activation commitments remain open in Plumb #617 and Wharf #354.

The shared record reader authenticates an immutable semantic object's exact bytes
through its completed native issuer job and digest-bearing confirmation step,
with protected-main history and fresh authority checks. Capture roles do not
issue records. This producer authentication alone establishes no baseline
adoption, boundary equality, execution, canary or publication permission; the
matching trusted Wharf issuer must still be deployed and consumed.
The initial baseline reader additionally verifies the actual annotated stable,
complete original distribution and recorded combinations, original world,
full source and immutable Scene collection, and exact native adoption review.
Missing historical plans or resolved worlds remain unknown. This retrospective
adoption does not claim original historical execution or release compliance.
An effective tree differing from the archived source remains unknown until its
complete effective-source materials are available; a hash alone is insufficient.
The same material verification is available before issuance without authenticating
the supplied record. Issuers consume that shared check; baseline readers add
exact native object authentication. Neither phase grants publication permission.

World-bound review v2 names the exact before, candidate and original family anchor
as separate snapshot/world digest pairs. Its shared projection validates each
source/world binding and augments a copy of every snapshot with the same reserved
runtime world material before the existing assessment algorithm runs. It never
rewrites an adopted snapshot or rolls its anchor. World changes require the exact
review even when source/Scene are identical; a byte difference alone does not
force Y. The native reader authenticates the current declaration and author, not
the supplied worlds or their semantic truth. This projection is advisory until
trusted history, candidate-world provenance and complete admission are verified;
v1 snapshot-only reviews cannot substitute for world-bound review.

The native marker inventory reads every bounded GitHub tag page twice and
rechecks repository identity. It retains each canonical release name and the
native listed commit; repeated pages, unreadable or oversized inventories and
moved observations cannot become an empty reservation set. It does not recover
deleted tags, authenticate annotated tag objects, prove historical ordering or
establish a complete retained version history. Full admission must combine these
current observations with its immutable history and original family evidence;
this inventory alone grants neither equality nor publication permission.

Retained-history checkpoints bind the original adopted stable, exact repository,
predecessor and cumulative marker/source reservations. Shared preparation reads
the current discovery and complete current inventory, preserves disappeared tags,
and refuses retargeting or a replaced root. The native reader authenticates each
checkpoint through its dedicated history role and exact digest step, verifies
the complete chain and full adoption, and rechecks mutable discovery and authority.
Its 128-checkpoint/16-MiB observation bounds fail unknown. Pre-inception history
remains unknown. Preparation data authenticates no supplied native fields and
does not prove absence, conditional writes or recovery. Current discovery does
not establish complete historical coverage, new-family transitions or publication.
The matching writer, formal consumption and complete admission remain open.

The effective-source payload reader verifies bounded decompression, every raw
blob's size, SHA256 and Git identity, regular-file modes, complete directory
structure and the reconstructed effective Git tree against the supplied world.
Its current portable-path support is ASCII; unsupported Unicode remains unknown.
This data reader neither authenticates the supplied world or producer nor grants
publication. Existing baseline and comparison readers still refuse unavailable
effective-source authority; successful payload decoding cannot bypass them.

The native prepared-world reader authenticates a supplied descriptor's two exact
public objects through the original completed semantic-world job and its unique
dual-digest confirmation step, reusing the issuer's native identity verifier.
It verifies complete effective source and rechecks immutable objects and current
Wharf authority. Its authentication concerns original preparation, not current
candidate freshness, semantic closure/equality, adoption or publication. CLI
facts summarize source identities/counts; the library retains verified file data.
Retained source objects and real installed consumption remain separate rollout
obligations. This API does not loosen baseline or advisory comparison checks.

Verified frames join an authenticated original baseline or a native prepared-world
descriptor with its exact immutable source/Scene collection. Prepared frames retain
the descriptor's bytes as reserved runtime material, so reviews bind complete
source provenance as well as world inputs. Frame fields cannot be constructed or
deserialized by callers. Shared frame projection reuses the same comparison and
assessment path; it can handle differing effective trees only after complete
source verification. Raw advisory projection still returns unknown there.
Frames do not establish current candidate freshness, trusted history, semantic
truth or publication permission; complete admission must reobserve those facts.

The `windtun` library feature exports
`plumb::windtun::qualification::{Inspector, Read, Remote}`. `Inspector` owns the
bounded protocol judgment; `Read` supplies authenticated observations and never
grants a verdict. The production `Remote` reads fixed GitHub metadata through
`gh`, anonymous HTTPS objects and GHCR manifest bytes. It executes no product,
suite or container and has no storage writer. Custom transports retain their own
source authentication obligations; memory observations are test evidence only.

`windtun check` selects an official repository and non-stable marker. It binds
the product's exact annotated Git tag, source and manifest, current protected Wharf verifier
requirements, latest declared attempt, native roles and complete steps, current
product/runner/scene/image and actual archive bytes. The original reusable binary
origin has an empty commit; the publication commit belongs to its validated
binding. These identities must not be conflated. Mutable discovery and input
worlds are rechecked after the immutable references and bytes are verified.

The current scope is Linux x86_64, the recorded single-product combination and
GHCR runtime images. Other registries and combinations remain unknown or refused.
Distribution reads accept exactly the legacy record or that record with a
`wharf.plan.reference/v1` reference. The reference binds the same source and
original completed publisher; its planner attempt may precede completion. A
later observing run does not retarget it. Its digest is validated as metadata,
without a private plan read or a claim of content association.

Reports keep qualification, unassessed content association and promotion separate;
promotion is always false. This read-only surface does not activate promotion
checks in stamp or Wharf. Help owns its selectors and exit codes. A source merge
does not establish the release, installed consumer or delivery canary promised by
PerishLab/plumb#509.

- `release` only stamps its immutable marker on the head of a release line.
  `ship` hands that marker to wharf, which publishes every medium. `depot
  consign` places a marker's changelog, skill or scenes in wharf's yard, dispatches
  wharf to lodge it, and reads Depot back; wharf is Depot's only writer.
  Neither is a phase inside the other.
- A release is one marker and one seal over every executable it carries, so a
  CLI and its server release together by construction. Each executable in
  `binaries` may narrow its targets and stay out of the installed manager
  (`[release.binary.<name>]`); a placement (`deb`, `oci` with its chart,
  `cfworker`) names what it carries. Every executable binds the product's
  identity prefix, and a server built from `crates/api` is named
  `<product>-api`.
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
  its changes held by main (contained, not ancestral, since main takes only
  squash merges; `release rejoin` reports it and landing the line's changes
  settles it), its changelog on Depot and its skill where one is declared
  (`depot consign`), and its line closed. `command/operator/owed` is their
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
- Depot consigns changelog, skill and scene generations;
  configuration travels inside the binary. Scenes carry regular file materials,
  not an admission to execute them; Seatrial owns their execution semantics. Generations remain immutable and
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
every decision that put it there, is in the Issues and pull requests that
changed it.

Historical family inspection authenticates complete adoption/transition chains,
reconstructs every formal frame, retained reservation and world-bound review,
and repeats those inputs before returning. Z preserves its initial X.Y anchor;
verified Y establishes a new anchor without replacing the root. Supplied Request
fields must equal the independent derivation. Missing transitions, exhausted
bounds, missing effective-source materials and unverified X stay unknown.
It selects no current stable, admits no prospective candidate and enables no gate.
Transition preparation additionally binds current latest stable and fixed history,
repeats the independent derivation and returns no authentication for supplied
native fields or its derived family. It admits no prospective release. Historical preparation instead verifies the
original bound checkpoint and complete formal publication, so later tool releases
do not prevent retrospective reconstruction. Its explicit historical freshness
never substitutes for current release admission.
Its matching transition producer and formal consumer qualification remain open.

## Commitments

These bind how Plumb is written rather than what a repository carries, so no
tree answers them and the catalogue does not hold them. Each is absorbed, being
absorbed, or Plumb's own.

Absorbed; every product inherits them:

- A repository names its product once, as `[identity] ns` in `plumb.toml`,
  equal to the repository name. Release, doctor and every later identity read
  it from there; `[release]` carries no name of its own.
- Human policy enters through the cascade. Radicula owns the generic config
  runtime and derive; Plumb supplies defining-package prefix and environment
  lookup. Each binary declares its sections, values and explicit layering, and
  owns the variables it supplies to a template whose grammar the substrate owns.
  A manifest spells every injected environment key exactly as the target product
  derives it.
- Radicula owns the generic execution Context and its scoped carriers. Radicula
  Trace consumes Context and owns its local frame completion; retained Context views
  never own that completion. Locus owns its semantic Context and observation
  records. The global observation Run selection remains a separate policy.

Being absorbed; plumb owns the end form, and no product invents its own
meanwhile:

- A stateful tool keeps one data home, resolved as ordinary four-layer
  configuration. The end form derives it from a declared identity, such as
  `#[plumb.cli(ns = "santi", key = "cli")]` yielding the `SANTI_CLI_DATA_HOME`
  layer; its default location, naming, the migration of existing variables and
  its Windows placement are plumb's to decide. Until then a product adds no new
  name or precedence for it.
- A CLI run has one observation: `plumb::observation` (feature
  `observation`) records `cli.start`, then `cli.finish` with the exit code and,
  for a non-zero exit, the `<producer>.fault` role holding the first failure
  code the run noted. The shared observation binds Locus's parent-link collector
  on the start record only, so a set `LOCUS_PARENT` appears as `locus.parent`
  there; `parent` writes the caller's current frame span onto a child
  `Command` and writes nothing when observation is off. A product declares its
  producer, the file holding its `[locus]` section and its buffer, and notes
  faults at its refusal sites. A fault is a `'static` stable identifier (a
  Cookbook code, a catalogue rule id or a refusal kind), never message text,
  arguments or paths. Each `Run` owns its engine, context and fault, so a
  process may observe several runs in turn; free `view()`/`fault()` address the
  run started last and not yet finished, and `Run::view`/`Run::fault` address
  that run only. Each product moves its own copy onto it as its own change.
  `Run::open` accepts the product's Locus Hook and returns configuration or
  construction errors without printing them; disabled or missing configuration
  returns `Ok(None)`. Engine diagnostics follow that Hook. Products retain
  their own terminal-safe diagnostic handoff and business failure policy.
  `Run::start` keeps the default stderr adaptor and its existing Option result.
- A released skill never claims enforcement its matching binary does not
  provide. Install, status and upgrade keep the global skill ledger anchored to
  stable, and a non-stable skill takes an exact version and an explicit path,
  receives a staged marker, and never enters managed state.

Plumb's own; a product may adopt the exports but is not judged against them:

- Machine state may be rewritten. Machine-written records live below the tool's
  data-home state directory, name their shape version, and refuse a version
  they do not know.
- A state write lands beside the target and replaces it whole.
- Acting on a managed path requires both a registry record and an in-path
  marker naming the tool.
- Package subprocesses resolve and verify their tools through captured
  `config::Execution` bindings. Executable lookup and platform invocation stay
  inside that mechanism.

Anonymous public bucket GETs use one fixed ten-second global deadline, covering
connection, response and body consumption. The shared reader preserves 404 as
missing and all other unreadable/status failures as refusal. It adds no retry,
alternate authority or evidence substitution. A successful byte read alone grants
no authenticity, distribution completion or release permission; consumers retain
their existing record, digest, identity and current-authority checks.

Stable family selection has a separate canonical record and fixed discovery.
A separately retained inception binds the first selection; lost discovery never
authorizes a different first branch. Its dedicated native role authenticates selection origin, while current selection
also reconstructs the family, authenticated predecessor sequence and retained
history against the caller's expected original root. Predecessors must stay on
the selected branch and advance stable. The latest retained stable must still
exist at its exact live source; deleting a tag cannot authorize older selection.
Current reservations may extend the selection's bound checkpoint. Repeated source,
material, history, inventory, discovery and authority checks refuse movement.
Nomination authenticates its observed family, never its supplied native fields or
conditional update. The producer owns protected predecessor observation, CAS and
unknown-write recovery; source data and public absence prove no writer authority.
These metadata facts do not admit prospective releases or activate a scope.
