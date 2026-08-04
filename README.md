# plumb

A plumb line for repositories.

This is the workshop's living skeleton: the operational envelope is worked out
here, published as a CLI, and kept honest by being run rather than described.
Every repository in the ecosystem should find its shadow here — and when one
cannot, that is plumb's responsibility, not the repository's exception.

- `crates/cli` — the CLI. Hold a repo against the skeleton and report where it
  hangs untrue. Checking lands before scaffolding.
- `crates/lib` and `crates/macro` — the shared substrate and derive.
- `apps/web` — plumb.perish.uk, answering why, what and how. Built and
  prerendered by the guard, shipped by its own lane, and read back after every
  deploy against the fingerprint it just built — so the skeleton's web path is
  exercised end to end rather than claimed.

A template that is only copied rots. This one is run.

`plumb doctor --json [ROOT]` emits the same verdict as the default human
report with a positive shape snapshot, stable finding codes, evidence, scope,
mechanized standing, owner, tags, and whole-catalog coverage. `ok` follows the
command exit status; `clean` is true only when no out-of-true, unknown, or blind
finding remains.

`plumb rule list` queries the complete version-matched law catalog, including
prose-only laws that do not affect the doctor verdict. `plumb rule show RULE_ID`
explains one rule's law, standing, evidence, and owner. Namespace, tag,
standing, and owner selectors are closed typed vocabularies; unknown values
refuse. See `docs/rules.md`.

First-party dependencies in the `@perish` JSR scope and `perish` Cargo
registry track live stable latest. Deno declarations carry no version
requirement; Cargo keeps its native requirement spelling. In both ecosystems,
the exact lock resolution must equal the latest stable registry version.
`plumb doctor` reads registry metadata but never edits a manifest or lock.
Missing manifest, lock, or registry evidence is blind and blocks the command;
deliberate movement remains a standard Deno or Cargo operation.

## Change boundaries

`plumb precommit` proves that one committed Git delta stays inside an explicit
set of repository-relative write prefixes:

```sh
plumb precommit . \
  --base <exact-commit-oid> \
  --head <exact-commit-oid> \
  --write crates/lib \
  --write crates/cli
```

The repository HEAD must equal `--head`, the worktree must be clean including
untracked files, and `--base` must be its ancestor. Add, delete, type change,
and gitlink paths enter the delta; rename and copy enter both their old and new
paths. `.` explicitly claims the whole repository. Malformed boundaries,
symbolic revisions, non-UTF-8 paths, and unread Git evidence refuse.

`--json` emits `plumb.precommit/v1` with resolved OIDs and normalized `write`,
`changed`, and `outside` sets. This is an operation-local evaluator, not a Git
hook or a Doctor rule. A coordinator such as Concord owns the claim lifecycle
and calls the same library API; Plumb neither reads that control plane nor
assigns claims.

## Domain vocabulary transitions

Doctor also compares a Plumb-bundled transitional domain dictionary with the
active Git closure. Configuration stores only canonical `p64-v1` values: `~`
plus unpadded Base64URL over a lowercase ASCII atom. Decode then encode must be
byte-identical; malformed, noncanonical, or duplicate decoded terms refuse.

Each decoded term is matched as an ASCII case-insensitive byte substring over
tracked path and current worktree bytes. Symlinks contribute their link-target
bytes without being followed; gitlinks contribute only their tracked path.
`docs/CHANGELOG/**` is the fixed historical exemption. Repositories cannot add
dictionary entries or exclusions. A hit is out of true and unread closure
evidence is blind.

Retirement is deliberately temporary. After the domain either removes a term
or restores it to live use, its encoded dictionary entry is deleted. The first
release of this mechanism carries an empty retired set, so it adds the proof
surface without beginning a vocabulary transition.

## Install the CLI

```sh
curl -fsSL https://releases.plumb.perish.uk/manage.sh | sh
```

Select an exact stable release for a deliberate rollback:

```sh
curl -fsSL https://releases.plumb.perish.uk/manage.sh | sh -s -- install --version v0.13.1
```

Windows uses `https://releases.plumb.perish.uk/manage.ps1`. Both managers
install versioned binaries under the user's local data directory and expose
`plumb` from the user's local bin directory.

Non-stable releases are disposable validation candidates. Name the channel and
exact version and give both seats an explicit isolation root:

```sh
isolation=$(mktemp -d)
curl -fsSL https://releases.plumb.perish.uk/manage.sh -o "$isolation/manage.sh"
sh "$isolation/manage.sh" install \
  --channel beta \
  --version v0.16.0-beta.1 \
  --install-root "$isolation/install" \
  --bin-dir "$isolation/bin"
"$isolation/bin/plumb" doctor .
```

