# Plumb v0.18.26

声明过的 Node 站点现在无需根 `Cargo.toml` 即可部署。

- `plumb site deploy` 仍会用当前 Git commit 标记每次构建。
- 存在 Cargo workspace 或 package version 时，`BUILD_VERSION` 继续使用它。
- 不含 `Cargo.toml` 的仓库会收到空的 `BUILD_VERSION`，不再在站点构建开始前失败。

因此对 Svelte 等纯 Node 仓库，credential-free plan 与真实部署路径现在保持一致。
