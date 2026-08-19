# Plumb v0.27.0

Twelve cuts. Most remove the same thing: a claim standing beside the mechanism
instead of taken from it — a second document about what a command does, a
constant nobody measured, a name accepted where a digest was available.

## A dry run walks the course it would run

`--dry-run` printed a second document, hand-written beside the code it claimed
to describe. Two texts drift, and both directions had been measured: `plumb
land` planned a query the client never sends, and `plumb stable freeze` printed
two lines while doing six — the sixth being `git tag`, pushed. An irreversible
action the preview had never mentioned, in the one review a release gets before
the irreversible part.

What blocked a single path was that a preview could not know what it had not
done: the commit a branch stands at, whether the line exists at all. Hence the
placeholders, hence the second text. That obstacle was self-imposed, because a
read is not a mutation. A dry run now performs every read and skips only the
writes, so at the moment each write would happen it knows what the real run
knows. `prepare` resolves whether the branch stands rather than describing both
outcomes; `freeze` prints the commit it would tag rather than `<head>`.

Every mutation is declared as one step whose first argument is the sentence
describing it, so a write that reaches the remote without a line describing it
is not expressible. A preview that cannot read what it needs now refuses,
where before it printed a confident plan to freeze a branch that was not there.

## The npm evidence is the tarball, not the name

The module adaptor skipped publishing when the registry answered that the
version existed, which proves only that something was published under that
identity. After publishing it fetched `dist.shasum` and discarded it, checking
that the command exited zero. The strongest claim the npm lane could make about
a release was that a name exists — weaker than cargo, S3, oci and chart, and
weaker than the changelog shipped beside it.

Whether npm could do better was unknown, so nothing was done. It is known now,
and it was measured: a fresh clone at v0.26.0, installed from the lockfile,
stamped and packed, reproduces the published tarball byte for byte. So the
strong check applies. `carried` reads `dist.integrity` — sha512, what an npm
client verifies for itself — and compares it before publishing, to decide
whether the version standing there is this projection, and after, so the
fetched digest is read rather than discarded. Either mismatch is `published
module drift`, the refusal oci and chart already raise.

One thing surfaced on the way. The first comparison failed, and the only
difference was key order in `package.json`: `stamp` parses into a map that
sorts, so the manifest is alphabetised in passing. Reproducibility here rests
on that, and rested on it by accident.

## A medium publishes only after the release stands

Before projecting cargo, npm, oci or chart, the gate read the capsule off local
disk and checked the version inside it. A capsule is an upload plan, built
before anything is uploaded, so it proved that this machine compiled the
version — not that the release stands anywhere. The media announce a version to
registries the world reads.

Inside the lane the gate was never what held the line; the projecting jobs need
the sealing job, and that ordering already guaranteed it. The gate matters where
the ordering is absent, which is an operator finishing a release by hand. It now
reads the seal back from the public authority and compares the digest, using the
fetch `plumb ship binary verify` already performs. The capsule carries that URL
and digest, so no job learns a new variable. It checks the seal alone rather
than every object, and fires only for a medium the product declares.

## One ceiling for every attachment

The width an attachment could declare was a per-attachment table, and the
numbers were whatever the products in front of it had needed. A cap set by
observed pressure is a declaration a product makes for itself with extra steps,
which is the thing the file says it exists to prevent. `ceiling = 10` is one
number for every attachment, and the table carries only departures from it.

## The agent budget measures what it documents

`document.magnitude` applies a magnitude to the target and the evidence mass.
Architecture scaled on source leaves and brief on source lines, each inside a
corridor; agent and design held 320, a number that measures nothing, in the one
document a reader opens first. A constant is wrong at both ends: it did not move
while this repository grew, so pressure landed on the sentences rather than on
the count of things worth saying, and it allowed a repository of a dozen files
the same 320 lines, which is no guard at all. Agent and design now share the
architecture arm, so the guard tightens where it was doing nothing and yields
where the repository earned it.

## packport becomes rejoin

`packport` was not a word. It sat where `backport` would go, and a backport is
not this operation either: a backport carries a fix to an older line, and this
merges the frozen release line into main so the stable commit is reachable from
it. The direction is opposite and the cargo is different — one carries content,
this carries only reachability. `rejoin` reads straight through the lifecycle:
prepare, pick, freeze, rejoin.

The old word enters the retired dictionary, which is what turns a rename into a
mechanism: a tracked byte still carrying it is out of true, here and in every
repository the next release reaches. Frozen changelog history keeps it, because
source projection excludes that seat and immutable history is not a thing to
correct.

Alongside it, a rejoin that conflicts now has one named resolution rather than
a judgement call taken differently each time.

