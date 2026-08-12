# 迁移

已经在 v0.18.19 下完成迁移的仓库无需修改文件。

任何跳过兼容版本的仓库，在运行 v0.18.20 Doctor 前必须先遵循 v0.18.19
migration 文档与对应平台脚本，人工阅读所有提议的 source / target，记录
`plumb document` 给出的 seals，并先取得干净的 v0.18.19 Doctor 结果。
v0.18.20 不接受或改写退役声明，也不再提供通用 lock 的替代命令。

Doctor JSON 消费方应停止读取 `shape.strategy` 与 `shape.skills`，改读
`shape.documents`；具名 `brief` 条目包含受管 skill target、source 数量、
text、budget 与 leaf 数量。

Rule catalog 消费方应删除退役的 `lock.*` 与 skill shape-rule 标识。document
schema、admission、evidence 与 magnitude 现在完整承接 source skill 契约。
