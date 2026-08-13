# Plumb v0.18.28

## One Cloudflare dialect

Plumb now consumes Runseal 0.17.0 for Cloudflare control-plane operations. Site
inspection uses its token verification, Worker service, and Worker domain
verbs. Retirement uses its account token factory and R2 bucket and custom
domain verbs.

The former `vendor::cloudflare` HTTP client is removed. Plumb retains site and
retirement orchestration, resource interpretation, destructive confirmations,
and guaranteed temporary-token revocation.

Minted token values remain inside Runseal's redacted, zeroizing secret result
until they are used for a bounded R2 operation and revoked. Status-sensitive
behavior such as an absent Worker or bucket uses the structured Cloudflare
fault rather than error text.
