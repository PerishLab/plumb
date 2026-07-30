# Plumb v0.17.1

Deno writes semantically equivalent requirements in canonical form inside its
lockfile. In particular, a declared pre-1.0 caret such as `^0.2.1` is recorded
as `~0.2.1`. Plumb v0.17.0 looked only for the declared bytes and therefore
reported a valid, frozen Sealkit `0.2.1` resolution as blind.

Plumb now tries the declared lock key first, then follows the unique Sealkit
entry in Deno's direct workspace dependencies. It still judges only local
bytes, never edits the lock, and refuses an absent or ambiguous direct
dependency rather than choosing among candidates.
