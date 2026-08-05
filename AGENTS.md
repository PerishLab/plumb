# Agents

This repository is the workshop's living skeleton. `ectropy .` must print
`clean` before anything lands, and CI runs the complete repository guard in
explicit fail-fast order. Ectropy has one severity: every finding is an error.

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
- `docs/audit.md` — the opt-in Locus audit surface and its observation boundary.
- `apps/web` — the site at plumb.perish.uk, shipped by the deploy lane on
  dispatch through `plumb site deploy` and verified by readback
  (`docs/site.md`).
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
  activation, smoke, source binding, and packport topology checks. Those
  mechanisms do not live in product scripts.
- Every Plumb-owned artifact is byte-reproducible from the declared payload.
  Archive members are ordered and carry canonical timestamps, owners, and
  modes; MSVC binaries use the reproducible linker mode. Rebuilding one exact
  version from one commit must produce the same seal.
- Actions owns the reusable target matrix, artifact transport, credential
  binding, and release sequencing. Product repositories expose only
  `release-exact.yml` and `release-stable.yml` as thin callers.
- Every permanent release resolves canonical stable Plumb once and freezes that
  exact stable version across all jobs. The sole recovery exception is the
  typed `PerishLab/plumb` v0.18.14 self-hosting contract: source-built
  `v0.18.14` may generate only `v0.18.14-beta.1`, and that exact public beta may
  generate only stable `v0.18.14`. The command admits no product, authority,
  channel, version, workflow, branch, or generator selector; it records exact
  source/public provenance and leaves no alternate lane behind.
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
- Exact publication may bind any operator-selected branch ref; the called
  shared workflow freezes its direct event ref and commit once and every job
  checks out that commit. Product callers expose and forward no second source.
  `plumb release dispatch` is the generic Forgejo entrypoint and preserves
  arbitrary exact channel and branch selection.
  Stable alone must originate from `refs/heads/release/vX.Y.Z`. A stable
  release line is managed by `plumb stable prepare|pick|freeze|packport`,
  prepared by linear `cherry-pick -x`, frozen before publication, and remains
  independent from an unblocked `main`.
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
- Release identity lives in exact seals and the stable pointer; new releases do
  not create Git tags. Historical tags are retained as history, not consensus.
- After stable succeeds, a local operator packports the release line into
  `main` with a topology-preserving merge. The stable commit must become an
  ancestor of `main` before another stable line can activate. This settlement
  is independent from the Actions lane; exact publication and ordinary `main`
  work remain unblocked. The settled release branch remains permanently frozen
  as the stable version's source and audit boundary; the merge request never
  asks Forgejo to delete it.
- Every product repository uses the same Forgejo secret names:
  `RELEASE_PUBLISH_S3_*`, `RELEASE_ACTIVATE_S3_*`, and optional
  `RELEASE_REGISTRY_TOKEN`. Authority comes from `plumb.toml`, not a repository
  variable.

## Site

- A repository declares one site with `apps/*/wrangler.jsonc`; Plumb derives
  its package, assets, worker, routes, and fingerprint without a repository
  ship wrapper.
- `plumb site plan` is credential-free, `plumb site inspect` reads Cloudflare
  state, and `plumb site deploy` builds, uploads, reads binding, and proves the
  public edge serves the built fingerprint.
- Site authority enters only through `PLUMB_SITE_TOKEN`,
  `PLUMB_SITE_ACCOUNT`, and `PLUMB_SITE_DOMAIN`. The token never enters command
  arguments or logs.
- Deploy, bound, and reachable are separate outcomes. Binding is
  `yes|no|unknown`; `PLUMB_SITE_BLIND=true` can excuse failed reachability only
  when binding is positively known.

## Control-plane cold-start

Repository creation belongs to the Forgejo control plane. Bucket, domain,
token, and credential lifecycle belongs to the declared infrastructure
resource owner. Plumb neither creates those resources nor writes their
credentials into Forgejo; it consumes the already provisioned
`RELEASE_PUBLISH_S3_*` and `RELEASE_ACTIVATE_S3_*` authority at publication
time.

No cold-start wrapper or token factory belongs in this repository. A missing
authority blocks release setup at its owning control plane rather than
inviting Plumb to infer or repair external state.

Plumb itself needs one explicit genesis ceremony to publish and activate the
first release that contains this substrate. The ceremony runs the source-built
binary once with the same capsule protocol and separate credentials. No
bootstrap branch or alternate permanent workflow survives genesis.

Repository retirement remains a destructive control-plane responsibility.
Plumb exposes no generic retirement command, and this repository carries no
retirement wrapper or implementation. An operator must use the explicit
authority and procedure of the owning control plane rather than infer a
successor command.
