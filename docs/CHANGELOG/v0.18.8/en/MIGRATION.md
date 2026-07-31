# Migrating to Plumb v0.18.8

Install stable Plumb before removing a repository's existing release or site
wrapper. The CLI replacement must be available before Doctor accepts the
wrapperless shape.

For human release operation, use:

```sh
plumb release dispatch --channel beta --version vX.Y.Z-beta.N --ref <branch>
plumb stable prepare --version vX.Y.Z
plumb stable pick --version vX.Y.Z --commit <full-sha>
plumb stable freeze --version vX.Y.Z
plumb release dispatch --channel stable --version vX.Y.Z \
  --promotion-channel beta --promotion-version vX.Y.Z-beta.N
plumb stable packport --version vX.Y.Z
```

Exact channels retain branch freedom. Stable alone uses `release/vX.Y.Z`.
Do not delete the release branch after packport; it remains the immutable
source and audit boundary.

A site repository keeps its dispatch-only deploy workflow and
`apps/*/wrangler.jsonc`, installs stable Plumb in the workflow, and calls:

```sh
plumb site plan
plumb site inspect
plumb site deploy
```

Set `PLUMB_SITE_ACCOUNT`, `PLUMB_SITE_DOMAIN`, and `PLUMB_SITE_TOKEN` for
inspection or deployment. A known blind vantage uses
`PLUMB_SITE_BLIND=true`; it cannot excuse an unknown binding.

An env-only Runseal profile carries only `RUNSEAL_REPO_*` paths. Guard invokes
Plumb and Ectropy directly. Remove product-owned release, ship, and cold-start
wrappers after their owning CLI or control plane is available. Plumb consumes
already provisioned release authority and never mints or synchronizes it.
