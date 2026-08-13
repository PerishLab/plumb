# Plumb v0.18.25

The living web policy now derives its source vocabulary from the repository it
inspects.

- A web seat containing Svelte sources admits `.svelte` files and excludes
  generated `.svelte-kit` output.
- A Svelte-only seat no longer inherits a TSX grant. Mixed seats retain both
  source forms when both are present.
- Frozen release commits now run the same push guard required by exact and
  stable coordination.

This lets Svelte repositories preserve their own component vocabulary while
keeping the release commit itself as the guarded publication boundary.
