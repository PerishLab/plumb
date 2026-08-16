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
carries it and that this repository calls. `publishes` therefore reports the
declaration rather than a guess assembled from directory names.

## A gate asks the manifest, never the environment

`ship <attachment> publish` asked every release for a compiled capsule by way of
`PLUMB_RELEASE_OUTPUT`. A release declaring only attachments can never set it:
nothing compiles a capsule for it and no authority holds one. The gate refused a
shape Plumb itself defines, one step before the registry and after everything
cheap had already succeeded.

Whether a release carries a seal is a manifest fact, so the gate now asks
`Spec::binary()`. The image and chart attachments likewise declare the forge
`account` they authenticate as; an account is a public name, so a lane supplying
it held a decision the product owns. `PLUMB_RELEASE_REGISTRY_ACCOUNT` is gone.

## Refusals stop pointing at the wrong thing

A projection read its credential in argument position, ahead of the early return
that answers for an attachment nobody declared, so `ship oci publish` refused a
product with no image because no account was set.

`plumb land` refused in the same shape elsewhere. The forge lists every pull, not
only the open ones, and a landing branch is retained after its pull merges, so
matching on refs alone returned a pull that closed long ago and the landing
waited out a guard that would never run again. Every second landing of one topic
branch hung on it.

## The module adaptor speaks pnpm

Only pnpm resolves `catalog:` and `workspace:` while packing; npm copied them
into the published manifest verbatim, where no consumer outside the workspace
can resolve them. It also publishes the archive `pack` produced, from a seat of
its own, so the bytes that ship are the bytes that were checked and the build
runs once instead of twice.

## A projection that already happened is not repeated

Chart and image could silently replace a published version, and the module
refused a re-run outright. All three now ask what the registry already carries:
the chart byte for byte, the image by a payload Plumb declares on it, and the
module by the identity a registry admits once. The image compares a payload
because pushing rewrites an image config, so no digest survives the round trip.

## The lane carries every declared projection

`ship oci`, `ship chart` and `ship npm` had no call site anywhere. The shared
binary lane now prepares them before the seal and publishes them behind it.

`packages/plumb` fixes the shape a published module takes here, and
`charts/plumb` carries a minimal workload. Neither image nor chart is declared
yet: the `account` field lands in this release and a manifest declaring it can
only be read from the next one.
