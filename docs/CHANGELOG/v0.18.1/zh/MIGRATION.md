# 迁移到 Plumb v0.18.1

把 release operator 升级到 Sealkit v0.3.2 或更高版本，并从 packport 调用中移除
`--keep-branch`。保留分支不再是可选行为。

阅读 `skills/plumb/references/protection.md`，在每个根目录带 `plumb.toml` 的仓库中
对齐规范的 `main` 与 `release/**` 规则，然后回读每条规则并逐字节比较其规范投影。

对于 stable pointer 已存在、但缺少 `release/vX.Y.Z` 分支的版本，在 pointer 记录
的精确 commit 上重建分支并应用 FROZEN 规则。不要创建新的 release tag。
