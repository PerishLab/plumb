# plumb

A plumb line for repositories.

This is the workshop's living skeleton: the operational envelope is worked out
here, published as a CLI, and kept honest by being run rather than described.
Every repository in the ecosystem should find its shadow here — and when one
cannot, that is plumb's debt, not the repository's exception.

- `crates/plumb` — the CLI. Hold a repo against the skeleton and report where it
  hangs untrue. Checking lands before scaffolding.
- `apps/web` — plumb.perish.uk, answering why, what and how. Deployed, so the
  skeleton's web path is exercised and not merely claimed.

A template that is only copied rots. This one is run.

## Paired Web/API dispatch

When a repository holds both a React/Vite application at `apps/web` and an
executable `api` crate at `crates/api`, `plumb doctor` checks the combination as
one product shape:

- `sidecar.toml` leases both development ports, waits for API endpoint
  readiness, and passes that endpoint to Web through a declared environment
  binding;
- API consumes `SIDECAR_PORT`, accepts the sidecar stamp, and emits its endpoint
  as API readiness;
- Web consumes `@perish/react-components`, activates
  `@perish/vite-plugin-design`, and takes its port and API binding from sidecar;
- production has separate API and Web image seats, separate chart workloads,
  Web-targeted ingress, and one Cargo/chart version train.

The rule is about the dispatch relationship. Product routes, binding names,
image names, and application-specific infrastructure remain the product's own
shape.
