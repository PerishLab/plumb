# Release lane anatomy

Release delivery is common Plumb substrate. A product repository supplies a
strict `[release]` table in its root `plumb.toml`, genuine product asset inputs,
and two thin callers into the shared Actions workflow. It does not carry build,
archive, skill, package, manager, storage, channel, verification, or
release-record implementations.

## Product declaration

The release table declares one product, canonical authority, binary set,
supported Rust targets, and typed attachments such as a skill, Cargo packages,
or a Debian payload root. Platform keys, archive names, environment identity,
version probes, and artifact metadata are derived by Plumb. Unknown fields
refuse. Capsule compilation also refuses a missing or extra artifact, so the
declaration is the one inventory used by build, managers, records, and
verification.

Generated managers and capsules are release-run artifacts. They are not source
files and are never checked in.

Release authority is an input, not a resource Plumb provisions. Repository
creation and resource, domain, token, escrow, and secret synchronization remain
with their owning control planes. Product repositories carry no cold-start
wrapper, and Plumb refuses missing publication authority rather than inferring
or repairing external state.

## Generator resolution

Every permanent release lane installs canonical stable Plumb through the root
manager. It does not pin Plumb: stable is the workshop's reliability anchor and
Plumb owns the correctness of its current compiler.

One release run resolves Plumb once. The exact seal records that binary's
reported version and the manager-template digest. A compiled capsule is sealed:
a retry publishes that capsule rather than silently recompiling it with a
different generator.

Plumb's first release of this substrate is a one-time genesis ceremony using
the source-built binary. Its typed recovery contract is fixed to
`PerishLab/plumb`, the canonical authority, exact beta `v0.18.14-beta.1`, stable
`v0.18.14`, and `release/v0.18.14`. Source-built `v0.18.14` records its compiled
commit while generating the beta; stable records the exact public beta seal URL
and byte digest and embeds the same seal as promotion proof. The CLI accepts
only the later receipt-derived generator, release, Actions, caller, and beta
digest identities, never a product, authority, version, workflow, branch, or
generator selector. Identity drift refuses. Genesis uses the same capsule,
conditional storage, public verification, and separate capabilities as every
subsequent release, and every active recovery surface is retired at settlement.
No permanent bootstrap lane exists.

## Lane spine

Actions asks Plumb for the target matrix. Platform jobs call
`plumb release build`; Cargo discovery, version stamping, target builds,
archives, and inspection stay in Plumb.

One coordinator performs the stateful sequence:

1. Resolve current stable Plumb once.
2. Bind the called workflow's direct event ref and commit once.
3. Run the repository's fresh guard.
4. Stamp and dry-run any declared registry attachment.
5. Gather target archives and build declared skill or package attachments.
6. Inspect the exact declared artifact set.
7. Compile one capsule with the product commit and exact release identity.
8. Publish content-addressed objects and the exact seal.
9. Verify every published object through the public authority.
10. Publish and read back any registry attachment.
11. For stable only, activate the root managers and stable pointer.
12. Smoke the generated manager on every supported platform.

Arbitrary hooks do not run inside capsule publication or stable activation.
Those phases stay small enough for their invariants to remain auditable.

Exact publication may bind any selected branch under `refs/heads/`; that
freedom does not weaken its immutable identity. Stable alone binds
`refs/heads/release/vX.Y.Z`. The release line is prepared by linear
`cherry-pick -x`, frozen before stable publication, and does not block ordinary
movement on `main`. Its exact protection remains frozen after publication.
`plumb release dispatch` owns workflow dispatch; `plumb stable
prepare|pick|freeze|packport` owns the stable branch lifecycle. Product
repositories carry neither operation in a local wrapper.

After activation and smoke, a local operator merges the release line into
`main` without flattening its topology and proves the published stable commit
is an ancestor. Packport is settlement, not an Actions tail job: a failed or
interrupted packport does not rewrite a successful stable result, block exact
publication, or block ordinary `main` work. It does block activation of the
next stable line. The settled release branch remains permanently as the
stable version's source and audit boundary.

## Immutable publication

Product bytes and generated exact managers live under
`v1/objects/sha256/<digest>/<name>`. One exact release seal lives at
`v1/releases/<channel>/<exact-version>/seal.json`.

Object publication is idempotent by digest. Exact seal creation is conditional
on absence; an existing byte-identical seal makes a rerun a no-op, while any
same-identity drift refuses. The seal records product, channel, exact version,
source commit, generator provenance, artifacts, managers, and stable promotion
proof when present.

Public readback fetches the seal and every named object, then proves byte count
and SHA-256. It does not trust the publisher's local workspace as evidence of
what the authority serves.

## Stable consensus

Stable is the only moving release intent. Its pointer lives at
`v1/channels/stable.json`; its generated public entrypoints live at
`/manage.sh` and `/manage.ps1`.

Stable promotion embeds the complete exact candidate seal plus its digest. The
candidate must name the same product, base version, source commit, and a
non-stable exact channel. Stable binaries are rebuilt from that commit with
stable version identity.

Activation conditionally updates the root managers before compare-and-swap of
the stable pointer. Every root-manager version can interpret both the prior and
next current record, so a failed attempt before pointer movement leaves the
prior consensus valid. The pointer is the sole consensus commit.

Exact seals and the stable pointer are also the complete release identity.
Permanent lanes create no new Git tags; historical tags remain historical
records and are not consulted as consensus.

Publishing exact objects and activating stable use different commands,
environment names, and persistent credentials. A non-stable lane never receives
the activation capability.

## Channel isolation

Stable is `vX.Y.Z`. A non-stable exact release is
`vX.Y.Z-<channel>.N`. Non-stable has no pointer, activation, or implied
version.

The canonical root manager may express an exact release for any channel.
Non-stable and custom-authority installs require exact identity plus explicit
install and bin paths disjoint from the stable defaults. Only canonical stable
may use the default seats and omit an exact version.

## Coupled Cargo packages

When a library and its procedural macro share one release identity, stamp and
assert both manifests before publishing either, then fully package and dry-run
the macro. Publish the macro first and wait until the registry reads it back
with the expected checksum. Only then can Cargo resolve the library's exact
registry dependency. A rerun verifies and skips an already matching package;
an absent macro beneath a present library or a same-version checksum mismatch
refuses.

## Skill artifacts

A repository that ships a skill packages `skills/<tool>/` verbatim and adds a
small internal marker naming its schema, skill, and release version. The
release spec declares that archive once under the `skill` artifact key. The
exact seal supplies its URL, digest, and size to skill installation.

## Site lanes

A site lane installs stable Plumb and calls `plumb site deploy`; repositories
carry the `apps/*/wrangler.jsonc` declaration and dispatch lane but no ship
wrapper. The CLI derives the product-owned build and public-readback boundary.
The control plane can report a domain as bound while the edge still routes
elsewhere, so *bound* and *reachable* are separate findings and only the second
proves that the site answers. Compare the fingerprinted asset in the served
page against the built one; a status code alone proves only that something
replied.

If the lane cannot reach the public edge, it declares that blindness rather
than treating an unreachable site as success. Reading the binding is itself a
permission: keep `unknown` distinct from `no`, and when binding is unknown,
readback is the only remaining evidence.
