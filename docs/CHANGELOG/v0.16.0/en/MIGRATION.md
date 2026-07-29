# Migrating to v0.16.0

## If you read standing out of the skill

Stop transcribing it. `plumb rule list --standing mechanized` and
`--standing prose-only` answer from the catalog compiled into the matching
binary. Copying a list into prose is how a claim drifts from what the checker
actually does.

Note what `clean` does and does not mean: it says this repository emitted no
finding, not that every catalogued law is mechanized. `plumb doctor --json`
carries whole-catalog coverage separately.

## If you keep prose under `skills/`

It is now scanned by ectropy in this repository. Adopting the same in another
repository means adding `skills/**/*.md` to `scan.include` and `skills/*` to
`module.roots`; nothing forces it.

## If a lane passes `--version` that can be empty

Nothing to do. A blank flag now falls back to the repository's declared version
instead of refusing with a wrong explanation. A lane passing a real version is
unaffected, and a repository declaring no version at all still refuses — with an
accurate reason this time.

## If you have web hooks named `use-*.ts`

Rename them for their subject — `use-health.ts` becomes `health.ts` — and drop
any ectropy boundary you added to waive the word law for that directory. The
exported hook keeps its React spelling: `useHealth`.

Nothing forces the rename; the check only refuses non-`.ts` or uppercase files
now. But a repository that keeps the old spelling still needs the boundary,
which is the waiver this release makes unnecessary.
