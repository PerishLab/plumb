# Plumb v0.30.0

## The outside is frozen and the inside may move

This release freezes four surfaces: the public `plumb` library API, the 17
top-level commands and their arguments, the 121 rule identifiers, and the
shape rendered into `ectropy.toml`. The command, shape, and judge internals may
now be rebuilt without asking downstream repositories to follow that work.

## A release declaration speaks before release day

Doctor reads `[release]` with the same strict reader used by every release
verb. A malformed declaration is `out of true`, including a key accidentally
captured by an attachment table. A declared attachment with no lane capable of
delivering it is also named locally.

When Plumb judges its own repository, a released binary whose embedded build
commit is an ancestor of the newer tree reports `blind`. Development builds and
ordinary product repositories make no such claim.

## Stable ancestry reads current remote truth

`release prepare` and `release freeze` fetch origin before checking that the
previous stable point is held by main. A stale `origin/main` can no longer let
the next stable line pass. Rejoin remains an explicit line operation; Plumb
automates the gate, not the merge decision.

## The path budget is three

The default Ectropy path limit is now 3, paired with fanout 10. Five files in
the old Plumb command tree retain narrow, named boundaries until the closure
rewrite removes them. The depot accepts the compiled and carried limit during
the publication window, then converges on the newly published value.

## Rendered guards adopt the depot first

Every rendered guard syncs the Plumb depot before asking which work to repeat.
Rules, taxonomy, policy constants, policy conditions, lane templates, and help
resources can therefore move together as one immutable depot version.
