# Skill

A **skill** is a tool's operating brief for an agent, shipped as a release artifact and installed
into the agent directories a machine already has. It is how a tool tells an agent what the binary
cannot enforce.

## What a skill carries

A binary hardcodes what it can evaluate in its own process. Most of what governs a repository lives
outside that reach: the anatomy of a release lane in another repository's CI, the shape a manifest
should take, the judgement of when a constraint has been earned. Those are not unwritten because
nobody wrote them — they are structurally beyond the compiler.

Before skills, that knowledge travelled three ways: scattered `AGENTS.md` files that do not know
about each other, law pages nobody opens unprompted, and conversation, which evaporates. A skill is
the first channel that puts it in the hands of whoever is actually editing the repository.

So a skill is not a prose rendering of `--help`. It is layered:

1. **Principles** — the constitution the tool serves: mechanism and law at the substrate, vocabulary
   with the product, shape enforced by the doctor, and the right to amend travelling with the layer.
2. **Laws** — the clauses themselves: the cascade and its doors, the fill grammar, the anatomy of a
   release lane, the shapes a repository must hold.
3. **Standing** — for each clause, whether it is machine-enforced today or prose on a wall. An agent
   must know which violations will be caught and which only it will remember.
4. **Invocation** — how to run the commands. The thinnest layer, and the one a naive skill would
   have been made entirely of.

Every tool's skill keeps its own law this way: the syntax checker's skill carries syntax law and its
exemptions, the process manager's carries manifest shape and its business-unaware rule.

## Admission and migration

A skill attracts clutter. A clause enters only when all three hold: it governs how a repository is
built or operated, the binary cannot enforce it today, and a real cost has been paid for its
absence.

When a clause becomes mechanized, its prose **demotes to one line** naming the check — it does not
stay duplicated. This is the split the task brief already draws between what still steers execution
and what has settled: a clause the doctor now enforces has settled, and restating it invites the two
copies to drift.

## Text budget experiment

Doctor observes skill size before it enforces a limit. For production source lines `S`, the first
candidate budget is `clamp(2 × ceil(sqrt(S)), 120, 400)` total Markdown lines beneath one skill
seat. Production source is physical lines under the conventional `src` and `lib` seats for Rust,
TypeScript, TSX, CSS, and SCSS; tests, documentation, dependencies, generated output, and the skill
itself do not contribute.

The square root makes prose grow slower than implementation: four times the source earns only
twice the brief, while the fixed ceiling prevents a large repository from turning its skill into a
manual. The current rule is observed, so Doctor reports source, budget, text, and file count without
changing its verdict. It becomes mechanized only after repositories at the floor, curve, and ceiling
show that the measure is stable.

## Standing must be true

The version invariant is not that every documented command exists — it is that **the standing a
skill claims matches what that version enforces**. Saying the doctor catches something it does not
is worse than silence: it hands an agent a guardrail that is not there. Standing changes and the
check that caused them land together.

Across tools the rule is narrower: a skill declares its own law and its own binary's checks, and
**references** another tool's clauses rather than restating them. A restated clause in a second
repository has nobody to keep it true.

## Package

The source lives in the tool's own repository at `skills/<tool>/`, holding `SKILL.md` and, when it
needs them, `references/`. The release packages that directory verbatim and adds an internal
`metadata.json` marker naming the schema, skill, and release version.

The published artifact enters the exact release seal as `artifacts.skill` with a name, URL, digest,
and size. It resolves through the same exact seal as the binary. One release, one version, both
faces of it.

## Installation

Installation is a managed act under the home law (`docs/home.md`), and its whole posture is that of
a guest:

- **Managed means stable.** Install, status, and upgrade select stable releases only. A non-stable
  brief cannot enter the global ledger or any detected agent seat.
- **Targets are found, not assumed.** A candidate agent directory is a target only when that agent
  is present on the machine. An explicit path is honored when it ends with the skill's own name.
- **Nothing unmanaged is touched.** Overwrite requires both the registry entry and the marker inside
  the directory. Force applies within what is managed; it is not a license over the rest of the
  disk.
- **Removal is exact.** Uninstall acts on recorded paths and no others.
- **Replacement is whole.** Unpack into staging beside the target, verify the digest first, then
  swap. A failed install leaves the previous skill standing.
- **Partial success is recorded.** When one target succeeds and another is refused, the success is
  written down and the refusal is reported.

## Candidate staging

`skill stage` is the deliberately separate non-stable path. It requires a channel, exact version,
and explicit path ending in the skill name. That target must not exist. The unpacked brief receives
a staged marker distinct from managed ownership and no state ledger is read or written. The caller
gives an isolated agent that path and removes the surrounding temporary root afterwards.

Stable refuses to stage because it belongs in managed seats. A candidate refuses managed install,
status, and upgrade even when its version is exact. Staged and managed ownership remain disjoint.

## Observation and upgrade

`skill status` reads the managed ledger, ownership markers, and the stable pointer plus its exact
seal. It does not download the skill artifact or write state. `skill upgrade --dry-run` renders the
same per-seat decisions without applying them.

The selected release is compared by semantic version and artifact digest. Current seats are no-ops;
an available newer release upgrades; a missing managed path restores. Same-version digest drift and
an implicit channel rollback refuse. An explicit older `--version` is a deliberate rollback. Real
upgrade downloads the artifact only when at least one owned seat needs to move.

Stable is the only managed install intent. A non-stable channel is useful for discovering immutable
validation cuts, but consuming one means staging its exact version outside managed state. That
keeps a beta or release candidate from becoming an agent-wide consensus by accident.

The mechanized checks — package shape, frontmatter, standing accuracy, and the release spec and seal
carrying its skill artifact — are declared here and not yet running; until they land, this page is
the wall.
