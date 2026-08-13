# Migration

Existing Claude and Codex seats need no action.

If `~/.grok` already exists, the next `plumb skill install` will attempt
`~/.grok/skills/plumb`. A directory already there that is not in the managed
ledger is refused. Remove that directory, then install. `--force` cannot
claim it.

Installations that do not have a Grok home remain unchanged.
