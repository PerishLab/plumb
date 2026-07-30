# Migrating to Plumb v0.18.0

Install Plumb v0.18.0 before adopting the new release workflows and move the
release wrapper to Sealkit `^0.3.1`. Refresh the frozen Deno lockfile with a
zero dependency-age floor when the release is still new:

```sh
deno install --config .runseal/deno.json --lock .runseal/deno.lock \
  --minimum-dependency-age=0
```

Product exact and stable workflows no longer declare a second `ref` input.
Dispatch the workflow on the desired source branch and pass only the exact
version or promotion inputs; the caller forwards the event ref and SHA to the
shared workflow.

Prepare a stable line locally:

```sh
runseal :release prepare --version vX.Y.Z
runseal :release pick --version vX.Y.Z --commit <sha>
runseal :release freeze --version vX.Y.Z
runseal :release --channel stable --version vX.Y.Z \
  --promotion-version vX.Y.Z-beta.N --watch
runseal :release packport --version vX.Y.Z
```

Stable dispatch derives `release/vX.Y.Z`; remove any stable `--ref`. Packport
deletes the settled branch by default; pass `--keep-branch` only when the source
history must remain as a branch.

Do not create a release tag. Existing tags remain historical records but are no
longer consulted as release consensus.
