# Migrating to Plumb v0.18.2

No action is required when CLI auditing is disabled or when the existing
trace and report settings are sufficient.

To attach an opaque target label to each invocation, set the label yourself
and select only that named environment value:

```sh
PLUMB_AUDIT_TARGET=repository-a \
PLUMB_LOCUS_TARGET_COLLECTORS=environment:PLUMB_AUDIT_TARGET \
PLUMB_LOCUS_REPORT_FILE=/tmp/plumb-audit.jsonl \
plumb doctor .
```

Review the sensitivity and stability of every selected fact. Collector chains
fall through only when a fact is absent; an invalid present fact or collector
error refuses the audit record rather than silently selecting another source.
See `docs/audit.md` for the complete bounded grammar.
