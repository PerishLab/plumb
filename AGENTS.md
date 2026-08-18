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
- `apps/web` — the site at plumb.perish.uk, declared as `[release.cfworker]` and projected by the rendered
  ship lane like any other medium: an exact channel stages a version behind its own preview URL and only
  stable reaches the domain, and the deploy lane that dispatched `plumb ship site deploy` is gone. It is the
  Svelte specimen the web shape checks, and the shape moved with it: a web app carries `@perish/design`,
  and views and components are `.svelte` files.
- `packages/*` — publishable specimens, when they earn their place.
- `Containerfile` and `charts/*` — the image and chart carriers the ship
  adaptors project onto.

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
- A seal projects the Git index, so a declared source seat holding an untracked
  leaf is out of true and `plumb document` proposes nothing for it: the seal
  would otherwise stand for a tree the seat no longer is, and the drift the
  next reader sees would be someone else's. Track the leaf or ignore it.
- Governed lanes are rendered, never written. `plumb lane` derives each one from the release
  declaration and the repository shape, and drift is byte comparison against a fresh render, not a
  recorded seal: a generated target needs no evidence it cannot already derive. Rendering decides
  which jobs exist, so a product declaring no binary gets a lane without build, seal, and smoke
  rather than an empty matrix, which Forgejo never creates and whose dependents block forever. Drift is noted, never out of
  true: a release refuses on a rendered lane that drifted and on the absence of the lane it must dispatch, while every other unrendered lane only draws the note, so adoption stays incremental.
  Rendering itself refuses a shape the forge cannot run — secrets under `workflow_call`, an empty matrix, an installed tool absent from the job path, a release following a push — because only the forge parses a lane body, so the check stands where the text is written.
- Numbers a mechanism must honour live in Plumb's rules directory, not in a constant someone edits by hand: the width an attachment may declare, and the forge image every rendered lane runs in. Widening a width is a change to Plumb, which is the point — a product cannot grant itself a shape by declaring one. Plumb also records the width it has actually released, and a declaration above that draws a note rather than a refusal, because the gap between permitted and exercised is exactly the evidence that a path has never run.
- A ship object is one attachment, not one package: every package an attachment declares carries one input hash, one version, and ships together, because a unit the language forced into several boxes is still one unit. A seal records that hash per object — the path, mode, and recorded object of every tracked leaf under the derived roots, plus the pinned toolchain, stamped with the version those inputs first appeared in — and `[release.depends]` adds only the edges no convention derives. It measures what Git recorded, never the bytes lying in the working seat, so a dirty runner cannot move the answer.
- A projection never skips. The input hash says what moved since the last stable; it does not decide what ships. Every declared object projects at the release version, because a published identity carries that version and not shipping one punches a hole in the sequence that no later release can fill.
- Document evidence values are excluded from the semantic `plumb.toml` source
  projection. No other source bytes, target bytes, or binding fields are
  excluded.
- Source code admits no comments or documentation comments. Unclear behavior
  is repaired through names, types, boundaries, and tests rather than prose in
  source.

## Release

- The root `plumb.toml` `[release]` table is the complete product-owned release
  declaration: product, authority, binaries, Rust targets, and typed product
  inputs such as a skill, Cargo attachment, or Debian payload. Platform keys,
  archive names, environment prefixes, and artifact metadata are derived.
- A release declares a binary shape, or one attachment, or both. A skill and a
  Debian payload ride the binary authority and cannot stand without it, while
  every other attachment names a registry that holds its own ledger, so a
  release may declare one and nothing else. The shape a product declares is the
  shape Plumb answers to; refusing one it defines refuses its own product.
- Plumb owns Cargo discovery and stamping, target builds, archives, skill and
  Debian assembly, manager generation, capsules, storage, verification,
  activation, smoke, source binding, and packport topology checks. Those
  mechanisms do not live in product scripts.
- `release` holds the truth cycle and `ship` holds every projection of it. A
  verb belongs to whichever object it acts on: `release` keeps `source`,
  `compile`, `packport`, `authority`, `activate`, and `inspect`, while
  `ship binary` takes `activate`, `assemble`, `build`, `dispatch`, `inspect`,
  `managers`, `matrix`, `publish`, `recovery`, `smoke`, and `verify`, `ship
  cargo` takes `publish` and `rehearse`, `ship oci` takes `build` and `publish`,
  `ship chart` takes `package` and `publish`, and `ship npm` takes `pack` and
  `publish`. Release activation advances only the consensus pointer and release
  inspection proves only pointer and seal identity. Binary activation shifts
  generated managers and binary inspection proves the projected artifacts and
  managers. A projection never keeps a compatibility alias for a verb that
  moved.