The root manager is stable-owned but can express an exact isolated install for
any channel. The exact release seal also records content-addressed generated
managers for reproducible validation.

Releases publish one identity to object storage and the private Cargo registry.
Every channel first creates
`v1/releases/<channel>/<exact-version>/seal.json`; non-stable stops there.
Stable promotion proves one exact candidate from the same commit, then moves
`v1/channels/stable.json` and the root managers. Release identity is carried by
exact seals and the stable pointer rather than new Git tags. Public inspection
and generated manager smoke are reusable Plumb operations rather than a third
workflow lane. Stable is sourced only from `release/vX.Y.Z`; after activation a
local operator merges that line into `main` and proves the stable commit is now
an ancestor before the next stable activation. The settled release branch stays
permanently frozen as the version's source and audit boundary.

Generic release operation lives in the CLI rather than repository wrappers:

```sh
plumb release dispatch --channel beta --version vX.Y.Z-beta.N --ref <branch>
plumb stable prepare --version vX.Y.Z
plumb stable pick --version vX.Y.Z --commit <full-sha>
plumb stable freeze --version vX.Y.Z
plumb release dispatch --channel stable --version vX.Y.Z \
  --promotion-channel beta --promotion-version vX.Y.Z-beta.N
plumb stable packport --version vX.Y.Z
```

Exact dispatch keeps channel and branch selection open. Only stable derives and
walls `release/vX.Y.Z`; packport retains that frozen branch after a
topology-preserving merge.

Site operation is also CLI-owned:

```sh
plumb site plan
plumb site inspect
plumb site deploy
```

Plumb derives the one `apps/*/wrangler.jsonc` application and keeps upload,
Cloudflare binding, and public fingerprint readback as three separate results.
Product repositories carry only the dispatch lane and declaration, not a ship
wrapper.

## Paired Web/API dispatch

When a repository holds both a React/Vite application at `apps/web` and an
executable `api` crate at `crates/api`, `plumb doctor` checks the combination as
one product shape:

- `sidecar.toml` leases both development ports, waits for API endpoint
  readiness at `/api/health`, and passes that endpoint to Web as `API_URL`;
- API consumes the leased port through `SIDECAR_PORT` or an explicit
  `"{port}"` environment mapping, accepts the sidecar stamp, emits its endpoint
  as API readiness, and mounts its whole public surface under `/api`;
- Web consumes `@perish/react-components`, activates
  `@perish/vite-plugin-design`, loads the in-memory views manifest, and leaves
  sidecar port and API proxy handling to the plugin;
- the guard runs a real Web build, so the design compiler remains the
  authoritative check for `views/**/*.tsx`;
- production has separate API and Web image seats, separate chart workloads,
  Web-targeted ingress, and one Cargo/chart version train.

The rule is about the dispatch relationship. Product routes, image names, and
application-specific infrastructure remain the product's own shape.

## Ectropy policy

Plumb owns the repository-shaped `ectropy.toml` template while ectropy remains
the independent AST executor. `plumb doctor` compares scan sets, module roots,
all limits, comment and word settings, test and environment territory, and
required syntax bans semantically; ordering and formatting do not matter.
When `skills/` exists, its Markdown briefs and per-skill roots are part of that
canonical policy.
Repository-specific syntax boundaries, Ectropy vocabulary, and additional
non-test grants remain local allowances rather than becoming skeleton defaults.
The Plumb-bundled retired domain dictionary is a separate workshop transition
mechanism and is not configurable by repositories.

`plumb policy` prints the reconciled policy; `plumb policy --write` updates the
repo-root `ectropy.toml` atomically. The reconciliation owns only the canonical
shape-derived sections and preserves those local allowances.

Application Web code keeps lowercase `views/**/*.tsx`,
`lib/components/**/*.tsx`, and lowercase `lib/hooks/**/use-*.ts` roles.
`lib/components` is style-free through ectropy's path-scoped style ban.
Package-specific styling systems such as StyleX are dependency policy instead:
Plumb rejects their manifests through its styling blacklist, and ectropy never
guesses package identity from names such as `stylex` or `css`.

The reusable component seat is the independent design system.
`packages/components` is therefore invalid in the general skeleton; Plumb does
not encode any repository identity or special Design layout to make that rule.

Runseal profiles provide explicit environment, argument, and symlink injection.
Generic lifecycle wrappers and repository-owned Git hooks are not repository
facts: guard runs directly in the canonical workflow, and landing uses
`plumb land`. Product-specific lifecycle code remains owned and tested by the
product that needs it.
