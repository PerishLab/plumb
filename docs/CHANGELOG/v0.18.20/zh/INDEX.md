# Plumb v0.18.20

本版本将闭合的 document strategy 模型提升为受管仓库唯一的文档面。

- 每个包含 `plumb.toml` 的仓库必须声明一条已确认的 `agent` document；同时
  可以从闭合的 `architecture`、`design` 与具名 `brief` strategy 中选择。
- v0.18.19 有界兼容窗口结束后，独立 skill strategy、通用 affirmation
  schema、对应 Doctor rules 与 `plumb lock` 命令全部删除。
- Doctor 现在从共享 Git index snapshot 推导顶层 layout。空目录或未跟踪目录
  不会再导致本地 worktree 与干净远端 checkout 得出不同结论。
- Doctor JSON 与 rule catalog 不再暴露退役的 skill-shape / generic-lock
  字段和规则；brief 仍由 document report 与 document rules 完整管理。

本版本不改变已安装 skill 管理、release locking、Cargo / Deno 依赖 lock
检查或 manager 进程锁；它们是彼此独立的现行产品面。
