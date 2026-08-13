# Plumb v0.18.27

## One Forgejo dialect

Plumb now depends on the published Runseal library and calls the same
structured Forgejo resource verbs used by `runseal :perish @forgejo`.
Plumb retains Git operations, release topology, retries, polling, and outcome
interpretation; it no longer carries a second authenticated Forgejo HTTP
client.

The former `vendor/forgejo` implementation and `tea.yml` token parser are
removed. Forgejo authority is explicit through `FORGEJO_URL` and either
`FORGEJO_TOKEN_FILE` or `FORGEJO_TOKEN`. Land, stable preparation and
packport, release dispatch and task outcome perception, recovery, and retire
all use Runseal 0.16.3 structured calls.

Court fixtures now exercise the shared HTTP paths, including branch
protection readback, reusable-workflow task disambiguation, guard status, and
merge payloads.
