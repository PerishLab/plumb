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
