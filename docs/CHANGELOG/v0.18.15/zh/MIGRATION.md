# 迁移到 Plumb v0.18.15

每个含有 `skills/` 源码 seat 的仓库都必须在根部 `plumb.toml` 增加：

```toml
[skill]
strategy = "brief"
```

`brief` strategy 要求每个 `skills/*` seat 恰好包含 `SKILL.md`、`PATHS.md`
和 `SCENARIOS.md`。三份 Markdown 共享
`clamp(2 * ceil(sqrt(源码行数)), 120, 400)` 行的总预算。升级前应删除其他根部
条目，并将文本总量收敛到推导预算内。

不要声明文件名、数值上限或公式参数；它们由选定的 strategy 持有，直接配置这些
字段会被拒绝。
