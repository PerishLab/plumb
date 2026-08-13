# Migrating to Plumb v0.18.28

No command or configuration names change. Continue to provide `PLUMB_SITE_*`
for site operation and `PLUMB_RETIRE_*` for an explicitly confirmed
retirement.

Consumers that build the `plumb` crate's `vendor` feature must resolve exact
Runseal 0.17.0. The private `plumb::vendor::cloudflare` module has been removed;
call `runseal::tool::cloudflare` for atomic Cloudflare operations.

Loopback HTTP Cloudflare endpoints are admitted for bounded local contract
tests. Every non-loopback site endpoint still requires HTTPS.
