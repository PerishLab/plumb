# Plumb v0.18.13

## Forgejo reusable workflow outcomes

`plumb release dispatch --watch` previously trusted the parent run aggregate
and asked a GitHub-shaped jobs route for failure details. Forgejo can report a
reusable-workflow parent as `blocked` after every underlying task succeeds, and
the jobs route is not part of the current Forgejo API.

The watcher now reads Forgejo's paginated action-task surface only when a
terminal result needs disambiguation or diagnostics. It correlates tasks with
the repository-unique run number, accepts an all-success reusable workflow,
keeps optional skipped tasks neutral, and reports named failed, cancelled, or
blocked tasks. Unknown or malformed status evidence refuses instead of being
treated as success.

The existing raw `Client::run` API remains unchanged. `Client::outcome` and the
typed `Outcome` projection carry the new judgment.
