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

## A dry run walks the course it would run

`--dry-run` printed a second document, hand-written beside the code it described.
Both drift directions were measured: `plumb land` planned a query the client never
sends, and `plumb stable freeze` printed two lines while doing six — the sixth
being `git tag`, pushed. An irreversible action the preview never mentioned, in
the one review a release gets before the irreversible part.

What blocked a single path was that a preview could not know what it had not
done. That obstacle was self-imposed, because a read is not a mutation. A dry
run now performs every read and skips only the writes, so it resolves whether a
branch stands and prints the commit it would tag. Every mutation is declared as
one step whose first argument is the sentence describing it, so a write that
reaches the remote undescribed is not expressible, and a preview that cannot
read what it needs refuses where before it printed a confident plan.

## The npm evidence is the tarball, not the name

The module adaptor skipped publishing when the registry answered that the
version existed, and afterwards fetched `dist.shasum` and discarded it. The
strongest claim the npm lane could make was that a name exists.

Whether npm could do better was unknown, so nothing was done. It is known now,
and measured: a fresh clone at v0.26.0, installed from the lockfile, stamped and
packed, reproduces the published tarball byte for byte. So `carried` reads
`dist.integrity` and compares before publishing and after, and either mismatch is
`published module drift`. The first comparison failed on key order in
`package.json`, because `stamp` parses into a map that sorts — reproducibility
here rests on that, and rested on it by accident.

## A medium publishes only after the release stands

The gate before projecting cargo, npm, oci or chart read the capsule off local
disk. A capsule is an upload plan built before anything is uploaded, so it proved
that this machine compiled the version — not that the release stands anywhere. It
now reads the seal back from the public authority and compares the digest, using
the fetch `plumb ship binary verify` already performs. It checks the seal alone,
and fires only for a medium the product declares.

## A rendered stable lane advances the pointer

Shifting the managers and advancing the consensus pointer are two operations,
and the rendered lane ran only the first. A stable release therefore published
every object, reported activation, and left the channel naming the release
before it; the smokes then installed what was still canonical and reported the
mismatch they exist to catch.

`v0.26.0` shipped with this in its renderer. Its own release met it and was
finished by hand; `sidecar v0.8.0` met it as the first stable release rendered by
that version and could not complete, because the deed needs credentials that
exist only inside a lane (#346). Any repository whose lanes came from v0.26.0
carries it, and rendering again is what removes it.

## Numbers that measure something

The width an attachment could declare was a table whose numbers were whatever the
products in front of it had needed — a cap set by observed pressure, which is a
declaration a product makes for itself with extra steps. `ceiling = 10` is one
number for every attachment. The agent and design document budgets held 320, a
number measuring nothing, in the document a reader opens first; they now measure
their source inside a corridor, so the guard tightens on a repository of a dozen
files and yields where a repository earned it.

## packport becomes rejoin

`packport` is not a word, and `backport` is not this operation: a backport
carries a fix to an older line, and this merges the frozen line into main so the
stable commit is reachable from it. `rejoin` reads through the lifecycle:
prepare, pick, freeze, rejoin. The old word enters the retired dictionary, which
is what turns a rename into a mechanism — a tracked byte still carrying it is
out of true, here and in every repository the next release reaches.

## Smaller truths

`oci` had no object in the input ledger, so the seal answering which media moved
said nothing about the image. An image is now an object: its `Containerfile`,
whatever `[release.depends]` adds, and the release version where the image wraps
this release's archive.

The Debian package wrote the semver string into the dpkg `Version` field
verbatim, where `-` separates upstream from revision and an empty revision sorts
below a present one — so a prerelease claimed to be newer than the release it
precedes. Only the first `-` becomes a tilde now.

A landed commit said its subject twice, because the title came from `%s` and the
body from `%B`. `doctor` names an installed brief whose version is not the
running binary's, as an observation and not a finding. A Cargo rehearsal says
which packages it cannot rehearse, since an attachment holds one unit whose
packages pin each other. The bootstrap that generated v0.26.0 is retired.
