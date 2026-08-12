# Plumb v0.18.15

## 封闭的 skill limit strategy

含有源码 skill seat 的仓库现在需要在 `plumb.toml` 中选择一个由 Plumb
持有的 limit strategy。每个 strategy 同时封闭允许的根文件枚举和文本总量；
仓库不能直接提供 `files`、`limit` 或公式参数。

首个 `brief` strategy 允许 `SKILL.md`、`PATHS.md` 和 `SCENARIOS.md`，并从
生产源码规模推导三者共享的预算。Doctor 会拒绝缺失或未知的 strategy、开放的
文件集合、不可读证据和超量文本。