## The seal speaks about the image it shipped

The input ledger covered every declared attachment except one. `oci` had no
object, so the seal answering "which media moved since the last stable" said
nothing about the image — a hole exactly where nobody would look for one. An
image is now an object: its `Containerfile`, plus whatever `[release.depends]`
adds, and where the image wraps this release's archive the version folds in too,
because such an image moves with the release however still its source has been.

## Locking the contract is one job, so it is one family

`plumb stable` is gone. Its four verbs are `plumb release prepare|pick|freeze|
rejoin`.

The contract this release exists to model is a version, a lock form, and a
projection surface. Two of those were in `release` and the third was not: a line
is the apparatus by which a version is fixed to a commit and made permanent, and
that is the lock form. It looked like a lifecycle of its own only because it has
its own git objects. It is not one — a line is cut for one release, carries that
release's points, and rejoins when the release ends. It has no existence outside
one.

What made it visible was the alternation. Written in order, a release read
`stable prepare → stable pick → release stamp → dispatch → stable freeze →
release stamp → dispatch → stable rejoin`, and every arrow crossed a family
boundary. The split was not by object or by phase but by implementation medium —
git refs on one side, the authority on the other — which is a detail wearing a
family name.

`rejoin` existed in both families meaning two things: performing the merge, and
proving the topology. It is one verb now. Given a version it performs and proves;
given none it reads the run's environment and proves what a run already did,
which is what the shared workflow still calls.

`ship` is untouched. It executes against what was locked, and the binary is one
of its adaptors like any other.

## A line and its points are different objects

Plumb stamped the stable point inside `freeze` and left the exact one to a hand:
the documented way to begin a beta was `git tag` followed by `git push`. The
machinery for doing it properly already existed — annotated, idempotent,
refusing to move a point that already stands — and had exactly one caller.

Giving the exact point its own verb made the older seam visible. The verb
families were organised by who runs them, while the model they implement is
organised by what they act on, and `stable` held both the line and one of its
points. A line is always stable; its points are not.

So the objects are separated. `plumb stable` opens, picks onto, freezes and
rejoins the line, and touches no point. `plumb release stamp` fixes a version
name to the head of the line it names and `plumb release retract` removes one,
both for either channel, because a point is what a tag means and a tag is where
the release segment begins.

`freeze` no longer stamps, so the invariant it carried by construction is
restated where it can be checked: dispatching a stable release refuses unless
its point already stands at the line head. A stable release is now declared
before it runs, in a step that says so. The bare `git tag` leaves the procedure,
and `retract` stops being stable-only — an exact point can be removed by the
verb that made it, reading its own channel from the authority.

## A rendered stable lane advances the pointer

Shifting the managers and advancing the consensus pointer are two operations,
and the rendered lane ran only the first. A stable release therefore published
every object, rewrote the managers, reported activation, and left
`v1/channels/stable.json` naming the release before it. The smokes then
installed what was still canonical and reported the mismatch they exist to
catch — which is the lane telling the truth about a lane that had not.

`v0.26.0` shipped with this in its renderer. Its own release met it and was
finished by hand; `sidecar v0.8.0` met it as the first stable release rendered
by that version and could not complete, because the deed that writes the pointer
needs credentials that exist only inside a lane (issue #346). The rendered lane
now runs both deeds and reads the activated surface back, as the shared workflow
always did.

Any repository whose lanes were rendered by v0.26.0 carries this. Rendering
again with this version is what removes it, and until then a stable release
cannot reach consensus.

## Smaller truths

The Debian package wrote the semver string into the dpkg `Version` field
verbatim. dpkg reads `-` as the upstream/revision separator and sorts an empty
revision below a present one, so a prerelease claimed to be newer than the
release it precedes. Only the first `-` becomes a tilde now. Nothing is hit by
this today — no apt repository stands in this release path and the installer
never touches the `.deb` — but a published artifact should not carry a claim
that is false the moment anything reads it.

A landed commit said its subject twice, once as the subject and once as the
first line of the body, because the title came from `%s` and the body from `%B`.
Every commit on this line carried it.

`doctor` names an installed brief whose version is not the running binary's,
wherever it runs. It is an observation and not a finding: the operator's seats
are not the repository's business, and a repository that is true stays true
beside a stale brief.

A Cargo rehearsal now says which packages it cannot rehearse and why, because
an attachment holds one unit whose packages pin each other exactly and the
second cannot be built from its packaged form until the first is published. The
ordered publish is what proves them. The bootstrap that generated v0.26.0 is
retired now that the release it bootstrapped stands on its own.
