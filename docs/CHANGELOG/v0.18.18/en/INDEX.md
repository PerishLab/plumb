# Plumb v0.18.18

## Retiring a delivery chain

`plumb retire` destroys one declared delivery chain. It is the mirror of
release: everything it removes is something Plumb declared, published, or
protected.

A product opts in by naming its release bucket and Cloudflare zone under
`[release.retire]` in `plumb.toml`. The custom domain follows from the release
authority already declared, so it is never restated. A product that declares
nothing cannot be retired.

The dry run is the default and reads no credentials. `--execute` acts only when
`--confirm-repo`, `--confirm-bucket`, and `--confirm-domain` each equal their
target verbatim. The destructive order is fixed: inventory, archive and purge
credentials, revoke the writer token, detach the domain, empty and delete the
bucket, delete the repository, remove the local escrow.

The credential purge names the secrets Plumb itself writes, and reports what it
removed separately from what was already absent, so the line cannot claim a
purge it did not perform.

## Ephemeral authority

Retirement mints its own scoped tokens rather than holding standing ones. A
short-lived account token and a bucket-item token are cut from the factory
declared in `PLUMB_RETIRE_ACCOUNT`, `PLUMB_RETIRE_TOKEN`, and
`PLUMB_RETIRE_API`, bounded by an expiry, and revoked in an arm that runs
whether the sweep succeeded or failed. Both permission groups resolve before
either token is cut, so no failure path leaves a minted token behind. A derived
token never enters command arguments or logs.

## The cold-start law, redrawn

The previous law said Plumb neither creates nor mutates external resources.
That had already stopped being true: Plumb writes branch protection, cuts
release branches, raises and merges pulls, writes objects into R2, and deploys
Cloudflare workers.

Plumb now states that it owns the complete life of a governed release surface,
including retirement, and that it may derive ephemeral authority to act on what
it governs. The cold-start prohibition survives, restated as an implementation
constraint: Plumb provisions nothing that does not already exist, and the
guarantee is that no provisioning call is written here at all.

## The vendor layer

The outward-service layer was named `forge` while it already exported a generic
HTTP sender and only ever spoke to Forgejo. It is now `vendor`, with the
Forgejo client under `vendor::forgejo` and a Cloudflare client under
`vendor::cloudflare`.

The shared sender previously hardcoded a Forgejo authorization scheme, which is
why the site lane carried a duplicate HTTP path. It now takes the complete
header value. Site binding, token verification, and worker lookup fold onto the
shared client; the public fingerprint readback keeps a dedicated fetch, because
that probe reads HTML and must not announce a JSON `Accept` header.

The `vendor` Cargo feature replaces `forge`. No published consumer enabled the
old one.
