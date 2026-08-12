# Plumb Locus audit

Plumb owns the meaning of its CLI observations and Locus owns their context,
collection, generation, and reporting mechanisms. Runtime policy crosses that
boundary through one typed `PLUMB_LOCUS_*` config section:

- `PLUMB_LOCUS_ENABLED` is the master gate and accepts only `true` or `false`.
- `PLUMB_LOCUS_REPORT_FILE` selects Locus's built-in file reporter.
- `PLUMB_LOCUS_TRACE_FILE` selects the shared-file trace generator.
- `PLUMB_LOCUS_TRACE_ID` supplies a literal trace key.
- exact `CODEX_THREAD_ID` collection is bound to `locus.trace` as
  `codex.thread` when observation is enabled.
- `PLUMB_LOCUS_TARGET_COLLECTORS` supplies the explicit ordered collector chain
  bound to `plumb.target`.

Missing and blank values inherit the total defaults. The gate defaults to
`false`. While it is false, Plumb returns before Locus bootstrap and does not
run a collector or generator, open a reporter, or create an audit file.
Configured collector and reporter values may therefore remain inherited
without creating an ambient observation path.

When the gate is true, `PLUMB_LOCUS_REPORT_FILE` is required. Missing or invalid
enabled configuration refuses the audit record without replacing the CLI
command's own result.

Trace precedence is explicit `PLUMB_LOCUS_TRACE_ID`, exact `codex.thread`
collection, inherited Context, and configured shared or default random
generation. Collection reads only `CODEX_THREAD_ID`, limits it to 512 bytes,
and freezes provenance into the start Atom. Absence falls through; invalid
present input refuses the audit record. Plumb attaches no executor, activity,
lifecycle, or authority meaning to the opaque trace key.

`PLUMB_LOCUS_TARGET_COLLECTORS` is a comma-separated chain. Each entry selects
one exact bounded fact:

- `environment:NAME` reads one named environment value.
- `argv:INDEX` reads one argument after the executable, starting at zero.
- `process:directory`, `process:executable`, or `process:id` reads one named
  process fact.

Plumb binds the first present value to `plumb.target` and limits it to 512
bytes. Only absence advances the chain. Invalid or unreadable facts refuse the
audit record; there is no ambient environment, argument, or process scan.

## Operator control

An operator may hold one global policy in the shell environment inherited by
every Plumb process. On a host whose single shared shell entry is
`~/.profile`, that file or its owned fragments is the control plane. It may
authoritatively export false and clear the remaining variables to keep the
host muted.

This skill does not write the profile, choose an enabled state, select a report
path, or nominate a collector. Those are operator facts. Per-invocation
overrides use the same variables and do not create a second config surface.

## Consumption

Plumb's root `locus.toml` is a separate read-side declaration. It cannot enable
the runtime audit surface. Its coarse size and repetition analyzers inspect
both `locus.trace`, the caller-managed cross-process context, and `locus.span`,
one Plumb command invocation:

```sh
locus inspect /path/to/plumb < /path/to/plumb-audit.jsonl
```

Every complete inspection emits a coverage summary. A trace finding with clean
spans says only that the aggregate crossed the declared sieve while no single
invocation did. Inspection does not infer a cause, judge the command, mutate
the report, or authorize collection.
