# Agents

This repository is a constitution-era web workspace. `negentropy --strict .`
must print `clean` before anything lands.

## Stack

node 24, pnpm workspaces, vite, react, typescript, vitest, biome, sass. Every
version is pinned exact in the `catalog` of `pnpm-workspace.yaml`; packages
reference `catalog:` only.

## Layout

- `apps/react/` — the vite app `@open-web/react`: `src/{lib,views,components}`,
  `tests/`. Never name a workspace package bare `react`.
- `packages/components/` — the component library `@open-web/components`:
  `src/` with `lib.ts` as the export surface, `tests/`.

## Territory

- Style: only `packages/components` owns style declarations. A component
  imports its own `.scss` sibling. Apps contain zero `.scss` files.
- Tests: vitest over `tests/**/*.test.ts` per package; the constitution grants
  test syntax only on those paths.

## Laws in practice

- Single word: every file, directory, and declared name in scanned sources is
  one vocabulary atom (`vocabulary.toml` registers exceptions; keep it empty).
- Block depth <= 4; markup depth <= 8; path depth <= 4 from the module roots
  (`apps/*/src`, `apps/*/tests`, `packages/*/src`, `packages/*/tests`).
- No comments in scanned sources or configs.
- Style declarations only under `packages/components/**`; apps consume
  component classNames and declare nothing.

## Closed blind spot

The tsx and scss grammars landed in negentropy v0.2.0-beta.1; `negentropy.toml`
scans `.ts`, `.tsx`, and `.scss` under `apps/` and `packages/` plus
`docs/**/*.md`. Markup nesting is its own law: depth <= 8 per element tree.
Keep `.tsx` surfaces thin anyway — components and views only.

## Baked data

`runseal :bake` runs `negentropy --vocabulary .` across the four sibling repos
(`negentropy`, `runseal`, `sidecar`, `open-web`) and writes
`apps/react/src/data/vocabulary.json`. The JSON is committed so the site works
without re-baking; re-run after vocabulary-visible changes.

## Shipping

`pnpm ship` (`runseal :ship`) lays the built site onto the R2 bucket:

1. `pnpm --filter @open-web/react build`
2. snapshot: `dist/index.html` is copied to `<route>/index.html` for every
   path in `apps/react/src/lib/routes.ts`, so deep links resolve on R2 static
   hosting without a worker; the route table is parsed, never hardcoded
3. `aws s3 sync` then `aws s3 cp` against the R2 S3 endpoint — hashed assets
   ship `immutable`, html shells ship `max-age=60, must-revalidate`
4. verify: `/` and the first deep route must answer 200 on the public domain

Flags: `--dry-run` prints the full plan (build, snapshot list, sync commands
with redacted credentials, verify URLs) and executes nothing; `--check`
probes the zone DNS record for the site domain and the R2 bucket through
`runseal @tool cloudflare` (raw `api request` where typed commands lack
coverage) and degrades gracefully while secrets are unfilled.

Secrets live under `.local/secrets/` (gitignored), one `KEY=value` per line,
`#` comments allowed. Fill every value before a real `:ship`; the wrapper
fails cleanly naming exactly the unfilled keys.

- `ship.env` — read by `:ship` at start:
  - `OPENWEB_SITE_S3_AK` — R2 S3 access key id for the site bucket
  - `OPENWEB_SITE_S3_SK` — R2 S3 secret access key
  - `OPENWEB_SITE_S3_BUCKET` — bucket name that holds the built site
  - `OPENWEB_SITE_S3_URL` — S3 endpoint, `https://<account-id>.r2.cloudflarestorage.com`
  - `OPENWEB_SITE_DOMAIN` — public site host, no scheme
- `cloudflare.env` — read by `runseal @tool cloudflare` during `--check`:
  - `CLOUDFLARE_ACCOUNT_ID` — account identifier
  - `CLOUDFLARE_API_TOKEN` — token with zone read + R2 read scope
  - `CLOUDFLARE_ZONE_NAME` — zone carrying the site domain

## Operating

- Never commit on `main`; the pre-commit hook refuses it. Branch, then commit.
- `runseal :init` validates tools and entrypoints and installs git hooks.
- `runseal :guard` is the full local gate: negentropy pin check, `pnpm biome ci .`,
  `pnpm -r exec tsc --noEmit`, `pnpm -r test`, `deno fmt --check .runseal`,
  `deno check` over the wrappers, `negentropy --strict .`.
- `runseal :land` squash-merges the topic branch on Forgejo after guard passes.
- The negentropy pin is `.runseal/negentropy.version` (stable channel); CI
  installs exactly that version and runs `negentropy --strict .`.
- `sidecar.toml` defines the react dev server as the app target:
  `pnpm --filter @open-web/react dev`.
