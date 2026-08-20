# Plumb v0.29.0

## Configuration travels without a release

Plumb's rules and its lane templates now publish as immutable objects to a
depot of Plumb's own, and a synced seat answers before the compiled bytes do.
The compiled bytes remain, permanently: a tool that cannot run without reaching
the network fails exactly where it is needed most.

The guarantee the rules directory carried survives the move. The depot's
authority is Plumb's, written by Plumb's own lane from Plumb's own recorded
roots, so a product still cannot grant itself a shape by declaring one. It can
only follow the shape Plumb published.

A version is a timestamp and nothing else, because a total order needs no verb
to advance and no judgement to compare. The channel pointer naming the current
one is the only movable object in the store. Nothing pins.

## Two laws stand on the seat

`depot.roots-published` holds that every configuration root a repository
records is carried by the held version under the same digest.
`depot.schema-supported` holds that the running binary is at or above the floor
that version declares. Both read the held seat and never the network, so a
stale seat is an observation while a seat that exists and cannot be read is
blind.

## Release notes moved, and became mutable

A release note is a seat, not a gate. Every released version has one known
address in the depot where migration guidance lives, and a version whose note
says only that the source is the guidance has still occupied it.

Notes are mutable, and they may be because ship is not. Identity is paid once,
at the version and its seal, so a note about that version cannot mislead a
reader about which version it describes. It can only be improved, which is what
a migration guide needs: a hazard found three days after a version shipped
belongs in that version's note.

Compiling a release no longer proves a note, so a note can never fail a
release. Seals carry no changelog proof; those written before this rule are
still read, never written again.

## Seats have one truth source

`plumb::seat` names the three seats a tool may hold, and Plumb reads its own
global seat through it rather than deriving one in place.

## The substrate is no longer required to be latest

`deps.first-party-stable-latest` no longer refuses a repository for consuming a
Plumb that is not the live stable latest. The depot floor decides currency now.
A dependency that cannot be read still refuses.
