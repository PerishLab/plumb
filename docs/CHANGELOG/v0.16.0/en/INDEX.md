# Plumb v0.16.0

This release makes canonical stable a repository-wide consensus anchor.
Non-stable channels remain exact, immutable validation candidates and may only
install into explicit isolated seats.

Plumb now owns the complete binary release mechanism: Cargo discovery and
version stamping, target builds, archives, skills, Debian packages, Cargo
attachments, generated managers, sealed storage, verification, activation,
smoke, and stable tags. Operational values enter `plumb release` through typed
environment fields.

Every Plumb-owned archive now has canonical ordering, timestamps, ownership,
and modes, while MSVC binaries use reproducible linking. Rebuilding one exact
version from one commit therefore reproduces the same immutable seal instead
of depending on runner or linker timestamps.

The Plumb skill also records repetition as an ownership signal: strongly
repeated Plumb-shaped mechanisms should be surfaced for possible substrate
absorption when their closure is clear.
