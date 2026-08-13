# Plumb v0.18.26

Declared Node sites can now deploy without a root Cargo manifest.

- `plumb site deploy` still stamps every build with the current Git commit.
- `BUILD_VERSION` uses the Cargo workspace or package version when one exists.
- A repository without `Cargo.toml` receives an empty `BUILD_VERSION` instead
  of failing before its site build starts.

This makes the credential-free plan and the real deployment path agree for
Svelte and other Node-only repositories.
