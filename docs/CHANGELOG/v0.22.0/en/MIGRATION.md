# Migrating to Plumb v0.22.0

## An image or chart attachment needs a job image carrying its client

`ship oci build` runs `docker build` and `ship chart package` runs
`helm package`. Declaring `[release.oci]` or `[release.chart]` passes Doctor on
the strength of the declaration alone, so a repository whose lane image carries
neither client will declare a medium it cannot reach and fail at the deed.

The shared `release-binary` lane now runs a Forge image carrying both clients
and names `DOCKER_HOST` for them. A repository calling that lane needs nothing
further. A repository running its own lane must supply both the clients and the
endpoint before it declares either attachment.

## Nothing is required of a repository that declares neither

The image, chart and module deeds return to the shared lane for every product it
serves. A product declaring no such attachment opens no endpoint, authenticates
nowhere and pays nothing: each deed answers that it has none and succeeds.
