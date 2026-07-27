# Agents

This repository is the workshop's living skeleton. `negentropy --strict .` must
print `clean` before anything lands, and CI runs the same guard the pre-commit
hook runs.

## The relation

negentropy owns its blindspots: a construct it cannot parse is the checker's
debt, not the author's exception. plumb inherits that relation for SHAPES. If a
repository in this ecosystem has no shadow here, plumb owes the shape.
Divergence is the skeleton's debt.

Downstream repositories do not get scanned by plumb, and plumb does not know
they exist. The CLI travels to them: install it, run it in a repository, read
what it reports. Feedback comes home as issues on this repo. That is the hot
link, and it is the only one.

## Layout

- `crates/plumb` — the CLI, published as a binary through the release lanes.
- `apps/web` — the site at plumb.perish.uk.
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
  debt relation above does not hold for the one repo that declares it.

## Release

- `manage.sh` and `manage.ps1` are the public install/update/uninstall
  entrypoints.
- R2 metadata and immutable version assets are release truth. Beta advances
  from beta metadata; stable advances only when the Cargo workspace version is
  newer than stable metadata.
- Stable tags are created only after R2 publish, metadata verification, and
  manager smoke.
- Forgejo needs the `PLUMB_RELEASES_PUBLIC_URL` repository variable and the
  four `PLUMB_RELEASES_S3_*` repository secrets. Keep local source values in
  the ignored `.forgejo/release.env`, initialized from
  `.forgejo/release.env.example`.
