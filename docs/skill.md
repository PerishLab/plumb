# Skill

A **skill** is a tool's operating brief for an agent, shipped as a release
artifact and installed into the agent directories a machine already has. It is
how a tool tells an agent what the binary cannot enforce.

## What a skill carries

A binary hardcodes what it can evaluate in its own process. Most of what
governs a repository lives outside that reach: the anatomy of a release lane in
another repository's CI, the shape a manifest should take, the judgement of
when a constraint has been earned. Those are not unwritten because nobody wrote
them — they are structurally beyond the compiler.

Before skills, that knowledge travelled three ways: scattered `AGENTS.md` files
that do not know about each other, law pages nobody opens unprompted, and
conversation, which evaporates. A skill is the first channel that puts it in
the hands of whoever is actually editing the repository.

So a skill is not a prose rendering of `--help`. It is layered:

1. **Principles** — the constitution the tool serves: mechanism and law at the
   substrate, vocabulary with the product, shape enforced by the doctor, and
   the right to amend travelling with the layer.
2. **Laws** — the clauses themselves: the cascade and its doors, the fill
   grammar, the anatomy of a release lane, the shapes a repository must hold.
3. **Standing** — for each clause, whether it is machine-enforced today or
   prose on a wall. An agent must know which violations will be caught and
   which only it will remember.
4. **Invocation** — how to run the commands. The thinnest layer, and the one a
   naive skill would have been made entirely of.

Every tool's skill keeps its own law this way: the syntax checker's skill
carries syntax law and its exemptions, the process manager's carries manifest
shape and its business-unaware rule.

## Admission and migration

A skill attracts clutter. A clause enters only when all three hold: it governs
how a repository is built or operated, the binary cannot enforce it today, and
a real cost has been paid for its absence.

When a clause becomes mechanized, its prose **demotes to one line** naming the
check — it does not stay duplicated. This is the split the task brief already
draws between what still steers execution and what has settled: a clause the
doctor now enforces has settled, and restating it invites the two copies to
drift.

## Standing must be true

The version invariant is not that every documented command exists — it is that
**the standing a skill claims matches what that version enforces**. Saying the
doctor catches something it does not is worse than silence: it hands an agent a
guardrail that is not there. Standing changes and the check that caused them
land together.

Across tools the rule is narrower: a skill declares its own law and its own
binary's checks, and **references** another tool's clauses rather than
restating them. A restated clause in a second repository has nobody to keep it
true.

## Package

The source lives in the tool's own repository at `skills/<tool>/`, holding
`SKILL.md` and, when it needs them, `references/`. The release packages that
directory verbatim and adds `metadata.json` naming the schema, the skill, and
the release version.

The published artifact enters the release metadata as `artifacts.skillTarGz`
with a name, a url, and a sha256, and it resolves through the same channel and
version surface as the binary. One release, one version, both faces of it.

## Installation

Installation is a managed act under the home law (`docs/home.md`), and its
whole posture is that of a guest:

- **Targets are found, not assumed.** A candidate agent directory is a target
  only when that agent is present on the machine. An explicit path is honored
  when it ends with the skill's own name.
- **Nothing unmanaged is touched.** Overwrite requires both the registry entry
  and the marker inside the directory. Force applies within what is managed; it
  is not a license over the rest of the disk.
- **Removal is exact.** Uninstall acts on recorded paths and no others.
- **Replacement is whole.** Unpack into staging beside the target, verify the
  digest first, then swap. A failed install leaves the previous skill standing.
- **Partial success is recorded.** When one target succeeds and another is
  refused, the success is written down and the refusal is reported.

The mechanized checks — package shape, frontmatter, standing accuracy, and the
release metadata carrying its skill artifact — are declared here and not yet
running; until they land, this page is the wall.
