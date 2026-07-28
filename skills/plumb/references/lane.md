# Release lane anatomy

A release lane lives in each repository's CI, in YAML, on a system the binary
never sees. Nothing here can be compiled; all of it can be checked by reading,
and every clause below was paid for by an incident.

## The spine

**1. Resolve metadata.** Compute the target version from the prior published
state — the registry itself, not a local file. Refuse a version that regresses,
and refuse a rerun of a version already published unless the lane is explicitly
repairing it.

**2. Guard fresh.** Run the full check, not an incremental one. A release must
not inherit a cache that hides a stale artifact.

**3. Stamp the manifest the publisher reads.** In a workspace, the root
manifest is often not the published one. A stamp written to the wrong file is
invisible to the publisher and to any grep that reads the same wrong file back.

*Incident:* a workspace migration left the stamp on the root manifest. The lane
published the bare base version to an immutable registry and only the
post-publish verify noticed — after the escape.

**4. Assert the dry run.** Run the publisher's dry run and require its output
to name the intended version before the lane may proceed. This is the last
point at which a mistake is still free.

**5. Publish idempotently.** A version already present is a skip, not a
failure, so a repair rerun is safe.

**6. Verify by readback.** Confirm from the registry, not from local state.

**7. Seal.** Tag, and record what was released.

**8. Report failure outward.** Emit the lane log where an operator can reach it
without CI log access — an issue, an artifact, a notification. On systems whose
API does not expose job logs, this is the only forensic trail that survives.

**9. Offer a rehearsal.** A switch that runs the whole lane without publishing,
tagging, or touching credentials.

## The rehearsal clause

A lane that has not run is not known to work. Packaging scripts reference crate
names, paths, and flags that ordinary development changes freely, and nothing
outside the lane exercises them.

*Incident:* a crate was renamed in one commit. The packaging scripts kept
building the old package name and the release lane was broken for five versions
— until someone triggered a release and the lane failed on every non-Linux
runner at once. The local packaging script would have shown it in seconds.

Before triggering a lane that has been idle across renames or restructuring,
run its packaging script locally.

## Channels

Two channels, one direction: beta proves, stable promotes. A beta cut computes
its own suffix from the registry and stamps it only inside the runner; a base
version already stable refuses further betas. Promotion runs the same spine
against the same commit.

## Skill artifacts

A repository that ships a skill packages `skills/<tool>/` verbatim, adds a
metadata file naming the schema, skill, and release version, and publishes it as
`artifacts.skillTarGz` with a name, url, and sha256 alongside the binaries. The
artifact map is assembled by hand in most lanes — adding an entry usually means
touching both the assembly and whatever asserts its length.
