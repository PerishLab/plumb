# Plumb v0.17.0

Plumb sees a cargo family as something a repository publishes. The taxonomy
knew binaries and JSR packages, so a Rust library family released to a registry
declared nothing and answered to nothing: no release lane was required of it,
no operator wrapper was checked against it, and the vocabulary that separates
shipping a site from releasing a package could not reach it.

A release table naming binaries ships a binary, and one naming cargo ships
crates. The earlier test asked only whether a release key was present, which
meant a repository declaring cargo alone would have been read as a binary
publisher and asked for lanes it has no use for.

Plumb declares both surfaces and now carries the operator wrapper its own law
asks of any repository publishing registry packages. The wrapper calls the
local build, the way the guard already calls its own doctor, because a tool
cannot route its own release through a copy of itself that does not exist yet.
