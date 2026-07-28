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

**7. Seal.** Record every immutable release identity. A stable release also
gets a Git tag; a prerelease does not. Tags are promotion anchors, not an
inventory of disposable validation cuts.

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

One stable sink, zero or more prerelease channels, and one direction. Stable is
`X.Y.Z`; a prerelease is `X.Y.Z-<channel>.N`. Products own which channel names
exist and how they promote. The lane owns the grammar, the monotonic sequence,
runner-only stamping, publishing, readback, and refusal once the base version
is stable.

Stable may be selected through its moving channel metadata. A non-stable
consumer must name the exact immutable version; its channel latest pointer is
discovery, not an install intent. Strongly coupled packages published across a
compiler boundary use exact internal requirements as well.

Prerelease sealing stops at verified immutable artifacts and metadata. Stable
promotion runs the same spine against the same commit and adds the one durable
Git tag.

## Coupled Cargo packages

When a library and its procedural macro share one release identity, stamp and
assert both manifests before publishing either, then fully package and dry-run
the macro. Publish the macro first and wait until the registry reads it back
with the expected checksum. Only then can Cargo resolve the library's exact
registry dependency, so perform the library's full package and dry run there
before publishing it. A repair rerun verifies and skips an already matching
package; an absent macro beneath a present library or a same-version checksum
mismatch refuses.

## Skill artifacts

A repository that ships a skill packages `skills/<tool>/` verbatim, adds a
metadata file naming the schema, skill, and release version, and publishes it as
`artifacts.skillTarGz` with a name, url, and sha256 alongside the binaries. The
artifact map is assembled by hand in most lanes — adding an entry usually means
touching both the assembly and whatever asserts its length.

## Site lanes

A site lane is the same spine with a different registry: build, deploy, then
read back from the edge. Its verification differs in one way worth stating —
the control plane can report a domain as bound while the edge still routes
elsewhere, so *bound* and *reachable* are separate findings and only the second
is evidence that the site answers. Compare the fingerprinted asset in the
served page against the built one; a status code alone proves only that
something replied.

A stuck binding usually clears on a second, identical ship. If the lane runs
somewhere that cannot reach the public edge, it must declare that blindness
rather than treat an unreachable site as success.

Reading the binding is itself a permission. A credential without it gets a
refusal, and a refusal parsed for a result set yields an empty one, which reads
as *not bound* — so a healthy site fails on a finding the lane never actually
made. Keep `unknown` distinct from `no`, and when binding is unknown the
readback becomes the only evidence there is: blindness cannot excuse it, because
a ship that could neither ask nor look has proved nothing at all.
