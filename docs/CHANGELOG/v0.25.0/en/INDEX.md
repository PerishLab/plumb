# Plumb v0.25.0

## An image takes its payload from the authority that published it

An image wrapping a binary read the archive from a local artifact directory and
labelled itself with that file's digest. The claim — this image carries that
product — rested on a file only the runner had seen, and the projection job of a
rendered lane holds no such directory at all, so the medium could not project
there.

Where the archive is absent, the payload now comes from the published release:
the seal names the object, the object is fetched from the authority, and its
digest must equal the one the seal records or the projection refuses. The label a
consumer reads is then the digest of the object the public authority serves,
which is what the claim always meant.

A lane that holds the archive is unchanged, because the shared workflows build an
image before they publish anything, and the archive is all they have. A rendered
lane projects after the seal exists, which is why it can ask the authority
instead.
