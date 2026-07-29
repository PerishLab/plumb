# plumb v0.16.0

## Standing is queryable, not transcribed

The rule catalog compiled into the binary is now the sole source for rule
identity, standing, explanation, evidence, and ownership. `plumb rule list` and
`plumb rule show <id>` read it; `plumb doctor --json` reports whole-catalog
coverage separately from a repository's findings.

The skill's `Standing` section used to be three hand-maintained lists, which
made "a clause claimed as enforced that is not" a permanent hazard — the one
defect that document says it cannot afford. It now points at the catalog
instead. Namespaces, tags, owners, and standings are closed vocabularies, and an
unknown selector refuses rather than returning an empty result that could be
mistaken for knowledge.

`blind` is a finding grade, not a standing: it means a mechanized evaluator ran
and could not read the evidence it needed. A prose-only rule is not blind merely
because no evaluator exists. See `docs/rules.md`.

## The skeleton's own prose is under the checker

`ectropy.toml` now scans `skills/**/*.md` and treats `skills/*` as module roots,
so the brief plumb ships is held to the same syntax law as the code shipping it.

## `plumb changelog` no longer explains a refusal by guessing

Passing an empty or whitespace-only `--version` made the command answer "the
repository declares none, so pass --version" — a statement about the repository
it had never read. A blank flag is now an absent one, and the repository's own
declared version answers instead.

This surfaced while rolling the changelog gate out to the other six managers.
`stim` threads `inputs.version_override` straight into `RELEASE_VERSION`, and
that input is optional, so the flag arrives blank whenever an operator forgets
it. The refusal was correct; the reason given for it was invented.
