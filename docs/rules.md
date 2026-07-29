# Rule catalog

The rule catalog compiled into `plumb` is the complete, version-matched index
of laws the skeleton recognizes. It is the sole source for rule identity,
standing, explanation, evidence, ownership, namespaces, and tags. Documentation
and the released skill reference it; neither parses prose to reconstruct it.

## Identity

A rule ID is `<namespace>.<name>`. The namespace and name fields shown by the
CLI are derived from that ID, never stored as another identity. An ID keeps one
meaning for its lifetime. Removing a law or changing its meaning is an explicit
version change; there are no aliases or deprecated tombstones in the initial
catalog. A renamed namespace therefore creates a new rule.

Namespaces, tags, owners, and standings are closed vocabularies compiled into
the same binary. Unknown rule IDs and unknown selector values refuse instead of
returning an empty result that could be mistaken for knowledge.

## Standing

- `prose-only` records a law and the evidence a future evaluator would need. It
  emits no finding and changes no exit status.
- `observed` records evidence without rendering a verdict. The initial catalog
  has no observed rules.
- `mechanized` has an evaluator and may emit a finding.

`blind` is a finding grade, not a standing. It means a mechanized evaluator ran
but could not read the evidence needed for its verdict. An unimplemented
prose-only rule is not blind.

`plumb doctor --json` includes whole-catalog counts under `coverage`.
`clean` continues to mean that the current repository produced no finding; it
does not claim that every catalogued law is mechanized.

## Query

```sh
plumb rule list
plumb rule show structure.missing-wrapper
plumb rule namespaces
plumb rule tags
plumb rule owners
```

Every surface accepts `--json`. `list` composes typed selectors:

```sh
plumb rule list \
  --namespace site \
  --standing prose-only \
  --tag site \
  --owner release
```

Repeated `--tag` values are an AND. Values supplied through `--any-tag` are an
OR group. `--without-tag` excludes matches. Repeated `--standing` and `--owner`
values are OR groups. Comma-separated values and repeated flags are equivalent.
There is deliberately no query language and no doctor filtering: the catalog
is for discovery and explanation, while the doctor always renders its complete
verdict.
