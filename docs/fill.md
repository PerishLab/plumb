# Fill

Runtime values enter config text through one template grammar. A CLI
that leases a port, names a namespace, or resolves an endpoint and
wants that value inside a config string — a health url, a socket path,
an env value handed to a spawned target — writes a variable into the
text and resolves it through `fill`.

## Division

The crate owns the grammar and the resolver; it owns no variable. What
`{port}` or `{namespace}` mean, and which variables exist at all, is
each CLI's own vocabulary — declared in its manifest documentation,
supplied to `fill` as a table at resolution time. The same division the
cascade draws for config fields holds for template variables: mechanism
at the substrate, vocabulary with the tool. Before `fill`, every tool
grew its own replace chain with its own quiet corners; the grammar is
now one, and the corners are lit.

## Grammar

- `{name}` names a variable; the resolver replaces it with the table's
  value for `name`.
- `{{` and `}}` are the literal braces; nothing else escapes.
- A variable the table does not hold is a refusal, not a pass-through:
  an unresolved template in delivered config is a misread, the same
  refusal posture the cascade takes for a malformed file. Silent
  pass-through is what the old replace chains did, and it hid typos in
  delivered manifests.
- An unclosed `{`, a bare `}`, and an empty `{}` are refusals too.

## Injection

The first consumer is the supervisor seam: a process manager leasing a
port and handing it to a spawned target through the target's own env
vocabulary — the cascade-derived key, never a tool-branded name. The
manifest writes the target's key explicitly and templates the value;
`fill` resolves it at spawn. A hand-written key that drifts from the
target's vocabulary fails silent (a blank env reads as absent), which
is the standing reason a shape check belongs in the traveling CLI.
Until it lands, this page is the wall.