- The adaptor set is a closed enumeration in Plumb, one entry per medium the
  ecosystem publishes to: `binary`, `site`, `cargo`, `npm`, `oci`, and `chart`.
  All six are absorbed, because a medium with no name here is a medium every
  repository invents for itself. The enumeration landed before the projections
  did, which is why filling each one in was an addition rather than a break.
  Nothing enters it on speculation: an entry earns its place from a medium
  already published to, never from one that might be.
- A capsule covers what a capsule can cover, and the gate is keyed to the
  declaration rather than to the medium. A release that declares a binary shape
  compiles a capsule, so every publishing deed refuses one that seals another
  version, exactly as `ship binary publish` takes the capsule as its payload. A
  release that declares no binary shape compiles no capsule and holds no
  authority to keep one in, so demanding one would refuse a shape Plumb itself
  defines; those deeds answer to the declared projection surface instead. The
  deeds that only prepare -- `cargo rehearse`, `oci build`, `chart package`,
  `npm pack` -- stay outside the rule either way, because they mutate nothing
  beyond the working tree. Publication remains the one irreversible point of a
  release.
- That rule binds a release that carries a seal, and whether one does is a
  declaration rather than an environment. A release naming a product and an
  authority compiles a capsule and publishes it; a release declaring only
  attachments compiles none, because nothing produces one and no authority
  holds one, so its projections answer to the registries that receive them.
  Reading the gate from a release output path instead refused a shape Plumb
  itself defines, for a reason that had nothing to do with the deed asked for.
- `Containerfile`, `charts/plumb`, and `packages/plumb` are carriers. They exist
  so the image, chart, and module adaptors project a real medium rather than a
  described one, and each now fixes the shape a published artifact of its kind
  takes here: the module carries `build`, `test`, `typecheck` and `prepack` with
  every tool pinned through the workspace catalog, and the chart carries a
  minimal workload a cluster would admit. The content is deliberately small and
  deliberately real; each takes a responsibility of its own when one earns the
  seat.
- Every Plumb-owned artifact is byte-reproducible from the declared payload.
  Archive members are ordered and carry canonical timestamps, owners, and
  modes; MSVC binaries use the reproducible linker mode. Rebuilding one exact
  version from one commit must produce the same seal.
- This repository renders and carries its own release lanes and reaches no
  shared workflow. Actions still owns the reusable matrix, artifact transport,
  credential binding, and sequencing for repositories not yet rendered.
- Every permanent release resolves canonical stable Plumb once and freezes that
  exact stable version across all jobs. The sole exception is the typed
  `PerishLab/plumb` v0.26.0 bootstrap: stable `v0.26.0` may be generated only by
  the published exact beta `v0.26.0-beta.1` standing at the same commit. It
  exists because v0.26.0 removes the projection skip, and a release generated by
  the version that still holds the skip would publish only the media that moved.
  The contract admits no selector; it is a rendered install step bound to one
  branch and one published beta, it records that provenance in the seal, and both
  sides come out in the release that follows.
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
  The candidate is derived, never named: exactly one published exact seal must
  stand at the frozen commit, and `plumb stable freeze` refuses zero or many.
  Stable binaries are rebuilt with stable identity from that commit.
- Exact publication binds exactly `refs/tags/<exact-version>`; the called shared workflow freezes its direct event ref and commit once and every job checks out that commit. Product callers expose and forward no second source. `plumb ship binary dispatch` is the generic Forgejo entrypoint and takes only `--version`: it derives the channel, derives the ref that carries it, and sends no identity input at all. Stable alone must originate from `refs/heads/release/vX.Y.Z`. A stable release line is managed by `plumb stable prepare|pick|freeze|packport`, prepared by linear `cherry-pick -x`, frozen before publication, and remains independent from an unblocked `main`. Packport is the last step of a release, not optional tidying: `prepare` and `freeze` refuse while the last stable point is a commit `origin/main` does not hold, because a line opened over an unported one collides on the way back.
- A release run pins its verdict as it pins its generator: `plumb stable prepare` records the datum the line judges against under `.plumb`, the mechanism's own seat that source projection excludes as it excludes `docs/CHANGELOG`, written as TOML because a formatter reads a repository's JSON and machine-owned state is not a repository's to format, and on that line `plumb doctor` reads the recorded stable answers instead of asking a live registry. A line carrying no datum is out of true. The line is the one a run declares in `PLUMB_RELEASE_VERSION`, or failing that the checked-out branch, so CI needs no declaration and none leaks into everything else a job runs. The recording commit owns its seat and sweeps whatever an earlier Plumb left there, so a line migrated forward carries one datum and not two, and `freeze` accepts that commit without cherry-pick provenance because it proves itself: it touches only the seat, and what it leaves there decodes for the line it names. `main` still tracks live latest, because drift detection is its job and reproducibility is the line's.
- A watch ends only when the whole run graph has. A run that reports success while a job is still in flight, has laid out no job at all, or carries a job that failed has not succeeded, and `plumb ship binary dispatch --watch` says so rather than handing back the dispatch's own good news. A forge refusal always names the operation Plumb was performing when it arrived, and a stale head names the commit the pull actually stands at.
- The ref carries the release, so neither caller takes a version. Both stay
  dispatched by an operator, who selects the ref instead of typing an identity:
  `release-exact.yml` on the exact tag, `release-stable.yml` on the frozen
  release line. Both forward only guard evidence, never an identity. A
  release never follows from a push, so the anchor is chosen from refs that
  already exist and no lane starts by accident.
