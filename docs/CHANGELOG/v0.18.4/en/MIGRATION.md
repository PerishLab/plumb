# Migrating to Plumb v0.18.4

Nothing is asked of a repository. No rule, declaration, or configuration
surface changes, and a doctor that was clean stays clean.

Upgrade if your doctor reports a blind reading of `deno.lock` for a
first-party package — for example `@perish/sealkit has 0 matching resolutions
for jsr:@perish/sealkit@^0.3.1`. Before this release there was no local way
out of it: the requirement rule refuses a tilde as a pin, while the lock
lookup could only find the tilde Deno had written. Both halves are satisfied
now, and the unversioned first-party form v0.18.3 asks for was never affected.

Expect the release to surface findings rather than remove them. The blind was
suppressing the dependency rules for that package, so a stale resolution
hidden behind it becomes visible on the first run after upgrading. That is
the finding v0.18.3 intended to report.
