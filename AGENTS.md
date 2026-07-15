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

## Perceiving

`runseal :playwright` is the project-specialized path of playwright-cli
(`@playwright/cli`, catalog-pinned): it binds the routes table, the sidecar
`health_url`, and the artifact home `.local/playwright/` into an `openweb`
browser session, and passes everything else through untouched.

- `shot <route...>` / `shot --all` — screenshots to `.local/playwright/shots/<route>.png`
- `text <route...>` — accessibility snapshots to `.local/playwright/snaps/<route>.yml`
- `console [level]` / `status` / `close` — live-page console, app+session state, teardown
- `runseal :playwright -- <raw args>` — the full playwright-cli surface inside the session

The app process belongs to sidecar (`sidecar.toml`); the wrapper only probes
`health_url` and fails with a pointer when nothing serves. One-time setup:
`pnpm exec playwright-cli install-browser chromium` (headless shell, ~115 MiB).

## Shipping

`runseal :ship` deploys the site as Cloudflare Workers Static
Assets — SPA routing is the worker's home turf; R2 keeps the release-artifact
role only:

1. `pnpm --filter @open-web/react build`
2. `pnpm exec wrangler deploy --domain <OPENWEB_SITE_DOMAIN>` from
   `apps/react/`; `wrangler.jsonc` names the worker `openweb`, serves
   `./dist`, and sets `not_found_handling: single-page-application` so deep
   links resolve without route snapshots; the `--domain` flag attaches the
   custom domain and its DNS record at deploy time
3. verify: `/` and the first deep route from `apps/react/src/lib/routes.ts`
   must answer 200 on the public domain

Flags: `--dry-run` prints the plan with redacted credentials, then runs
`wrangler deploy --dry-run` (credential-free) when `dist/` exists; `--check`
verifies the token, the zone, the site DNS record, and worker existence via
`runseal @tool cloudflare` (raw `api request` where typed commands lack
coverage), degrading gracefully while secrets are unfilled or before the
first deploy.

Secrets live under `.local/secrets/` (gitignored), one `KEY=value` per line,
`#` comments allowed. Three values total; the wrapper fails cleanly naming
exactly the unfilled keys.

- `ship.env` — read by `:ship` at start:
  - `OPENWEB_SITE_DOMAIN` — public site host, no scheme
- `cloudflare.env` — read by `:ship`, `--check`, and `runseal @tool cloudflare`:
  - `CLOUDFLARE_ACCOUNT_ID` — account identifier
  - `CLOUDFLARE_API_TOKEN` — token with Workers Scripts edit + Zone DNS edit
    (plus zone read) scope
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
