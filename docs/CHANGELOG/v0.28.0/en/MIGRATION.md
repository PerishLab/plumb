# Migrating to Plumb v0.28.0

## Render the lanes again

Every governed repository's guard moves one line: `cargo test` now runs after
the release check. Render and land it at your convenience — the drift is noted,
never out of true, and a repository declaring nothing keeps exactly the behaviour
it has today.

## Declaring is opt-in, and a narrow declaration is the one risk

A repository with no `[workflow.hash]` renders as it did before and asks nothing
of the lock. Declare a key and the step it covers runs only when the paths you
named moved.

Nothing mechanical catches a declaration that is too narrow, because only the
product knows what its own steps read. What a narrow one buys is a step skipped
that would have failed. The trap worth naming: a step that runs this tool against
this repository reads this repository. In this repository `cargo test` asserts
that `plumb doctor` finds the tree true, so its input is the whole tree and not
the crates alone, while the formatter, the linter and the release check read only
what Cargo reads.

Two rules keep a declaration honest. Merge keys whose input is the same, because
splitting them buys nothing. Merge keys where one step is another's prerequisite,
because skipping a package install while running the check that needs its modules
is a failure no declaration described.

Use `plumb workflow status --since <rev>` before trusting a shape. It replays the
declaration across real history and says how often each key would have held.

## The lock seat needs no per-repository setup

The four settings reach every governed repository through the organisation, and
the lane names them itself. A new repository declares `[workflow.hash]` and is
done.

## Nothing happens until this version is the installed stable

A lane installs the released tool, and the released tool is by construction older
than the lane that installs it. Asking and recording both tolerate their own
failure, so a guard rendered by this version and run by an older one behaves
exactly as it does today: everything runs, nothing is recorded, and the seat
stays empty until the tool that understands the question is the one installed.
