# Plumb v0.21.0

## Doctor reads the release declaration

Doctor never parsed `[release]`. It peeked at raw TOML for two keys and
swallowed a parse failure, so a declaration that had stopped parsing reported
`true to the skeleton` through three guard runs, two pull requests and a
landing, and failed only on release day.

Two rules now read it through the strict spec every release verb uses.
`release.spec-declared` leaves prose and becomes mechanized: the declaration
parses whole, or Doctor names the refusal. `release.attachment-deliverable`
answers the second half — a declared attachment resolves to a shared lane that
carries it and that this repository calls.

`publishes` therefore reports the declaration rather than a guess assembled from
directory names. A repository that carries `packages/*` but declares no module
attachment no longer reports one.

## A gate asks the manifest, never the environment

`ship <attachment> publish` asked every release for a compiled capsule by way of
`PLUMB_RELEASE_OUTPUT`. A release declaring only attachments can never set it:
nothing compiles a capsule for it and no authority holds one. The gate refused a
shape Plumb itself defines, one step before the registry and after everything
cheap had already succeeded.

Whether a release carries a seal is a manifest fact. The gate now asks
`Spec::binary()`, lifted out of the validation that already computed it so one
definition serves both. A release naming a product and an authority is bound
exactly as before; a release declaring only attachments answers to the
registries that receive it.

The image and chart attachments declare the forge `account` they authenticate
as. An account is a public name, so a lane supplying it held a decision the
product owns; the credential stays in the environment. `PLUMB_RELEASE_REGISTRY_ACCOUNT`
is gone.

## Refusals stop pointing at the wrong thing

A projection read its credential in argument position, ahead of the early return
that answers for an attachment nobody declared, so `ship oci publish` refused a
product with no image because no account was set. Each adaptor now reads its
credential once it knows it has something to project.

`plumb land` refused in the same shape from another surface. The forge lists
every pull rather than only the open ones, and a landing branch is retained
after its pull merges, so matching on refs alone returned a pull that closed
long ago and the landing waited out a guard that would never run again. Every
second landing of one topic branch hung on it. The dry run had been printing
`GET /pulls?state=open` for a filter the code never applied.

## The module adaptor speaks pnpm

Only pnpm resolves `catalog:` and `workspace:` while packing. npm copied them
into the published manifest verbatim, where no consumer outside the workspace
can resolve them.

It also publishes the archive `pack` produced, from a seat of its own. The bytes
that ship are the bytes that were checked, the build runs once instead of twice,
and the deed stops asking a working tree it deliberately stamped to look clean.

## A projection that already happened is not repeated

Chart and image could silently replace a published version, and the module
refused a re-run outright. All three now ask what the registry already carries:
the chart byte for byte through a readback, the image by a payload Plumb
declares on it, and the module by the identity a registry admits once.

The image compares a payload rather than a digest because pushing rewrites an
image config, so a locally built image and the same image read back share no
digest. The payload it was built from survives the round trip.

A release that reached some media and not others now resumes by re-running.

## The lane carries every declared projection

`ship oci`, `ship chart` and `ship npm` had no call site anywhere. Three adaptors
answered, Doctor accepted their attachment tables, and no lane delivered them.
The shared binary lane now prepares them before the seal and publishes them
behind it, beside the Cargo attachment it already carried.

## The carriers take content

`packages/plumb` fixes the shape a published module takes here: `build`, `test`,
`typecheck` and `prepack`, with every tool pinned through the workspace catalog.
`charts/plumb` carries a minimal workload a cluster would admit.

Neither image nor chart is declared yet. The `account` field lands in this
release and a manifest declaring it can only be read from the next one, exactly
as the adaptor enumeration landed before its projections did.
