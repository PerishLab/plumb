# Plumb v0.18.6

## The workflow is the repository fact

A governed repository now carries one canonical guard workflow without being
required to keep generic wrappers or repository-owned Git hooks. The workflow
itself proves that guard runs Plumb and Ectropy, exercises the Rust release
profile, and builds a governed web package when present.

Transitional and product-specific wrappers remain visible to Doctor and must
keep a known role, but their absence is valid. Generic init, land, and release
behavior belongs to the substrate entrypoint that owns it rather than to a
copied file in every repository.

## Strict typed cascades

The Cascade derive accepts `#[cascade(strict)]` for a top-level configuration
and `#[cascade(section, strict)]` for a nested section. Strict file partials
reject unknown fields instead of silently accepting a misspelled or obsolete
profile key.

Strictness is explicit. Existing cascades keep their current behavior until
their product vocabulary is ready to close the file boundary.
