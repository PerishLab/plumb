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

For Sealkit consumers, the positive shape snapshot includes both the import
requirement and the exact resolution read from `.runseal/deno.lock`. Plumb
judges those bytes offline. It never resolves a registry version or edits the
lock; deliberate movement remains a standard Deno operation.

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
`v1/channels/stable.json` and the root managers. The stable tag is created only
after public verification and install smoke. Public inspection and generated
manager smoke are reusable Plumb operations rather than a third workflow lane.

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
Repository-specific boundaries, vocabulary, and additional non-test grants
remain local allowances rather than becoming skeleton defaults.

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

`.runseal` is adapter territory. Test files are forbidden there; logic that
needs dedicated tests belongs in `@perish/sealkit` and wrappers remain thin.
