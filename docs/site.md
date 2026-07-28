# Site

A repository that carries `apps/web` ships a site the same way it ships a binary: one operator
entry, one lane, credentials the lane holds and the operator does not, and a readback that proves
what answers is what was built.

## The lane

`deploy.yml` runs on dispatch alone. Landing changes main; shipping the site is a separate,
deliberate act, so a documentation change never moves what the world sees.

The lane calls the repository's own ship wrapper — the same entry an operator runs locally. One
implementation serves both callers, which is why the wrapper takes its credentials through the env
door: the operator's come from `.local/secrets`, the lane's from repository secrets and variables,
and neither path is a second implementation.

## Credentials

The key is minted for this purpose and nothing else: Workers write on the account, and zone read,
DNS write, and Workers routes write on **the one zone that carries the domain**. It does not expire,
which is only defensible because it is narrow and revocable on its own — so its identity is recorded
where the lane is described, and its value lives in the repository secret and nowhere else.

Minting through an account-owned factory has a shape worth writing down: a zone resource must be
nested under the account resource, or named by zone id directly. A flat
`com.cloudflare.api.account.zone.*` is refused with code 1001.

Secrets carry what must not be read back; variables carry what merely must be correct. The domain
and the account identifier are variables. Only the key is a secret.

## The artifact declares itself

The build stamps the commit and version into the health document the design plugin already emits,
and the prerender writes every published route into the sitemap. Both are artifact-side: they say
what this build _is_ and what it _produced_. The shipper reads them and never reads application
source — a shipper that scrapes source is coupled to a structure the app is free to move, and it
will break silently the first time the app moves it.

Verification compares the fingerprinted asset in the served page against the one in the built page.
A status code proves something answered; only the fingerprint proves the answer is this build.

## Three states, reported separately

A deploy has three outcomes and they are not one outcome:

- **deployed** — the upload succeeded.
- **bound** — the platform reports the domain attached to this worker. Reading this needs a
  credential that can, so the finding is `yes`, `no`, or `unknown`, and the three are not two. A
  denied request and an empty result set arrive looking alike; treating them alike reports a bound
  domain as unbound and fails a healthy deploy. Only `no` is a refusal.
- **reachable** — the edge serves this build to a client. Only a readback proves it, and only from a
  vantage that can see the public edge.

Collapsing these is how a lane comes to report success over a site that does not answer. When the
readback fails the lane fails, and the escape is explicit: `PLUMB_SITE_BLIND=1` declares a vantage
that cannot see the edge, and the lane then says plainly that it did not prove the site answers. A
lane that cannot prove liveness must say so rather than imply it.

The escape has a floor. When binding is `unknown` the readback is the only evidence left, so a blind
vantage cannot excuse it: a ship that could neither ask the control plane nor look at the edge has
proved nothing, and saying so is the only honest outcome.

_Incident:_ the first binding of a new domain went through the API cleanly — enabled, certificate
active, DNS proxied, deployment current, every field identical to a sibling that worked — and the
edge still routed to the placeholder address, answering 522. A second, otherwise identical ship
cleared it. A stuck binding looks exactly like a healthy one from the control plane, so the readback
is not a formality.

## Observation

No single vantage is trustworthy. During that incident a local resolver mapped the domain into a
proxy that failed only for it, and a third-party fetcher returned errors for a site that was
answering at the same moment. The fingerprint is what settles it: compare what came back to what was
built, and prefer the vantage you can explain.
