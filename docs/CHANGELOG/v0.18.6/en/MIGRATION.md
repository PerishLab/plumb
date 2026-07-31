# Migrating to Plumb v0.18.6

A governed repository needs one `.forgejo/workflows/guard.yml` lane. The lane
must invoke `plumb doctor` and Ectropy directly; Rust repositories must
exercise a release profile, and governed web repositories must build the named
web package.

Once that workflow carries the evidence, generic guard, init, and land
wrappers and repository-owned Git hooks may be removed. Product-specific or
transitional wrappers may remain, but each still needs a known role.

Configuration consumers may opt into closed file vocabularies:

```rust
#[derive(Cascade)]
#[cascade(strict)]
struct Profile {
    #[cascade(section)]
    isolation: Isolation,
}

#[derive(Cascade)]
#[cascade(section, strict)]
struct Isolation {
    root: PathBuf,
}
```

Add strictness only where unknown fields should fail. Unannotated cascades are
unchanged.
