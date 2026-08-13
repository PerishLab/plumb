# Migrating to Plumb v0.18.27

Stop relying on `~/.tea/tea.yml` for Plumb credentials. Provide the Forgejo
authority and one token seat:

```bash
export FORGEJO_URL=https://git.perish.top
export FORGEJO_TOKEN_FILE=/path/to/forgejo-token
```

`FORGEJO_TOKEN` remains available for CI and bounded tests. The token file is
preferred for operator use and contains only the token value.

No command syntax changes. `plumb land`, `plumb release dispatch`,
`plumb stable`, recovery, and retire retain their orchestration and refusal
semantics while using Runseal's in-process Forgejo dialect.
