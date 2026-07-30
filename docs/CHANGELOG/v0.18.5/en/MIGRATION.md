# Migrating to Plumb v0.18.5

A Rust repository adds one step to its guard. Anything that invokes cargo
with `--release` satisfies the rule; a type check is enough and is what the
defect class needs:

```ts
{
  label: "cargo release",
  runs: [["cargo", ["check", "--locked", "--workspace", "--all-targets", "--release"]]],
}
```

Expect the first run after upgrading to fail rather than to report the new
finding — if the repository has never compiled that profile, the step may
find real errors waiting in it. That is the point; fix them there rather
than dropping the step.

A repository with no `Cargo.toml` is unaffected, and no other rule,
declaration, or configuration surface changes.
