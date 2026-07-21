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
