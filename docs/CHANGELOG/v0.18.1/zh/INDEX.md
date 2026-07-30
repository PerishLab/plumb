# Plumb v0.18.1

## 持久发布源码

已结算的 `release/vX.Y.Z` 分支现在是永久冻结的源码与审计边界。Packport 只改变
`main` 的祖先关系，不再允许移除分支或其保护。

## 保护规范墙

Plumb skill 现在为每个受管仓库持有一套规范且逐字节精确的分支保护形状。`main` 与
`release/**` 基线规则没有逐仓选项；精确 release 规则只从 PREPARING 或 FROZEN
状态推导。

这条法则刻意保持 prose-only。`plumb doctor` 不会声称自己读到了 Forgejo 控制面
真值；本地 operator 负责应用每条规则，并逐字节校验 API 回读投影。
