# Migration

No declaration change is required. Node-only repositories can keep their
existing `apps/*/wrangler.jsonc` site contract.

If a root `Cargo.toml` was added only to satisfy the former deployment path,
remove it after confirming no other repository tool consumes it.
