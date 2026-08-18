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

## An exact release starts from a point a mechanism made

Plumb stamped the stable point and left the exact one to a hand: the documented
way to begin a beta was `git tag` followed by `git push`. The machinery for
doing it properly already existed — annotated, idempotent, refusing to move a
point that already stands — and was reachable only through `freeze`, which is
the stable path.

`plumb stable stamp --version <exact>` reaches it. It derives the line the
version belongs to, reads that line's head, and stamps there. It refuses a
stable version, which belongs to `freeze`, and `freeze` refuses an exact one.
The bare `git tag` leaves the release procedure.

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

The stable pointer advances before the rehearsal that cannot run until it has,
and the bootstrap that generated v0.26.0 is retired now that the release it
bootstrapped stands on its own.
