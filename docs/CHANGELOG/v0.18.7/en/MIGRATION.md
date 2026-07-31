# Migrating to Plumb v0.18.7

Forgejo repositories already using `.forgejo/workflows/guard.yml` need no
workflow move.

A GitHub repository uses `.github/workflows/quality.yml` as its canonical
guard seat. Give that workflow the shared concurrency block:

```yaml
concurrency:
  group: guard-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
```

Remove an obsolete second canonical seat if both the Forgejo guard and GitHub
quality workflows are present. Product-specific workflows beside the selected
guard seat are unchanged.

No runtime audit migration is required. Operators who already have a Plumb
audit report may inspect it explicitly:

```sh
locus inspect . < /path/to/plumb-audit.jsonl
```

Inspection does not enable collection or mutate the report.
