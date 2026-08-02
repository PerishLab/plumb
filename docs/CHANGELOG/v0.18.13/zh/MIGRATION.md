# 迁移到 Plumb v0.18.13

本次发布不需要任何仓库迁移。

若操作脚本精确解析 `plumb release dispatch --watch` 的失败后缀，请把
`failed jobs:` 改为 `failed tasks:`。点名项现在来自 Forgejo action-task 面，并可能包含
失败、取消或阻塞的任务。
