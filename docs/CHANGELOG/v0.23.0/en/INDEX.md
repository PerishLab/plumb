# Plumb v0.23.0

## A governed lane is rendered, never written

Every repository kept its own workflows by hand, and every one of them drifted
alone. Nothing in a guard was ever a fact only a human held: the release
declaration says what the repository is, the repository shape says what proves
it, and the running Plumb carries the template.

`plumb lane` derives each governed workflow and `--write` records it. Drift is a
byte comparison against a fresh render rather than a recorded seal, because a
target Plumb can regenerate needs no evidence it cannot already derive.

Rendering decides which jobs exist, not only how wide they run. A product
declaring no binary renders a lane without the build, seal and smoke jobs. The
alternative was an empty matrix, and Forgejo does not skip an empty matrix job:
it never creates one, and every dependent then blocks forever.

Drift is noted rather than out of true. It cashes out only when a release runs,
so `doctor` reports it and stays green while a dispatch refuses. A lane never
rendered at all draws the note and nothing more, which is what lets a repository
adopt the mechanism on its own schedule.

## A stable release derives the exact one it promotes

Promotion arrived as two dispatch arguments, though the seal they named was
already checked for four things including an identical commit. A fact that
survives four checks is not a declaration.

Exact tags standing on the frozen commit are now the candidate set, and a
published seal on the authority is the count. Exactly one must stand there;
zero and many both refuse, with no heuristic. The refusal lands at
`stable freeze`, before an artifact exists, so a failed exact release is rerun
rather than followed by the next candidate.

## A worker is a medium a product declares

The account and domain a site published under arrived from lane variables, which
let a workflow decide where a product appears. `[release.cfworker]` declares
both, so the worker joins the projection surface and takes a job like any other
medium. Exact channels stage a version behind its own preview URL and never
touch traffic; only stable reaches the declared domain.

## A seal says what each object was built from

Every ship object now carries an input hash over the tracked leaves under roots
derived from where it already lives, together with the toolchain the rendered
lane pins. Each hash is stamped with the version those inputs first appeared in,
so an object that has not moved since an older release says so plainly.

## A projection passes over what did not move

The stamp saying which release an object last changed in was already enough to
know it need not be built again. Every adaptor now reads it: a crate, package,
chart or worker unchanged since an earlier release is named together with that
release and is not projected. A reader can tell a skip from an omission because
the projection says which one it is.

A skip is an optimisation and never a gate. A baseline that cannot be read
projects everything and says so instead of refusing, so the failure direction is
a release that publishes more than it had to, never one that publishes a lie.

Cargo carries the one consequence worth naming. A package that did not move
keeps the release it last published under, and every requirement on it names
that release, so a coupled pair stays pinned exactly without demanding a version
nobody built.
