# Plumb v0.27.0

Most of these remove one thing: a claim standing beside the mechanism instead of
taken from it — a second document about what a command does, a constant nobody
measured, a name accepted where a digest was available.

## Locking the contract is one job, so it is one family

`plumb stable` is gone. Its verbs are `plumb release prepare|pick|freeze|rejoin`,
and `plumb release stamp|retract` make and remove a release point for either
channel.

The contract is a version, a lock form, and a projection surface. Two were in
`release` and the third was not: a line is the apparatus by which a version is
fixed to a commit and made permanent, and that is the lock form. It looked like
a lifecycle of its own only because it has its own git objects — but a line is
cut for one release, carries that release's points, and rejoins when it ends.

The alternation said so: in order a release read `stable prepare → stable pick →
release stamp → dispatch → stable freeze → release stamp → dispatch → stable
rejoin`, and every arrow crossed a family boundary. The split was by
implementation medium — git refs on one side, the authority on the other — a
detail wearing a family name.

An exact release now starts from a point a mechanism made: the machinery was
always there, annotated and refusing to move a point that stands, but only
`freeze` could reach it, so a beta began with `git tag` by hand. `freeze` no
longer stamps, so the invariant it carried is restated where it can be checked —
dispatching stable refuses unless its point stands at the line head.

`rejoin` was in both families meaning two things, performing the merge and proving
the topology. It is one verb: given a version it performs and proves, given none
it reads the run's environment and proves what a run already did. `ship` is
untouched — it executes against what was locked, and the binary is one of its
adaptors like any other.

## A line generates itself

A release that repairs the release mechanism was built by the mechanism it
repairs. v0.26.0 answered this with a one-time bootstrap, retired a commit later
as an exception that had served its purpose; the next version needed it again.

Every exact publication now points its channel at what that run published, and
the install step reads that pointer, so a run installs the newest exact Plumb
and falls back to canonical stable only when none stands. An exact run carries
the credential to move its own channel pointer, which it did not before. The
generator also records what it was rather than claiming stable: a prerelease
binary seals `exact-release` provenance naming its own published seal, and that
provenance must name a point on the line being released.

## A dry run walks the course it would run

`--dry-run` printed a second document, hand-written beside the code it described.
Both drift directions were measured: `plumb land` planned a query the client never
sends, and `plumb stable freeze` printed two lines while doing six — the sixth
being `git tag`, pushed. An irreversible action the preview never mentioned, in
the one review a release gets before the irreversible part.

What blocked a single path was that a preview could not know what it had not done
— an obstacle that was self-imposed, because a read is not a mutation. A dry run
now performs every read and skips only the writes, so it resolves whether a branch
stands and prints the commit it would tag. Every mutation is declared as one step
whose first argument is the sentence describing it, so a write that reaches the
remote undescribed is not expressible, and a preview that cannot read what it
needs refuses where before it printed a confident plan.

## The npm evidence is the tarball, not the name

The module adaptor skipped publishing when the registry answered that the version
existed, then fetched `dist.shasum` and discarded it. The strongest claim the npm
lane could make was that a name exists.

Whether npm could do better was unknown, so nothing was done. It is known now,
and measured: a fresh clone at v0.26.0, installed from the lockfile, stamped and
packed, reproduces the published tarball byte for byte. So `carried` reads
`dist.integrity` and compares before publishing and after, and either mismatch is
`published module drift`. The first comparison failed on key order in
`package.json`, because `stamp` parses into a map that sorts — reproducibility
here rests on that, and rested on it by accident.

## A rendered stable lane advances the pointer

Shifting the managers and advancing the consensus pointer are two operations, and
the rendered lane ran only the first. A stable release therefore published every
object, reported activation, and left the channel naming the release before it;
the smokes then installed what was still canonical and reported the mismatch they
exist to catch.

`v0.26.0` shipped with this in its renderer; its own release was finished by
hand, and `sidecar v0.8.0` could not complete at all because the deed needs
credentials that exist only inside a lane (#346). Any repository whose lanes came
from v0.26.0 carries it, and rendering again is what removes it.

## Numbers that measure something

The width an attachment could declare was a table whose numbers were whatever the
products in front of it had needed, which is a cap a product sets for itself with
extra steps; `ceiling = 10` is one number for every attachment. The agent and
design document budgets held 320, a number measuring nothing, in the document a
reader opens first; they now measure their source inside a corridor.

## packport becomes rejoin

`packport` is not a word, and `backport` is not this operation: a backport carries
a fix to an older line, and this merges the frozen line into main so the stable
commit is reachable from it. The old word enters the retired dictionary, which is
what turns a rename into a mechanism — a tracked byte still carrying it is out of
true, here and in every repository the next release reaches.

## Smaller truths

The gate before projecting a medium read the capsule off local disk — an upload
plan, which proves what was built rather than what stands. It now reads the seal
back from the authority and compares the digest.

`oci` had no object in the input ledger, so the seal answering which media moved
said nothing about the image. An image is now an object: its `Containerfile`,
whatever `[release.depends]` adds, and the release version where it wraps this
release's archive.

The Debian package wrote the semver string into the dpkg `Version` field
verbatim, where `-` separates upstream from revision and an empty revision sorts
below a present one — so a prerelease claimed to be newer than the release it
precedes. Only the first `-` becomes a tilde now.

A landed commit said its subject twice, because the title came from `%s` and the
body from `%B`. `doctor` names an installed brief whose version is not the running
binary's, as an observation and not a finding. A Cargo rehearsal says which
packages it cannot rehearse, since an attachment holds one unit whose packages pin
each other.
