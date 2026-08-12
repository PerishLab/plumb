# 迁移到 Plumb v0.18.19

运行本目录中对应平台的 migration 脚本。脚本会删除旧 `[skill]` 与
`[[lock]]`，新增一条未确认的根 `agent` binding，并为每个精确三文件 skill
seat 新增一条未确认的 `brief` binding。根 source 只是迁移期的保守证据，
不代表已经完成人工审阅。

随后处理 Doctor 报出的每个 Markdown：当前操作约束归入 `AGENTS.md`；当前
跨边界拓扑可以进入 `ARCHITECTURE.md`；产品原则可以进入 `DESIGN.md`；agent
操作必须进入三分 skill；发布历史保留在 `docs/CHANGELOG`；尚未收敛的工作
不进入仓库。

逐一阅读 source 与 target，运行 `plumb document .`，再手动写入提案 seal。
禁止跳过审阅直接复制，也不要新增排除项、任意目标路径或本地数值上限。
