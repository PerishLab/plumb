# Plumb v0.18.12

## Release radius

`plumb radius` answers, for one product and one candidate version over a
supplied set of repository roots, which declared seats currently resolve below
it. It reads each root's `Cargo.lock`, `deno.lock`, and `.runseal/deno.lock`,
compares by semver against the candidate the caller names, and needs no
registry and no network.

A root that cannot be read, a path that is not a directory, and a resolution
that is not a version are each reported blind with their reason rather than
counted as current. A workspace link is not a consumed registry version and is
excluded.

The evaluator takes no domain vocabulary and reads no private control-plane
state, so a delivery coordinator supplies the roots and Plumb supplies the
judgment. Like `precommit` it is an operation, not a Doctor rule, and it emits
no finding.

## Truthful release-line cuts

`plumb stable prepare` reported that it prepared a line from the base even when
the branch already existed and had not moved. An existing line standing at the
base is now reported as held, and an existing line at another commit refuses:
a release line is a frozen audit boundary and prepare does not move one. An
unreadable commit on either side refuses rather than comparing nothing.
