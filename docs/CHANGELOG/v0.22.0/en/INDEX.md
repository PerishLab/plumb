# Plumb v0.22.0

## Plumb declares every medium it projects onto

Plumb declared a binary, a skill and the Cargo family, and nothing else. The
image, chart and module attachments were withdrawn one release ago, each for its
own reason, and the reasons have now all expired.

The module attachment left because canonical stable handed npm its credential
unstripped, so npm sent `Authorization: Bearer Bearer <token>` and the registry
refused it. v0.21.0 moved credential reading into each adaptor; the lane that
publishes now runs that release.

The image and chart attachments never left for a reason inside Plumb at all.
Their deeds run `docker build` and `helm package`, and the job image carried
neither client. A product could declare `[release.oci]`, pass its own Doctor,
and then have nowhere to be built. The Forge image carries both clients now, and
the shared lane names the endpoint they speak to.

`plumb.toml` therefore declares an image, a chart, a module and the Cargo family
beside the binary and the skill. Six media, one declaration, one gate.

## A capsule covers what a capsule can cover

`AGENTS.md` still claimed that every publishing deed refuses a capsule sealing
another version. That stopped being true in v0.21.0, when the gate began asking
`Spec::binary()` instead of the environment: a release that declares no binary
shape compiles no capsule and holds no authority to keep one in, so demanding one
would refuse a shape Plumb itself defines.

The document now says what the code does. The rule holds wherever a capsule
exists, and where none can exist the deeds answer to the declared projection
surface instead. Publication remains the one irreversible point of a release.
