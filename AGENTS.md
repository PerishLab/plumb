# Agents

This repository is the workshop's living skeleton. `ectropy .` must print
`clean` before anything lands, and CI runs the same guard the pre-commit hook
runs. Ectropy has one severity: every finding is an error.

## The relation

ectropy owns its blindspots: a construct it cannot parse is the checker's
responsibility, not the author's exception. plumb inherits that relation for
SHAPES. If a repository in this ecosystem has no shadow here, plumb owes the
shape. Divergence is an error in the skeleton.

Downstream repositories do not get scanned by plumb, and plumb does not know
they exist. The CLI travels to them: install it, run it in a repository, read
what it reports. Feedback comes home as issues on this repo. That is the hot
link, and it is the only one.

## Layout

- `crates/cli` — the CLI, published as a binary through the release lanes.
- `crates/lib` and `crates/macro` — the substrate and derive consumed by the
  ecosystem.
- `apps/web` — the site at plumb.perish.uk, shipped by the deploy lane on
  dispatch and verified by readback (`docs/site.md`).
- `packages/*` — publishable specimens, when they earn their place.

The layout is not invented; it is the union already demonstrated by codehull and
ensign: `crates` for rust members, `apps` for deployable applications,
`packages` for publishable node packages, `docs` for prose.

## Boundaries

- LIVING SPECIMENS ONLY. An archetype held here must be really built and really
  published, or it is not exercised and will rot exactly like boilerplate. What
  is described but not run must say so.
- CHECKING BEFORE SCAFFOLDING. `check` ships before `new`. A generator encodes
  guesses; a diff harvests facts, and the ecosystem already holds eleven repos
  of facts.
- EVERY ELEMENT STAYS REMOVABLE. Encoding combinations is the point — many
  choices here are only defensible together, not alone — but no element may
  become unremovable, or its justification decays from finding to story.
- plumb MUST PASS ITSELF. Running the CLI here has to come back clean, or the
  relation above does not hold for the one repo that declares it.

## Release

- The root `plumb.toml` `[release]` table is the complete product-owned binary
  release declaration: product, authority, binaries, Rust targets, and typed
  product inputs such as a skill, Cargo attachment, or Debian payload.
  Platform keys, archive names, environment prefixes, and artifact metadata
  are derived.
- Plumb owns Cargo discovery and stamping, target builds, archives, skill and
  Debian assembly, manager generation, capsules, storage, verification,
  activation, smoke, and stable tags. Those mechanisms do not live in product
  scripts.
- Every Plumb-owned artifact is byte-reproducible from the declared payload.
  Archive members are ordered and carry canonical timestamps, owners, and
  modes; MSVC binaries use the reproducible linker mode. Rebuilding one exact
  version from one commit must produce the same seal.
- Actions owns the reusable target matrix, artifact transport, credential
  binding, and release sequencing. Product repositories expose only
  `release-exact.yml` and `release-stable.yml` as thin callers.
- Every permanent release resolves canonical stable Plumb once and freezes that
  exact stable version across all jobs. The source-built Plumb genesis ceremony
  is one-shot and leaves no alternate lane behind.
- Exact releases live at `v1/releases/<channel>/<exact-version>/seal.json`.
  Non-stable has no moving pointer and no activation operation. Consumers name
  both channel and exact version.
- Stable alone owns `v1/channels/stable.json`, `/manage.sh`, and
  `/manage.ps1`. Stable activation conditionally updates the generated root
  managers, then commits consensus by compare-and-swap of the stable pointer.
  The pointer is the only moving truth.
- The default install and bin seats admit only stable from the canonical
  release authority. Every other channel requires an exact version plus
  explicit install and bin paths disjoint from the defaults. Default stable
  mutation holds one lock, stages before switching, proves ownership on every
  destructive path, and refuses an implicit rollback.
- A stable capsule refuses to compile without
  `docs/CHANGELOG/v<version>/{en,zh}/{INDEX.md,MIGRATION.md}`, enforced by the
  Plumb compiler before the first irreversible action.
  `plumb doctor` does not check this: a changelog is owed by a release, not by a
  working tree. See `docs/changelog.md`.
- Stable is `X.Y.Z`. Every non-stable release is
  `X.Y.Z-<channel>.N`. Stable promotion embeds the complete exact candidate
  seal and its digest, and requires the same product, base version, and commit.
  Stable binaries are rebuilt with stable identity from that commit.
- Exact seal creation is create-only and idempotent by content. Publish and
  stable activation use separate credentials and separate Plumb commands.
- `plumb release inspect` takes its exact or stable public URL from the release
  environment and verifies the whole public surface.
- `plumb release smoke` performs the shared cross-platform generated-manager
  install, exact `--version` probe, update, and uninstall cycle from the product
  declaration.
- Cargo rehearses the head of each ordered attachment before publication, then
  publishes and reads back every package before preparing its dependent.
  Plumb therefore publishes `plumb-macro` before `plumb` and locks their
  coupled versions exactly without requiring an unpublished dependency to
  exist during rehearsal.
- Non-stable releases do not create Git tags.
- Stable tags are created only after exact publish, stable activation, and
  manager smoke.
- Every product repository uses the same Forgejo secret names:
  `RELEASE_PUBLISH_S3_*`, `RELEASE_ACTIVATE_S3_*`, and optional
  `RELEASE_REGISTRY_TOKEN`. Authority comes from `plumb.toml`, not a repository
  variable.

## Ecosystem release cold-start

`runseal :cold-start project` creates or verifies one exact empty Forgejo
repository before its integration checkout and first task member exist. It is
credential-free in dry-run mode and refuses a nonempty, archived, differently
described, or differently visible existing repository.

`runseal :cold-start release` is the explicit low-frequency control-plane
entrypoint for a new R2-backed Forgejo release chain. The permanent Cloudflare
authority lives only in the main checkout at
`.local/secrets/cloudflare-token-factory.env`, with mode `0600`, as the
account-owned token `super:perish.code`. It has only
`Account API Tokens Write`.

The naming and authority split is fixed:

- `super:perish.code` — permanent token factory; never enters CI or performs
  business-resource operations directly.
- `tmp:<bucket>` — 15-minute account-scoped R2 administration token, revoked
  on every completion path.
- `publish:<bucket>` — permanent exact-object publication capability scoped to
  one bucket.
- `activate:<bucket>` — permanent stable-consensus activation capability
  scoped to one bucket.

The wrapper creates or verifies the bucket and TLS 1.2 custom domain, derives
two independent S3 credential sets, verifies each, stores the local escrow at
`.local/secrets/releases/<product>.env`, and syncs only the derived
bucket-scoped values into Forgejo. Permission-group IDs are discovered by
exact name and resource scope at runtime. A persistent capability without its
matching local escrow is a fail-closed recovery case because its secret cannot
be reconstructed.

Plumb itself needs one explicit genesis ceremony to publish and activate the
first release that contains this substrate. The ceremony runs the source-built
binary once with the same capsule protocol and separate credentials. No
bootstrap branch or alternate permanent workflow survives genesis.

`runseal :retire` is the symmetric destructive control-plane entrypoint,
implemented and tested by `@perish/sealkit/retire`. It defaults to a
credential-free dry run and requires `--execute` plus exact repo, bucket, and
domain confirmations. Plumb keeps only the thin wrapper; `.runseal` owns no
retirement implementation or tests.
