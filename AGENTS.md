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
- `crates/lib/src/forgejo` — Plumb orchestration and Git adaptors over
  Runseal's structured Forgejo operations. It owns no HTTP sender and reads no
  `tea.yml`; authority enters through `FORGEJO_URL` with
  `FORGEJO_TOKEN_FILE` or `FORGEJO_TOKEN`.
- `crates/cli/src/dispatch/site` and `retire` — Cloudflare interpretation and
  orchestration over Runseal's structured Cloudflare operations. Plumb owns no
  authenticated Cloudflare HTTP sender or raw route dialect.
- `apps/web` — the site at plumb.perish.uk, shipped by the deploy lane on
  dispatch through `plumb site deploy` and verified by readback.
- `packages/*` — publishable specimens, when they earn their place.

The layout is not invented; it is the union already demonstrated by codehull and
ensign: `crates` for rust members, `apps` for deployable applications,
`packages` for publishable node packages, `skills` for operating briefs, and
`docs/CHANGELOG` for immutable release history.

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

## Documents

- A Plumb release owns one closed document strategy set. Repositories select a
  strategy and exact source seats; they do not configure target paths, file
  lists, exclusions, or numeric limits.
- Every repository carrying `plumb.toml` declares `[[document]]`; the former
  standalone skill and generic affirmation schemas have no compatibility path.
- `agent` owns root `AGENTS.md`, `architecture` owns optional root
  `ARCHITECTURE.md`, `design` owns optional root `DESIGN.md`, and `brief` owns
  exactly `SKILL.md`, `PATHS.md`, and `SCENARIOS.md` under one named skill seat.
- Tracked Markdown outside those declared targets and `docs/CHANGELOG/**` is
  out of true. Product payloads that happen to be Markdown do not acquire a
  repository-document exception.
- Current projections bind canonical source seals and one target/topology seal
  in `plumb.toml`. Source, target, or topology drift is out of true until a
  human reads both sides and records the proposal from `plumb document`.
- Document evidence values are excluded from the semantic `plumb.toml` source
  projection. No other source bytes, target bytes, or binding fields are
  excluded.
- Source code admits no comments or documentation comments. Unclear behavior
  is repaired through names, types, boundaries, and tests rather than prose in
  source.

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
- `release` holds the truth cycle and `ship` holds every projection of it. A
  verb belongs to whichever object it acts on: `release` keeps `source`,
  `compile`, `packport`, and `authority`, while `ship binary` takes `build`,
  `assemble`, `matrix`, `managers`, `publish`, `smoke`, `verify`, `dispatch`,
  and `recovery`. `activate` and `inspect` still span both sides, and
  `registry` still stands apart until a Cargo adaptor claims it; both are
  transitional, not settled. A projection never keeps a compatibility alias for
  a verb that moved.
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
  Plumb compiler before the first irreversible action. Exact release compile
  binds the previous public stable commit to the frozen candidate, measures
  textual churn plus changed paths, checks each language pair against its
  diff-derived budget, and retains the proof in the release seal.
- Existing changelog versions are frozen history. The current release may add
  its own language leaves and artifacts but cannot mutate an earlier version
  directory. Candidate language leaves do not scale their own budget; opaque
  artifacts and migration scripts remain in the measured diff.
- An optional `docs/CHANGELOG/v<base-version>/artifacts/` contributes its flat
  regular-file set to exact prerelease and stable seals. Plumb preserves bytes
  under filenames and owns only generic safety and collision refusal; product
  semantics do not enter the release model.
- Stable is `X.Y.Z`. Every non-stable release is
  `X.Y.Z-<channel>.N`. Stable promotion embeds the complete exact candidate
  seal and its digest, and requires the same product, base version, and commit.
  Stable binaries are rebuilt with stable identity from that commit.
- Exact publication binds exactly `refs/tags/<exact-version>`; the called
  shared workflow freezes its direct event ref and commit once and every job
  checks out that commit. Product callers expose and forward no second source.
  `plumb ship binary dispatch` is the generic Forgejo entrypoint.
  Stable alone must originate from `refs/heads/release/vX.Y.Z`. A stable
  release line is managed by `plumb stable prepare|pick|freeze|packport`,
  prepared by linear `cherry-pick -x`, frozen before publication, and remains
  independent from an unblocked `main`.
- The ref carries the release, so neither caller takes a version. Both stay
  dispatched by an operator, who selects the ref instead of typing an identity:
  `release-exact.yml` on the exact tag, `release-stable.yml` on the frozen
  release line. Both forward only promotion selection and guard evidence. A
  release never follows from a push, so the anchor is chosen from refs that
  already exist and no lane starts by accident.
- The channel is read from the version, never named beside it. `plumb release
  channel` derives it and is the only place that rule lives: `X.Y.Z` is stable
  and `X.Y.Z-<channel>.N` names its own channel. An exact version therefore
  carries its channel into every job that resolves it.
- A branch is a line and a tag is a point. `release/vX.Y.Z` accumulates the
  picked commits and remains the permanent audit boundary; the tag records
  which commit was published. An exact tag is a declaration and a convenience,
  not evidence: it may move, while the published seal cannot, and the seal
  wins wherever the two disagree.
- Exact seal creation is create-only and idempotent by content. Publish and
  stable activation use separate credentials and separate Plumb commands.
- `plumb release inspect` takes its exact or stable public URL from the release
  environment and verifies the whole public surface.
- `plumb ship binary smoke` performs the shared cross-platform generated-manager
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
  arguments or logs; it reaches Cloudflare through Runseal's in-process
  dialect.
- Deploy, bound, and reachable are separate outcomes. Binding is
  `yes|no|unknown`; `PLUMB_SITE_BLIND=true` can excuse failed reachability only
  when binding is positively known.

## Cold-start

Plumb owns the complete life of a governed product's release surface: land,
guard, release, publish, stable, site, and retire. Retirement is the mirror of
release, not a foreign errand, and it lives here because everything it destroys
is something Plumb declared, published, or protected.

Plumb does not cold-start. It creates no repository, no bucket, and no domain
that does not already exist, and it never infers or repairs missing external
state. A missing authority blocks at its owning control plane. This is an
implementation constraint, not a permission one: no provisioning call exists in
this codebase, and adding one is the change that must be refused, because the
authority Plumb already holds is sufficient to provision if such a call were
ever written.

Plumb may derive ephemeral authority to act on what it governs. `plumb retire`
cuts short-lived scoped tokens from the declared factory in `PLUMB_RETIRE_*`,
bounds them with an expiry, and revokes them in an arm that runs whether the
sweep succeeded or failed. A derived token never enters command arguments or
logs, and no derived token outlives the command that cut it. The factory
credential itself is provisioned elsewhere; Plumb consumes it and never mints
one.

Plumb itself needs one explicit genesis ceremony to publish and activate the
first release that contains this substrate. The ceremony runs the source-built
binary once with the same capsule protocol and separate credentials. No
bootstrap branch or alternate permanent workflow survives genesis.

Retirement destroys and cannot be undone, so it is bounded by declaration
rather than by argument. A product that can be retired names its bucket and
zone under `[release.retire]`; one that declares nothing cannot be retired at
all. The dry run is the default and reads no credentials. `--execute` acts only
when `--confirm-repo`, `--confirm-bucket`, and `--confirm-domain` each equal
their target verbatim, and the destructive order is fixed: inventory, archive
and purge credentials, revoke the writer, detach the domain, empty and delete
the bucket, delete the repository, remove the local escrow.
