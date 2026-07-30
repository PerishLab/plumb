# Plumb v0.18.3

## Live first-party dependency currency

Plumb now holds every direct published dependency owned by perish.code to the
registry's live stable latest. The rule covers the `@perish` JSR scope and the
`perish` Cargo registry without maintaining a second package-version table.

Deno declarations remain unversioned and their exact lock resolution must be
current. Cargo keeps its native requirement syntax, but its exact direct lock
resolution must also equal stable latest. Same-workspace path dependencies
remain release-train edges rather than published registry edges.

## Blocking evidence gaps

Doctor reads bounded live registry metadata and reports each dependency's
ecosystem, declaration seat, requirement, resolution, and latest version. A
known stale resolution is out of true. Missing or malformed manifest, lock, or
registry evidence is blind and now makes doctor nonzero; unknown repository
shape remains nonblocking.

Plumb only validates. It never edits a dependency declaration or lock.
