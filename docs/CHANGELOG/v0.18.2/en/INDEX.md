# Plumb v0.18.2

## Explicit target observation

Plumb can now bind one caller-selected CLI target fact to the product-owned
`plumb.target` context role. The ordered collector chain supports one named
environment value, one indexed argument, or one named process fact through
Locus 0.1.1.

The surface remains disabled by default. Plumb does not infer a repository
identity, scan argv or environment state, or expose a workspace path unless
the caller explicitly selects that exact fact.

## Frozen provenance

The start atom records which role, binding, collector, and selector produced
the fact. The finish atom inherits the same readonly context without sampling
again. Only absence advances an explicit collector chain; invalid or unreadable
facts refuse the audit record through the Locus diagnostic hook.
