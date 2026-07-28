# Lock

Some things in a repository are bound to other things, and the binding is
invisible to every compiler that touches them: a packaging script naming a
crate, a chart naming an environment key, a brief describing what a checker
enforces. When one side moves and nobody looks at the other, the pair goes
quietly wrong and stays wrong until something expensive notices.

A **lock** is a declared pair and a stamp: the paths on one side, the version
the repository stood at when someone last read them, and a hash of what they
held at that moment. When either drifts, the doctor refuses.

## Declaration

Locks live in the repo-rooted `plumb.toml`:

```toml
[[lock]]
name = "skill"
paths = ["skills/plumb"]
version = "0.11.0"
hash = "317b2a9c..."
```

The hash covers the declared paths, walked in full, sorted by path, each file
contributing its relative path, its length, and its bytes. Path separators read
as `/` and `\r\n` reads as `\n`, so a checkout that carries windows line endings
digests the same as one that does not. The normalization is part of the
contract: two machines must agree on the digest or the mechanism becomes noise,
and a checkout setting is not a change to what the paths hold.

## The check

`plumb doctor` reads each lock and refuses when either side has moved:

- **The content changed.** Somebody edited what the lock covers without
  affirming it. Read it, then re-affirm.
- **The version moved.** The repository has cut a version since the last
  affirmation, so whatever the covered paths describe may no longer be true.
  Read them, then re-affirm — even when the answer is that nothing changed.

`plumb lock` prints the values a fresh affirmation would record. It does not
write them.

## What a lock is and is not

A lock buys **attention, not accuracy**. Nobody can check that a brief still
describes a binary; a machine can only insist that a person looked at the
question at the moment it became askable. Recording a hash without reading
what it covers satisfies the letter and defeats the whole purpose, exactly as
a signature copied without reading does.

Two consequences follow, and they are law:

- **No automatic affirmation.** Not in CI, not behind a `--write`, not in any
  fix-it path. A mechanism whose entire product is a human glance is destroyed
  by the first script that supplies the glance.
- **The window is real.** A lock fires when the version moves, so a binding
  that breaks in the middle of a version stays broken until the next cut. The
  packaging script that named a renamed crate was wrong for five releases; a
  lock would have caught it at the sixth, not the first. This is a smaller
  guarantee than it looks, and claiming more of it would be the same sin the
  skill law names.

## Which pairs earn a lock

The list stays short. A pair earns a lock when all three hold: the two sides
are genuinely bound, no checker can verify the binding today, and a real cost
has already been paid for the drift. Declaring every plausible pair turns each
release into a wall of affirmations, and a wall of affirmations is a wall of
reflexes.

When a binding becomes mechanically checkable, its lock retires — the check is
strictly better, and keeping both invites the two to disagree.
