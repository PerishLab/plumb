# 迁移到 Plumb v0.16.0

binary 产品必须把发布声明迁移到根 `plumb.toml` 的 `[release]`。删除产品侧
发布脚本、`.forgejo/release.toml`、`release-verify`，以及本地实现的
manager、打包、验证和 registry publish 逻辑。

只保留 `release-exact.yml` 与 `release-stable.yml`，作为
`PerishLab/actions/.forgejo/workflows/release-binary.yml@main` 的薄
caller。仓库凭证统一使用 `RELEASE_PUBLISH_S3_*`、
`RELEASE_ACTIVATE_S3_*` 和可选的 `RELEASE_REGISTRY_TOKEN`。

安装统一调用 `PerishLab/actions/setup-binary@main` 并设置
`PERISH_SETUP_PRODUCT`。只有 stable 可以使用默认席位；non-stable 必须
同时给出精确版本，并自动进入隔离路径。