- The channel is read from the version, never named beside it. `plumb release
  channel` derives it and is the only place that rule lives: `X.Y.Z` is stable
  and `X.Y.Z-<channel>.N` names its own channel. An exact version therefore
  carries its channel into every job that resolves it.
- A branch is a line and a tag is a point. `release/vX.Y.Z` accumulates the
  picked commits and remains the permanent audit boundary; the tag records
  which commit was published. A tag is a declaration and a convenience, not
  evidence: it may move, while the published seal cannot, and the seal wins
  wherever the two disagree. Stable carries both a line and a point, and the
  line stays the audit boundary; the point exists so a declared release has one
  removable handle.
- `plumb stable freeze` stamps that point, and `plumb stable retract` removes
  it. Retraction reads the release authority first and continues only on a
  plain absence: a served seal refuses because the version projected something,
  and any other answer refuses because a destructive act never runs on a
  reading it cannot trust. It acts on the declaration alone and leaves the line
  standing, so what it removes is a name, never evidence.
- Exact seal creation is create-only and idempotent by content. Publish and
  stable activation use separate credentials and separate Plumb commands.
- `plumb release inspect` takes its exact or stable public URL from the release
  environment and proves release identity. `plumb ship binary inspect` reads
  the same surface and proves the binary projection.
- `plumb ship binary smoke` performs the shared cross-platform generated-manager
  install, exact `--version` probe, update, and uninstall cycle from the product
  declaration.
- Cargo rehearses the first package it will project, then publishes and reads
  back every package before preparing its dependent. A package that did not
  move keeps the release it last changed in, and every requirement on it names
  that one, so `plumb` and `plumb-macro` stay coupled exactly without
  republishing what stood still or demanding an unpublished dependency exist.
- Release identity lives in exact seals and the stable pointer; new releases do
  not create Git tags. Historical tags are retained as history, not consensus.
- After stable succeeds, a local operator packports the release line into
  `main` with a topology-preserving merge. The stable commit must become an
  ancestor of `main` before another stable line can activate. This settlement
  is independent from the Actions lane; exact publication and ordinary `main`
  work remain unblocked. The settled release branch remains permanently frozen
  as the stable version's source and audit boundary; the merge request never
  asks Forgejo to delete it.
- A registry projection takes its identity from the declaration and its secret
  from the environment: `account` on the image and chart attachments names the
  forge identity, and `PLUMB_RELEASE_REGISTRY_TOKEN` carries the credential. An
  account is a public name, so a lane that supplied it would hold a decision the
  product owns; a credential is not, so it never enters the manifest. An owner
  segment in an
  image or chart path names an organisation, which is not an account, so no
  login is derived from it. A module published under a prerelease version
  carries its channel as the distribution tag, because a registry that defaults
  a prerelease to latest would hand consumers an unreleased build.
- Every product repository uses the same Forgejo secret names:
  `RELEASE_PUBLISH_S3_*`, `RELEASE_ACTIVATE_S3_*`, and optional
  `RELEASE_REGISTRY_TOKEN`. Authority comes from `plumb.toml`, not a repository
  variable.

## Site

- A repository declares one site with `apps/*/wrangler.jsonc`; Plumb derives
  its package, assets, worker, routes, and fingerprint without a repository
  ship wrapper.
- `plumb ship site plan` is credential-free, `plumb ship site inspect` reads
  Cloudflare state, and `plumb ship site deploy` builds, uploads, reads binding,
  and proves the public edge serves the built fingerprint. The site is an
  adaptor rather than a command of its own, so nothing projects outside `ship`.
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
