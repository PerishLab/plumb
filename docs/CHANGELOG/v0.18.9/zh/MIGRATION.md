# 迁移到 Plumb v0.18.9

本版本不要求仓库迁移。

协调器可以在能够提供精确 base/head OID 与显式写路径前缀后采用
`plumb precommit` 或 `plumb::boundary` API。内置 retired 词典仍为空，因此本次
Doctor 升级不会引入任何词汇清理要求。
