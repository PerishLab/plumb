# Migrating to v0.17.1

Install Plumb v0.17.1 before migrating a repository to the supported Sealkit
`^0.2.1` line. No repository workaround is needed: keep the declared caret and
let the standard Deno install command write its canonical lock key.

Do not edit the lock key from `~0.2.1` back to `^0.2.1`. The lockfile belongs to
Deno; this release teaches Plumb to read Deno's representation.
