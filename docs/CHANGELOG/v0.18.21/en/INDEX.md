# Plumb v0.18.21

This release admits Grok as a managed agent skill seat.

- `plumb skill` discovers `~/.grok` the same way it discovers `~/.claude`
  and `~/.codex`, and installs the owned brief at `~/.grok/skills/plumb`.
- Ownership, ledger, force, upgrade, and uninstall are unchanged. An
  unmanaged path is still refused even when a matching marker is present.
- Concord and Ectropy inherit the seat when they next resolve `plumb` with
  the `skill` feature to this stable.

The release does not change document strategies, release locking, or the
default stable-only managed channel.
