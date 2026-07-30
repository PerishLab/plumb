# Plumb v0.18.4

## A locked JSR specifier is matched by range, not by text

The dependency reader built its lookup key from the requirement exactly as
written in `deno.json` and compared it to the lockfile's specifier verbatim.
Deno does not write it back verbatim: inside a zero major it canonicalises a
caret to a tilde, so `^0.3.1` is recorded as `~0.3.1`. The key never matched,
the dependency reported zero resolutions, and — since v0.18.3 made a blind
reading nonzero — the whole doctor exited nonzero on evidence that was
present and correct all along.

Every repository declaring a first-party JSR dependency with a caret was
unreadable this way, and the blind hid whatever else the dependency rules had
to say about it, including the staleness v0.18.3 had just begun to enforce.

The verbatim match still runs first, because a lockfile carrying two ranges
for one package needs the exact key to tell them apart. Only when it finds
nothing does the lookup read every specifier for that package and keep the
resolutions the declared requirement admits.

## Inherited Locus observation, muted by default

Plumb owns the meaning of its CLI observations; Locus owns their context,
collection, generation, and reporting. Runtime policy crosses that boundary
through one typed `PLUMB_LOCUS_*` section, whose master gate defaults to
`false` and returns before Locus bootstraps — so a configured collector or
reporter can sit inherited without creating an ambient observation path.
`references/audit.md` in the skill names the contract.

## What this does not change

No rule, standing, or declaration surface moves. A repository that was true
to the skeleton before remains so, and one that was blind on this now reads
its real findings instead — which for a stale first-party dependency means a
finding it could not previously see.
