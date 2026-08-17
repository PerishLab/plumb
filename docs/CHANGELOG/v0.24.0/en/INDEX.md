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
