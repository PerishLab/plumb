# Audit

Plumb emits no audit record by default. `PLUMB_LOCUS_ENABLED` is the typed
master gate and accepts only `true` or `false`. When it is absent or `false`,
Plumb returns before Locus bootstrap: no collector or generator runs, no
reporter opens, and no audit file is created. Its enabled Locus surface is an
observation path around one CLI invocation; it does not alter the command,
judge its result, or infer a repository identity.

The settings below are Plumb's named Locus bootstrap control surface.
`PLUMB_LOCUS_REPORT_FILE` selects the built-in JSONL reporter.
`PLUMB_LOCUS_TRACE_FILE` selects the shared-file trace-key generator, and
`PLUMB_LOCUS_TRACE_ID` supplies a literal trace key that wins over exact
collection, inheritance, and generation.

## Trace identity

When observation is enabled, Plumb asks Locus's named `codex.thread` collector
for exactly `CODEX_THREAD_ID` and binds a present value to `locus.trace`. It
does not scan the environment. The value is bounded at 512 bytes, and the
start Atom freezes collector provenance without duplicating the value.

An explicit `PLUMB_LOCUS_TRACE_ID` wins without running the collector. When
the environment fact is absent, identity falls through to the configured
shared-file generator or the default random generator. This keeps one Codex
thread continuous across independently observed products without teaching
Plumb executor, activity, lifecycle, or authority semantics.

## Target

`PLUMB_LOCUS_TARGET_COLLECTORS` asks Plumb to bind the product-owned
`plumb.target` role. Its value is an ordered, comma-separated collector chain:

- `environment:NAME` reads exactly one named environment value.
- `argv:INDEX` reads exactly one argument after the executable, starting at
  zero.
- `process:directory`, `process:executable`, and `process:id` read exactly one
  named process fact.

Plumb bounds every collected value at 512 bytes. It installs no collector and
reads no ambient target unless this setting is present. A chain advances only
when the selected fact is absent. An invalid or unreadable present fact refuses
the audit record, reaches the Locus diagnostic hook, and does not fall through.
Invalid collector configuration is refused during bootstrap. Audit refusal
does not replace the CLI command's own result.

The caller owns the sensitivity and stability of the fact it selects. For
example, this records an opaque repository label without exposing a path:

```sh
PLUMB_LOCUS_ENABLED=true \
PLUMB_AUDIT_TARGET=repository-a \
PLUMB_LOCUS_TARGET_COLLECTORS=environment:PLUMB_AUDIT_TARGET \
PLUMB_LOCUS_REPORT_FILE=/tmp/plumb-audit.jsonl \
plumb doctor .
```

The start atom freezes both the target value and collector provenance. The
finish atom inherits the same context. Provenance names the role, binding,
collector, and selector; it does not duplicate the collected value.

## Consumption

The reporter output is a JSONL-like stream of Locus atoms. Pairing start and
finish by context, grouping by `plumb.target`, measuring duration, and deciding
whether a result is an efficiency problem are downstream derivations. This
surface performs none of those interpretations.

The root `locus.toml` is Plumb's read-side inspection declaration. It is not
part of the runtime config cascade and cannot enable collection. It applies the
same coarse size and repetition mappings at two identity resolutions:

- `locus.trace` exposes excess across the caller-managed cross-process context.
- `locus.span` exposes excess inside one Plumb command invocation.

Inspect an explicitly obtained report without changing it:

```sh
locus inspect . < /path/to/plumb-audit.jsonl
```

Every complete inspection ends with a coverage summary. A trace finding with
clean spans means the aggregate crossed the sieve while no individual
invocation did; it does not attribute a cause or require splitting the trace.
