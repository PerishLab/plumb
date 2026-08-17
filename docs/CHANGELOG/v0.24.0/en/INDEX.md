# Plumb v0.24.0

## A rendered lane is checked against the forge that must run it

v0.23.0 rendered four workflows and none of them had ever been parsed by
Forgejo. Three shapes in them could not run, and no local test could say so,
because a lane's body is only read by the forge.

The exact release lane started from a tag push, so a pushed tag would have
released on its own and collided with the dispatch an operator sent. It also
forwarded no guard contexts, and an empty context list is not a refusal but a
silent excuse: `release evidence` answers that no evidence is required. Both are
regressions against the thin caller it replaced.

Every lane installed its tools with the published manager, which seats them under
the home directory, and then called them from a shell whose path never held that
seat. The guard failed on its last proof and the ship lane would have failed on
its first.

The ship lane declared its secrets under `workflow_call`, which this forge parses
for `inputs` and `outputs` alone. The whole file was unplannable, so every push
and every pull request produced a run that failed in zero seconds with no job and
no log, and the status it left blocked landing. The shared workflow it replaces
never declared them either: a callee reads what its caller maps.

Rendering now refuses a lane that carries any of those shapes, and names which
one. The check runs before a file is written, so a template edit that
reintroduces a known-fatal shape stops at the author rather than at someone
else's release.

## This product ships through its own lanes

`plumb` renders and carries its own four workflows, dispatch names the rendered
ones, and nothing here reaches the shared workflow repository. A repository that
has not rendered yet is untouched by this: its next release refuses before it
reaches the forge, naming the absent lane and the command that writes it, while
every other unrendered lane still only draws a note.

The module attachment returns with ordered packages and the worker joins it, so
the site is a declared medium rather than a lane of its own. Ship forwards the
credential a worker needs beside the registry token.

## The plan names the deed that prepares each medium

The projection step ran `plumb ship <medium> rehearse` for every medium, and only
Cargo and a worker hold that deed: an image builds, a chart packages, a module
packs. Three of five projections would have failed on an unrecognised subcommand.

Which word prepares a medium is a fact Plumb holds, so it belongs in the plan
rather than in the lane. The projected matrix now carries it beside the medium,
the lane spends it, and a test asks this binary whether every deed the plan names
exists at all.

## A published seal without input hashes is a baseline, not a refusal

Every stable seal published before v0.23.0 carries no input hashes, because
nothing recorded them yet. Reading one was treated as a malformed answer rather
than as an empty baseline, so `release compile` refused — and since every product
in this domain has exactly such a seal standing as its current stable, v0.23.0
could compile no release at all, including its own successor.

An absent field now reads as what it is: nothing is known about what moved, so
everything is projected and every object is stamped with this release. Only a
field that is present and malformed refuses.

The regression was invisible until now because a release always runs on the Plumb
already published, never on the one being released, so a fault in the release
engine surfaces one release late.
