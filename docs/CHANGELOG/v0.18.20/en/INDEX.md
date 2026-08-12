# Plumb v0.18.20

This release makes the closed document strategy model the only governed
repository document surface.

- Every repository carrying `plumb.toml` must declare one affirmed `agent`
  document and may declare the closed `architecture`, `design`, and named
  `brief` strategies.
- The standalone skill strategy, generic affirmation schema, their Doctor
  rules, and the `plumb lock` command are removed after the v0.18.19 bounded
  compatibility window.
- Doctor derives top-level layout from the shared Git index snapshot. Empty or
  untracked directories can no longer make a local worktree disagree with a
  clean remote checkout.
- Doctor JSON and the rule catalog no longer expose the retired skill-shape or
  generic-lock fields and rules. Brief documents remain fully governed through
  the document report and document rules.

The release does not change installed skill management, release locking, Cargo
or Deno dependency lock inspection, or manager process locks. Those are
separate current product surfaces.
